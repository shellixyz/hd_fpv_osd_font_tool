
use std::{error::Error, fmt::Display, path::Path};

use hd_fpv_osd_font_tool::prelude::*;
use thiserror::Error;

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
    AvatarFile(&'a str),
    TileGrid(&'a str),
    TileDir(&'a str),
    SymbolDir(&'a str),
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
    if let Some(path) = input.strip_prefix("djibin:") {
        Ok(ConvertArg::BinFile(path))
    } else if let Some(path) = input.strip_prefix("tilegrid:") {
        Ok(ConvertArg::TileGrid(path))
    } else if let Some(path) = input.strip_prefix("tiledir:") {
        Ok(ConvertArg::TileDir(path))
    } else if let Some(path) = input.strip_prefix("symdir:") {
        Ok(ConvertArg::SymbolDir(path))
    } else if let Some(path) = input.strip_prefix("avatar:") {
        Ok(ConvertArg::AvatarFile(path))
    } else if let Some((prefix, _)) = input.split_once(':') {
        Err(InvalidConvertArgError::InvalidPrefix(prefix.to_owned()))
    } else {
        Err(InvalidConvertArgError::NoPrefix)
    }
}

#[derive(Debug, Error)]
pub enum ConvertError {
    #[error("invalid `from` argument: {0}")]
    FromArg(InvalidConvertArgError),
    #[error("invalid `to` argument: {0}")]
    ToArg(InvalidConvertArgError),
}

fn convert_tiles(tiles: Vec<Tile>, to_arg: &ConvertArg, options: &ConvertOptions) -> anyhow::Result<()> {
    use ConvertArg::*;
    match to_arg {
        TileGrid(to_path) => {
            check_arg_image_file_extension(to_path).map_err(ConvertError::ToArg)?;
            tiles.save_to_grid_image(to_path)?
        },
        TileDir(to_path) => tiles.save_tiles_to_dir(to_path)?,
        SymbolDir(to_path) => {
            let sym_specs = SymbolSpecs::load_file(options.symbol_specs_file)?;
            tiles.to_symbols(&sym_specs)?.save_to_dir(to_path)?;
        },
        BinFile(to_path) => tiles.save_to_bin_file(to_path)?,
        AvatarFile(to_path) => tiles.save_to_avatar_file(to_path)?,
    }
    Ok(())
}

fn convert_tile_grid(tile_grid: TileGrid, to_arg: &ConvertArg, options: &ConvertOptions) -> anyhow::Result<()> {
    use ConvertArg::*;
    match to_arg {
        BinFile(to_path) => tile_grid.save_tiles_to_bin_file(to_path)?,
        TileDir(to_path) => tile_grid.save_tiles_to_dir(to_path)?,
        SymbolDir(to_path) => {
            let sym_specs = SymbolSpecs::load_file(options.symbol_specs_file)?;
            tile_grid.to_symbols(&sym_specs)?.save_to_dir(to_path)?;
        },
        TileGrid(to_path) => tile_grid.save_image(to_path)?,
        AvatarFile(to_path) => tile_grid.save_tiles_to_avatar_file(to_path)?,
    }
    Ok(())
}

pub fn convert_command(from: &str, to: &str, options: ConvertOptions) -> anyhow::Result<()> {
    let from_arg = identify_convert_arg(from).map_err(ConvertError::FromArg)?;
    let to_arg = identify_convert_arg(to).map_err(ConvertError::ToArg)?;
    log::info!("converting {} -> {}", from, to);

    use ConvertArg::*;
    match (&from_arg, &to_arg) {

        (BinFile(from_path), to_arg) => {
            let tiles = bin_file::load(from_path)?;
            convert_tiles(tiles, to_arg, &options)?;
        },

        (TileGrid(from_path), to_arg) => {
            check_arg_image_file_extension(from_path).map_err(ConvertError::FromArg)?;
            let tile_grid = crate::TileGrid::load_from_image(from_path)?;
            convert_tile_grid(tile_grid, to_arg, &options)?;
        },

        (TileDir(from_path), to_arg) => {
            let tiles = load_tiles_from_dir(from_path, 512)?;
            convert_tiles(tiles, to_arg, &options)?;
        },

        (SymbolDir(from_path), to_arg) => {
            let tiles = load_symbols_from_dir(from_path, 512)?.into_tiles_vec();
            convert_tiles(tiles, to_arg, &options)?;
        },

        (AvatarFile(from_path), to_arg) => {
            let tiles = load_avatar_file(from_path)?;
            convert_tiles(tiles, to_arg, &options)?;
        }

    }

    Ok(())
}

#[cfg(test)]
mod tests {

    use std::path::{PathBuf, Path};
    use std::env;
    use std::{io, fs};

    use temp_dir::TempDir;
    use sha2::{Sha256, Digest};

    use super::convert_command;

    // convert file through all the supported formats back to original file format and check whether the start and end files are identical
    fn convert<P: AsRef<Path>>(start_file_root: P, format: &str, start_file: P) {

        let start_end_ext = match format {
            "djibin" => "bin",
            "avatar" => "png",
            _ => panic!("unsupported format: {}", format)
        };

        let start_file = start_file.as_ref().to_path_buf().with_extension(start_end_ext);
        let end_file = Path::new("end").to_path_buf().with_extension(start_end_ext);

        let start_arg = format!("{format}:{}", start_file.to_str().unwrap());
        let end_arg = format!("{format}:{}", end_file.to_str().unwrap());

        let convert_loop = [
            &start_arg,
            "tilegrid:grid.png",
            "tiledir:tiledir",
            "symdir:symdir",
            &end_arg
        ];

        let start_file_path: PathBuf = [start_file_root.as_ref(), start_file.as_ref()].iter().collect();
        let temp_dir = TempDir::new().unwrap();
        fs::copy(start_file_path, temp_dir.child(start_file.to_str().unwrap())).unwrap();
        fs::copy("symbol_specs/ardu.yaml", temp_dir.child("ardu_symbol_specs.yaml")).unwrap();
        env::set_current_dir(temp_dir.path()).unwrap();

        for args in convert_loop.windows(2) {
            let (from_arg, to_arg) = (args[0], args[1]);
            let options = crate::ConvertOptions { symbol_specs_file: &Path::new("ardu_symbol_specs.yaml").to_path_buf() };
            convert_command(from_arg, to_arg, options).unwrap();
        }

        let [input_hash, output_hash] = [start_file, end_file].map(|file_name| {
            let mut hasher = Sha256::new();
            let mut file = fs::File::open(file_name).unwrap();
            io::copy(&mut file, &mut hasher).unwrap();
            hasher.finalize()
        });

        assert_eq!(input_hash, output_hash);

    }

    #[test]
    fn convert_dji_sd() {
        convert("test_files/djibinsetnorm", "djibin", "font");
    }

    #[test]
    fn convert_dji_hd() {
        convert("test_files/djibinsetnorm", "djibin", "font_hd");
    }

    #[test]
    fn convert_avatar_sd() {
        convert("test_files/avatar", "avatar", "user_ardu_36");
    }

    #[test]
    fn convert_avatar_hd() {
        convert("test_files/avatar", "avatar", "user_ardu_24");
    }

}