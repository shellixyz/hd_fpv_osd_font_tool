
use std::{error::Error, fmt::Display, path::Path};

use hd_fpv_osd_font_tool::prelude::*;

use crate::ConvertOptions;


#[derive(Debug)]
pub enum InvalidConvertArgError {
    InvalidPrefix(String),
    InvalidImageFileExtension {
        path: String,
        extension: Option<String>
    },
    InvalidPath(String),
    NoPrefix
}

impl Error for InvalidConvertArgError {}

impl Display for InvalidConvertArgError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use InvalidConvertArgError::*;
        match self {
            InvalidPrefix(prefix) => write!(f, "invalid prefix: {}", prefix),
            NoPrefix => f.write_str("no prefix"),
            InvalidImageFileExtension { path, extension: Some(extension) } => write!(f, "invalid image file extension `{}`: {}", extension, path),
            InvalidImageFileExtension { path, extension: None } => write!(f, "image path has no file extension: {}", path),
            InvalidPath(path) => write!(f, "invalid path: {}", path),
        }
    }
}

enum ConvertArg<'a> {
    BinFile(&'a str),
    TileGrid(&'a str),
    TileDir(&'a str),
    SymbolDir(&'a str),
}

impl<'a> ConvertArg<'a> {

    fn prefix(&self) -> &'static str {
        use ConvertArg::*;
        match self {
            BinFile(_) => "bin",
            TileGrid(_) => "tilegrid",
            TileDir(_) => "tiledir",
            SymbolDir(_) => "symdir",
        }
    }
}

fn check_arg_image_file_extension(path: &str) -> Result<(), InvalidConvertArgError> {
    match Path::extension(Path::new(path)) {
        Some(os_str) => match os_str.to_str() {
            Some("png") => Ok(()),
            Some(extension) => Err(InvalidConvertArgError::InvalidImageFileExtension { path: path.to_owned(), extension: Some(extension.to_owned()) }),
            None => Err(InvalidConvertArgError::InvalidPath(path.to_owned()))
        },
        None => Err(InvalidConvertArgError::InvalidImageFileExtension { path: path.to_owned(), extension: None })
    }
}

fn identify_convert_arg(input: &str) -> Result<ConvertArg, InvalidConvertArgError> {
    if let Some(path) = input.strip_prefix("bin:") {
        Ok(ConvertArg::BinFile(path))
    } else if let Some(path) = input.strip_prefix("tilegrid:") {
        Ok(ConvertArg::TileGrid(path))
    } else if let Some(path) = input.strip_prefix("tiledir:") {
        Ok(ConvertArg::TileDir(path))
    } else if let Some(path) = input.strip_prefix("symdir:") {
        Ok(ConvertArg::SymbolDir(path))
    } else if let Some((prefix, _)) = input.split_once(':') {
        Err(InvalidConvertArgError::InvalidPrefix(prefix.to_owned()))
    } else {
        Err(InvalidConvertArgError::NoPrefix)
    }
}

#[derive(Debug)]
pub enum ConvertError {
    FromArg(InvalidConvertArgError),
    ToArg(InvalidConvertArgError),
    InvalidConversion {
        from_prefix: String,
        to_prefix: String
    }
}

impl Error for ConvertError {}

impl Display for ConvertError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use ConvertError::*;
        match self {
            FromArg(error) => write!(f, "invalid `from` argument: {}", error),
            ToArg(error) => write!(f, "invalid `to` argument: {}", error),
            InvalidConversion { from_prefix, to_prefix } => write!(f, "invalid conversion from {} to {}", from_prefix, to_prefix),
        }
    }
}

pub fn convert_command(from: &str, to: &str, options: ConvertOptions) -> anyhow::Result<()> {
    let from_arg = identify_convert_arg(from).map_err(ConvertError::FromArg)?;
    let to_arg = identify_convert_arg(to).map_err(ConvertError::ToArg)?;
    log::info!("converting {} -> {}", from, to);

    use ConvertArg::*;
    match (&from_arg, &to_arg) {
        (BinFile(_), BinFile(_)) | (TileGrid(_), TileGrid(_)) | (TileDir(_), TileDir(_)) | (SymbolDir(_), SymbolDir(_)) =>
            Err(ConvertError::InvalidConversion { from_prefix: from_arg.prefix().to_owned(), to_prefix: to_arg.prefix().to_owned()})?,

        (BinFile(from_path), to_arg) => {
            let bin_file_tiles = bin_file::load(from_path).unwrap();
            match to_arg {
                TileGrid(to_path) => {
                    check_arg_image_file_extension(to_path).map_err(ConvertError::ToArg)?;
                    bin_file_tiles.save_to_grid_image(to_path).unwrap()
                },
                TileDir(to_path) => bin_file_tiles.save_tiles_to_dir(to_path).unwrap(),
                SymbolDir(to_path) => {
                    let sym_specs = SymbolSpecs::load_file(options.symbol_specs_file).unwrap();
                    bin_file_tiles.to_symbols(&sym_specs).unwrap().save_to_dir(to_path).unwrap();
                },
                _ => unreachable!()
            }
        },

        (TileGrid(from_path), to_arg) => {
            check_arg_image_file_extension(from_path).map_err(ConvertError::FromArg)?;
            let tile_grid = crate::TileGrid::load_from_image(from_path).unwrap();
            match to_arg {
                BinFile(to_path) => tile_grid.save_tiles_to_bin_file(to_path).unwrap(),
                TileDir(to_path) => tile_grid.save_tiles_to_dir(to_path).unwrap(),
                SymbolDir(to_path) => {
                    let sym_specs = SymbolSpecs::load_file(options.symbol_specs_file).unwrap();
                    tile_grid.to_symbols(&sym_specs).unwrap().save_to_dir(to_path).unwrap();
                },
                _ => unreachable!()
            }
        },

        (TileDir(from_path), to_arg) => {
            let tiles = load_tiles_from_dir(from_path, 512).unwrap();
            match to_arg {
                BinFile(to_path) => tiles.save_to_bin_file(to_path).unwrap(),
                TileGrid(to_path) => {
                    check_arg_image_file_extension(to_path).map_err(ConvertError::ToArg)?;
                    tiles.save_to_grid_image(to_path).unwrap()
                },
                SymbolDir(to_path) => {
                    let sym_specs = SymbolSpecs::load_file(options.symbol_specs_file).unwrap();
                    tiles.to_symbols(&sym_specs).unwrap().save_to_dir(to_path).unwrap();
                },
                _ => unreachable!()
            }
        },

        (SymbolDir(from_path), to_arg) => {
            let tiles = load_symbols_from_dir(from_path, 512).unwrap().into_tiles_vec();
            match to_arg {
                BinFile(to_path) => tiles.save_to_bin_file(to_path).unwrap(),
                TileGrid(to_path) => tiles.into_tile_grid().generate_image().unwrap().save(to_path).unwrap(),
                TileDir(to_path) => tiles.save_tiles_to_dir(to_path).unwrap(),
                _ => unreachable!()
            }
        }

    }

    Ok(())
}