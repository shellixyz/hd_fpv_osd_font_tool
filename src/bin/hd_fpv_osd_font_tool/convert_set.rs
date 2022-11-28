
use std::cmp::Ordering;

use derive_more::Display;
use thiserror::Error;

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
    if let Some(file_paths) = input.strip_prefix("djibinset:") {
        let files: Vec<&str> = file_paths.split(':').collect();
        match files.len().cmp(&4) {
            Ordering::Less => return Err(InvalidConvertSetArgError::BinSetInvalidArguments("too few arguments")),
            Ordering::Greater => return Err(InvalidConvertSetArgError::BinSetInvalidArguments("too many arguments")),
            Ordering::Equal => {},
        }
        Ok(ConvertSetArg::BinFileSet { sd_path: files[0], sd_2_path: files[1], hd_path: files[2], hd_2_path: files[3] })

    } else if let Some(path) = input.strip_prefix("djibinsetnorm:") {
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

#[derive(Debug, Error)]
pub enum ConvertSetError {
    #[error("invalid `from` argument: {0}")]
    FromArg(InvalidConvertSetArgError),
    #[error("invalid `to` argument: {0}")]
    ToArg(InvalidConvertSetArgError),
}

fn convert_tile_set(tile_set: TileSet, to_arg: &ConvertSetArg, options: &ConvertOptions) -> anyhow::Result<()> {
    use ConvertSetArg::*;
    match to_arg {
        BinFileSet { sd_path, sd_2_path, hd_path, hd_2_path } => tile_set.save_to_bin_files(sd_path, sd_2_path, hd_path, hd_2_path)?,
        BinFileSetNorm { dir, ident } => tile_set.save_to_bin_files_norm(dir, ident)?,
        TileSetGrids { sd_path, hd_path } => tile_set.save_to_grids(sd_path, hd_path)?,
        TileSetGridsNorm { dir, ident  } => tile_set.save_to_grids_norm(dir, ident)?,
        TileSetDir(dir) => tile_set.save_tiles_to_dir(dir)?,
        SymbolSetDir(dir) => {
            let sym_specs = SymbolSpecs::load_file(options.symbol_specs_file())?;
            tile_set.into_symbol_set(&sym_specs).unwrap().save_to_dir(dir)?;
        },
    }
    Ok(())
}

pub fn convert_set_command(from: &str, to: &str, options: ConvertOptions) -> anyhow::Result<()> {
    let from_arg = identify_convert_set_arg(from).map_err(ConvertSetError::FromArg)?;
    let to_arg = identify_convert_set_arg(to).map_err(ConvertSetError::ToArg)?;
    log::info!("converting {} -> {}", from, to);

    use ConvertSetArg::*;
    match (&from_arg, &to_arg) {

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

#[cfg(test)]
mod tests {

    use std::path::Path;

    use hd_fpv_osd_font_tool::osd::tile::container::tile_set::TileSet;
    use itertools::Itertools;
    use temp_dir::TempDir;

    use crate::convert_set::convert_set_command;

    use super::{identify_convert_set_arg, convert_tile_set};

    #[test]
    fn convert_set_all() {
        let formats = [
            // "djibinset",
            "djibinsetnorm",
            // "tilesetgrids",
            "tilesetgridsnorm",
            "tilesetdir",
            "symsetdir"
        ];

        let from_djibinsetnorm = TileSet::load_bin_files_norm("test_files/djibinsetnorm", &None).unwrap();
        let temp_dir = TempDir::new().unwrap();

        for format in formats {
            let to_arg_str = [format, temp_dir.child(format).to_str().unwrap()].join(":");
            let to_arg = identify_convert_set_arg(&to_arg_str).unwrap();
            let options = crate::ConvertOptions { symbol_specs_file: &Path::new("symbol_specs/ardu.yaml").to_path_buf() };
            convert_tile_set(from_djibinsetnorm.clone(), &to_arg, &options).unwrap();
        }

        for testing_formats in formats.iter().permutations(2) {
            let (from_format, to_format) = (testing_formats[0], testing_formats[1]);
            println!("testing {from_format} -> {to_format}");
            let from_arg = [from_format, temp_dir.child(from_format).to_str().unwrap()].join(":");
            let to_arg = [to_format, temp_dir.child(to_format).to_str().unwrap()].join(":");
            let options = crate::ConvertOptions { symbol_specs_file: &Path::new("symbol_specs/ardu.yaml").to_path_buf() };
            convert_set_command(&from_arg, &to_arg, options).unwrap();
        }

    }


}