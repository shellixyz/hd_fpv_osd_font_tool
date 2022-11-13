
use std::{cmp::Ordering, error::Error};
use std::fmt::Display;

use derive_more::Display;
use hd_fpv_osd_font_tool::osd::tile::container::save_symbols_to_dir::SaveSymbolsToDirError;
use hd_fpv_osd_font_tool::osd::tile::container::save_tiles_to_dir::SaveTilesToDirError;
use hd_fpv_osd_font_tool::osd::tile::container::save_to_bin_file::SaveTilesToBinFileError;
use hd_fpv_osd_font_tool::osd::tile::container::symbol::spec::LoadSpecsFileError;
use hd_fpv_osd_font_tool::osd::bin_file::{LoadError as BinFileLoadError, LoadSetError as BinFileLoadSetError};
use hd_fpv_osd_font_tool::osd::tile::container::tile_set::LoadTileSetTilesFromDirError;
use hd_fpv_osd_font_tool::osd::tile::container::symbol::set::LoadFromDirError as SymbolSetLoadFromDirError;

use crate::ConvertOptions;

use super::convert::InvalidConvertArgError;
use hd_fpv_osd_font_tool::prelude::*;

enum ConvertSetArg<'a> {
    BinFileSet {
        sd_path: &'a str,
        sd_2_path: &'a str,
        hd_path: &'a str,
        hd_2_path: &'a str,
    },
    BinFileSetNorm {
        dir: &'a str,
        ident: Option<&'a str>
    },
    TileSetGrids {
        sd_path: &'a str,
        hd_path: &'a str,
    },
    TileSetGridsNorm {
        dir: &'a str,
        ident: Option<&'a str>
    },
    TileSetDir(&'a str),
    SymbolSetDir(&'a str),
}

impl<'a> ConvertSetArg<'a> {

    fn prefix(&self) -> &'static str {
        use ConvertSetArg::*;
        match self {
            BinFileSet {..} => "binset",
            BinFileSetNorm {..} => "binsetnorm",
            TileSetGrids {..} => "tilesetgrids",
            TileSetGridsNorm {..} => "tilesetgridsnorm",
            TileSetDir(_) => "tilesetdir",
            SymbolSetDir(_) => "symsetdir",
        }
    }
}

#[derive(Debug, Display)]
pub enum InvalidConvertSetArgError {
    InvalidConvertArgError(InvalidConvertArgError),
    BinSetInvalidArguments(&'static str),
    TileSetGridsInvalidArguments(&'static str),
}

fn argument_norm_args(arg: &str) -> Result<(&str, Option<&str>), InvalidConvertSetArgError> {
    let args: Vec<&str> = arg.split(':').collect();
    if args.len() > 2 {
        return Err(InvalidConvertSetArgError::BinSetInvalidArguments("too many arguments"))
    } else if args.is_empty() {
        return Err(InvalidConvertSetArgError::BinSetInvalidArguments("too few arguments"))
    }
    let dir = args[0];
    let ident = args.get(1).cloned();
    Ok((dir, ident))
}

fn identify_convert_set_arg(input: &str) -> Result<ConvertSetArg, InvalidConvertSetArgError> {
    if let Some(file_paths) = input.strip_prefix("binset:") {
        let files: Vec<&str> = file_paths.split(':').collect();
        match files.len().cmp(&4) {
            Ordering::Less => return Err(InvalidConvertSetArgError::BinSetInvalidArguments("too few arguments")),
            Ordering::Greater => return Err(InvalidConvertSetArgError::BinSetInvalidArguments("too many arguments")),
            Ordering::Equal => {},
        }
        Ok(ConvertSetArg::BinFileSet { sd_path: files[0], sd_2_path: files[1], hd_path: files[2], hd_2_path: files[3] })

    } else if let Some(path) = input.strip_prefix("binsetnorm:") {
        let (dir, ident) = argument_norm_args(path)?;
        Ok(ConvertSetArg::BinFileSetNorm { dir, ident })

    } else if let Some(file_paths) = input.strip_prefix("tilesetgrids:") {
        let files: Vec<&str> = file_paths.split(':').collect();
        match files.len().cmp(&2) {
            Ordering::Less => return Err(InvalidConvertSetArgError::TileSetGridsInvalidArguments("too few arguments")),
            Ordering::Greater => return Err(InvalidConvertSetArgError::TileSetGridsInvalidArguments("too many arguments")),
            Ordering::Equal => {},
        }
        Ok(ConvertSetArg::TileSetGrids { sd_path: files[0], hd_path: files[1] })

    } else if let Some(path) = input.strip_prefix("tilesetgridsnorm:") {
        let (dir, ident) = argument_norm_args(path)?;
        Ok(ConvertSetArg::TileSetGridsNorm { dir, ident  })

    } else if let Some(path) = input.strip_prefix("tilesetdir:") {
        Ok(ConvertSetArg::TileSetDir(path))

    } else if let Some(path) = input.strip_prefix("symsetdir:") {
        Ok(ConvertSetArg::SymbolSetDir(path))

    } else if let Some((prefix, _)) = input.split_once(':') {
        Err(InvalidConvertSetArgError::InvalidConvertArgError(InvalidConvertArgError::InvalidPrefix(prefix.to_owned())))
    } else {
        Err(InvalidConvertSetArgError::InvalidConvertArgError(InvalidConvertArgError::NoPrefix))
    }
}

#[derive(Debug)]
pub enum ConvertSetError {
    FromArg(InvalidConvertSetArgError),
    ToArg(InvalidConvertSetArgError),
    InvalidConversion {
        from_prefix: String,
        to_prefix: String
    },
    LoadError(String),
    SaveError(String),
}

impl Error for ConvertSetError {}

impl Display for ConvertSetError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use ConvertSetError::*;
        match self {
            FromArg(error) => write!(f, "invalid `from` argument: {}", error),
            ToArg(error) => write!(f, "invalid `to` argument: {}", error),
            InvalidConversion { from_prefix, to_prefix } => write!(f, "invalid conversion from {} to {}", from_prefix, to_prefix),
            LoadError(error_string) => error_string.fmt(f),
            SaveError(error_string) => error_string.fmt(f),
        }
    }
}

impl From<GridLoadError> for ConvertSetError {
    fn from(error: GridLoadError) -> Self {
        ConvertSetError::LoadError(error.to_string())
    }
}

impl From<GridSaveImageError> for ConvertSetError {
    fn from(error: GridSaveImageError) -> Self {
        ConvertSetError::SaveError(error.to_string())
    }
}

impl From<SaveTilesToBinFileError> for ConvertSetError {
    fn from(error: SaveTilesToBinFileError) -> Self {
        ConvertSetError::SaveError(error.to_string())
    }
}

impl From<SaveTilesToDirError> for ConvertSetError {
    fn from(error: SaveTilesToDirError) -> Self {
        ConvertSetError::SaveError(error.to_string())
    }
}

impl From<SaveSymbolsToDirError> for ConvertSetError {
    fn from(error: SaveSymbolsToDirError) -> Self {
        ConvertSetError::SaveError(error.to_string())
    }
}

impl From<LoadSpecsFileError> for ConvertSetError {
    fn from(error: LoadSpecsFileError) -> Self {
        ConvertSetError::LoadError(error.to_string())
    }
}

impl From<BinFileLoadError> for ConvertSetError {
    fn from(error: BinFileLoadError) -> Self {
        ConvertSetError::LoadError(error.to_string())
    }
}

impl From<BinFileLoadSetError> for ConvertSetError {
    fn from(error: BinFileLoadSetError) -> Self {
        ConvertSetError::LoadError(error.to_string())
    }
}

impl From<SymbolSetLoadFromDirError> for ConvertSetError {
    fn from(error: SymbolSetLoadFromDirError) -> Self {
        ConvertSetError::LoadError(error.to_string())
    }
}

impl From<LoadTileSetTilesFromDirError> for ConvertSetError {
    fn from(error: LoadTileSetTilesFromDirError) -> Self {
        ConvertSetError::LoadError(error.to_string())
    }
}


fn convert_tile_set(tile_set: TileSet, to_arg: &ConvertSetArg, options: &ConvertOptions) -> Result<(), ConvertSetError> {
    use ConvertSetArg::*;
    match to_arg {
        BinFileSet { sd_path, sd_2_path, hd_path, hd_2_path } => tile_set.save_to_bin_files(sd_path, sd_2_path, hd_path, hd_2_path)?,
        BinFileSetNorm { dir, ident } => tile_set.save_to_bin_files_norm(dir, ident)?,
        TileSetGrids { sd_path, hd_path } => tile_set.save_to_grids(sd_path, hd_path)?,
        TileSetGridsNorm { dir, ident  } => tile_set.save_to_grids_norm(dir, ident)?,
        TileSetDir(dir) => tile_set.save_tiles_to_dir(dir)?,
        SymbolSetDir(dir) => {
            let sym_specs = SymbolSpecs::load_file(options.symbol_specs_file)?;
            tile_set.into_symbol_set(&sym_specs).unwrap().save_to_dir(dir)?;
        },
    }
    Ok(())
}

pub fn convert_set_command(from: &str, to: &str, options: ConvertOptions) -> Result<(), ConvertSetError> {
    let from_arg = identify_convert_set_arg(from).map_err(ConvertSetError::FromArg)?;
    let to_arg = identify_convert_set_arg(to).map_err(ConvertSetError::ToArg)?;
    log::info!("converting {} -> {}", from, to);

    use ConvertSetArg::*;
    match (&from_arg, &to_arg) {
        (BinFileSet{..}, BinFileSet{..}) | (BinFileSetNorm {..}, BinFileSetNorm {..}) | (TileSetGrids{..}, TileSetGrids{..}) |
        (TileSetGridsNorm {..}, TileSetGridsNorm {..}) | (TileSetDir(_), TileSetDir(_)) | (SymbolSetDir(_), SymbolSetDir(_)) =>
            return Err(ConvertSetError::InvalidConversion { from_prefix: from_arg.prefix().to_owned(), to_prefix: to_arg.prefix().to_owned()}),

        (BinFileSet { sd_path, sd_2_path, hd_path, hd_2_path }, to_arg) => {
            let tile_set = bin_file::load_set(sd_path, sd_2_path, hd_path, hd_2_path)?;
            convert_tile_set(tile_set, to_arg, &options)
        },

        (BinFileSetNorm { dir, ident }, to_arg) => {
            let tile_set = bin_file::load_set_norm(dir, ident)?;
            convert_tile_set(tile_set, to_arg, &options)
        },

        (TileSetGrids { sd_path, hd_path }, to_arg) => {
            let tile_grid_set = TileGridSet::load_from_images(sd_path, hd_path)?;
            convert_tile_set(tile_grid_set.into_tile_set(), to_arg, &options)
        },

        (TileSetGridsNorm { dir, ident }, to_arg) => {
            let tile_grid_set = TileGridSet::load_from_images_norm(dir, ident)?;
            convert_tile_set(tile_grid_set.into_tile_set(), to_arg, &options)
        },

        (TileSetDir(dir), to_arg) => {
            let tile_set = TileSet::load_from_dir(dir, 512)?;
            convert_tile_set(tile_set, to_arg, &options)
        },

        (SymbolSetDir(dir), to_arg) => {
            let symbol_set = SymbolSet::load_from_dir(dir, 512)?;
            convert_tile_set(symbol_set.into(), to_arg, &options)
        },

    }
}