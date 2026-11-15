use std::{error::Error, fmt::Display, path::Path};

use anyhow::bail;
use hd_fpv_osd_font_tool::{osd::avatar_file, prelude::*};
use thiserror::Error;

use crate::ConvertOptions;

#[derive(Debug)]
pub enum InvalidConvertArgError {
	InvalidPrefix(String),
	InvalidImageFileExtension { path: String, extension: Option<String> },
	InvalidPath(String),
	NoPrefix,
}

impl Error for InvalidConvertArgError {}

impl Display for InvalidConvertArgError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		use InvalidConvertArgError as ICAError;
		match self {
			ICAError::InvalidPrefix(prefix) => write!(f, "invalid prefix: {prefix}"),
			ICAError::NoPrefix => f.write_str("no prefix"),
			ICAError::InvalidImageFileExtension {
				path,
				extension: Some(extension),
			} => write!(f, "invalid image file extension `{extension}`: {path}"),
			ICAError::InvalidImageFileExtension { path, extension: None } => {
				write!(f, "image path has no file extension: {path}")
			},
			ICAError::InvalidPath(path) => write!(f, "invalid path: {path}"),
		}
	}
}

enum ConvertArg<'a> {
	/// Base bin file and optional extended bin file
	BinFile(&'a str, Option<&'a str>),
	AvatarFile(&'a str),
	TileGrid(&'a str),
	TileDir(&'a str),
	SymbolDir(&'a str),
}

fn check_arg_image_file_extension(path: &str) -> Result<(), InvalidConvertArgError> {
	match Path::extension(Path::new(path)) {
		Some(os_str) => match os_str.to_str() {
			Some("png") => Ok(()),
			Some(extension) => Err(InvalidConvertArgError::InvalidImageFileExtension {
				path: path.to_owned(),
				extension: Some(extension.to_owned()),
			}),
			None => Err(InvalidConvertArgError::InvalidPath(path.to_owned())),
		},
		None => Err(InvalidConvertArgError::InvalidImageFileExtension {
			path: path.to_owned(),
			extension: None,
		}),
	}
}

fn identify_convert_arg(input: &'_ str) -> Result<ConvertArg<'_>, InvalidConvertArgError> {
	if let Some(paths) = input.strip_prefix("djibin:") {
		Ok(match paths.split_once(':') {
			Some((base_path, ext_path)) => ConvertArg::BinFile(base_path, Some(ext_path)),
			None => ConvertArg::BinFile(paths, None),
		})
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

/// Saves tiles to a bin file, or to a base and an extended bin file when `ext_path` is provided
fn save_bin_files(tiles: &[Tile], base_path: &str, ext_path: Option<&str>) -> anyhow::Result<()> {
	let Some(ext_path) = ext_path else {
		tiles.save_to_bin_file(base_path)?;
		return Ok(());
	};
	if tiles.len() <= bin_file::TILE_COUNT {
		bail!(
			"cannot write an extended bin file from {} tiles, more than {} are needed",
			tiles.len(),
			bin_file::TILE_COUNT
		);
	}
	let (base_tiles, ext_tiles) = tiles.split_at(bin_file::TILE_COUNT);
	base_tiles.save_to_bin_file(base_path)?;
	ext_tiles.save_to_bin_file(ext_path)?;
	Ok(())
}

fn convert_tiles(tiles: &Vec<Tile>, to_arg: &ConvertArg, options: &ConvertOptions) -> anyhow::Result<()> {
	use ConvertArg as CA;
	match to_arg {
		CA::TileGrid(to_path) => {
			check_arg_image_file_extension(to_path).map_err(ConvertError::ToArg)?;
			tiles.save_to_grid_image(to_path)?;
		},
		CA::TileDir(to_path) => tiles.save_tiles_to_dir(to_path)?,
		CA::SymbolDir(to_path) => {
			let sym_specs = SymbolSpecs::load_file(options.symbol_specs_file())?;
			tiles.to_symbols(&sym_specs)?.save_to_dir(to_path)?;
		},
		CA::BinFile(base_path, ext_path) => save_bin_files(tiles, base_path, *ext_path)?,
		CA::AvatarFile(to_path) => tiles.save_to_avatar_file(to_path)?,
	}
	Ok(())
}

fn convert_tile_grid(tile_grid: &TileGrid, to_arg: &ConvertArg, options: &ConvertOptions) -> anyhow::Result<()> {
	use ConvertArg as CA;
	match to_arg {
		CA::BinFile(base_path, ext_path) => save_bin_files(tile_grid.as_slice(), base_path, *ext_path)?,
		CA::TileDir(to_path) => tile_grid.save_tiles_to_dir(to_path)?,
		CA::SymbolDir(to_path) => {
			let sym_specs = SymbolSpecs::load_file(options.symbol_specs_file())?;
			tile_grid.to_symbols(&sym_specs)?.save_to_dir(to_path)?;
		},
		CA::TileGrid(to_path) => tile_grid.save_image(to_path)?,
		CA::AvatarFile(to_path) => tile_grid.save_tiles_to_avatar_file(to_path)?,
	}
	Ok(())
}

pub fn convert_command(from: &str, to: &str, options: &ConvertOptions) -> anyhow::Result<()> {
	use ConvertArg as CA;

	let from_arg = identify_convert_arg(from).map_err(ConvertError::FromArg)?;
	let to_arg = identify_convert_arg(to).map_err(ConvertError::ToArg)?;
	log::info!("converting {from} -> {to}");

	match (&from_arg, &to_arg) {
		(CA::BinFile(base_path, ext_path), to_arg) => {
			let tiles = match ext_path {
				Some(ext_path) => bin_file::load_extended(*base_path, *ext_path)?,
				None => bin_file::load(base_path)?,
			};
			convert_tiles(&tiles, to_arg, options)?;
		},

		(CA::TileGrid(from_path), to_arg) => {
			check_arg_image_file_extension(from_path).map_err(ConvertError::FromArg)?;
			let tile_grid = crate::TileGrid::load_from_image(from_path)?;
			convert_tile_grid(&tile_grid, to_arg, options)?;
		},

		(CA::TileDir(from_path), to_arg) => {
			let tiles = load_tiles_from_dir(from_path, avatar_file::MAX_TILE_COUNT)?;
			convert_tiles(&tiles, to_arg, options)?;
		},

		(CA::SymbolDir(from_path), to_arg) => {
			let tiles = load_symbols_from_dir(from_path, avatar_file::MAX_TILE_COUNT)?.into_tiles_vec();
			convert_tiles(&tiles, to_arg, options)?;
		},

		(CA::AvatarFile(from_path), to_arg) => {
			let tiles = load_avatar_file(from_path)?;
			convert_tiles(&tiles, to_arg, options)?;
		},
	}

	Ok(())
}

#[cfg(test)]
mod tests {

	use std::{
		fs,
		io::Read,
		path::{Path, PathBuf},
	};

	use hd_fpv_osd_font_tool::{
		osd::tile,
		prelude::bin_file::{self, FontPart},
	};
	use itertools::Itertools;
	use sha2::{Digest, Sha256};
	use strum::IntoEnumIterator;
	use temp_dir::TempDir;

	use super::convert_command;

	fn files_are_identical(files: &[PathBuf]) -> bool {
		files
			.iter()
			.map(|file_path| {
				let mut hasher = Sha256::new();
				let mut file = fs::File::open(file_path).unwrap();
				let mut buffer = [0; 8192];
				loop {
					let bytes_read = file.read(&mut buffer).unwrap();
					if bytes_read == 0 {
						break;
					}
					hasher.update(&buffer[..bytes_read]);
				}
				hasher.finalize().to_vec()
			})
			.tuple_windows()
			.all(|(left, right)| left == right)
	}

	fn convert(from: &str, to: &str) {
		let options = crate::ConvertOptions {
			symbol_specs_file: &Path::new("symbol_specs/ardu.yaml").to_path_buf(),
		};
		convert_command(from, to, &options).unwrap();
	}

	fn bin_paths(tile_kind: tile::Kind) -> (PathBuf, PathBuf) {
		let dir = "test_files/djibinsetnorm";
		(
			bin_file::normalized_file_path(dir, tile_kind, None, FontPart::Base),
			bin_file::normalized_file_path(dir, tile_kind, None, FontPart::Ext),
		)
	}

	#[test]
	fn djibin_pair_round_trips_through_two_page_avatar() {
		let temp_dir = TempDir::new().unwrap();
		for tile_kind in tile::Kind::iter() {
			let (base, ext) = bin_paths(tile_kind);
			let avatar = temp_dir.child(format!("{tile_kind}.png"));
			convert(
				&format!("djibin:{}:{}", base.display(), ext.display()),
				&format!("avatar:{}", avatar.display()),
			);
			let tile_dimensions = tile_kind.dimensions();
			assert_eq!(
				image::image_dimensions(&avatar).unwrap(),
				(2 * tile_dimensions.width, 256 * tile_dimensions.height)
			);

			let (base_out, ext_out) = (
				temp_dir.child(format!("{tile_kind}_base.bin")),
				temp_dir.child(format!("{tile_kind}_ext.bin")),
			);
			convert(
				&format!("avatar:{}", avatar.display()),
				&format!("djibin:{}:{}", base_out.display(), ext_out.display()),
			);
			assert!(files_are_identical(&[base, base_out]));
			assert!(files_are_identical(&[ext, ext_out]));
		}
	}

	#[test]
	fn four_page_avatar_round_trips_through_tiledir_and_symdir() {
		let temp_dir = TempDir::new().unwrap();
		for tile_kind in tile::Kind::iter() {
			let (base, ext) = bin_paths(tile_kind);
			let tiledir = temp_dir.child(format!("{tile_kind}_tiles"));
			convert(
				&format!("djibin:{}:{}", base.display(), ext.display()),
				&format!("tiledir:{}", tiledir.display()),
			);
			// Pages 2 and 3 repeat pages 0 and 1, giving a 1024-tile collection
			for index in 0..512 {
				fs::copy(
					tiledir.join(format!("{index:03}.png")),
					tiledir.join(format!("{:03}.png", index + 512)),
				)
				.unwrap();
			}

			let avatar = temp_dir.child(format!("{tile_kind}_4.png"));
			convert(
				&format!("tiledir:{}", tiledir.display()),
				&format!("avatar:{}", avatar.display()),
			);
			let tile_dimensions = tile_kind.dimensions();
			assert_eq!(
				image::image_dimensions(&avatar).unwrap(),
				(4 * tile_dimensions.width, 256 * tile_dimensions.height)
			);

			let symdir = temp_dir.child(format!("{tile_kind}_symbols"));
			convert(
				&format!("avatar:{}", avatar.display()),
				&format!("symdir:{}", symdir.display()),
			);
			let from_symdir = temp_dir.child(format!("{tile_kind}_from_symdir.png"));
			convert(
				&format!("symdir:{}", symdir.display()),
				&format!("avatar:{}", from_symdir.display()),
			);
			assert!(files_are_identical(&[avatar, from_symdir]));
		}
	}

	#[test]
	fn convert_all() {
		let formats = ["djibin", "avatar", "tilegrid", "tiledir", "symdir"];

		let temp_dir = TempDir::new().unwrap();

		for tile_kind in tile::Kind::iter() {
			let from_djibin =
				bin_file::normalized_file_path("test_files/djibinsetnorm", tile_kind, None, FontPart::Base);
			let from_arg = format!("djibin:{}", from_djibin.to_str().unwrap());
			for to_format in formats {
				println!("testing djibin ({tile_kind}) -> {to_format}");
				let to_name = format!("{to_format}_{tile_kind}");
				let to_rel_path = match to_format {
					"djibin" => format!("{to_name}.bin"),
					"tilegrid" | "avatar" => format!("{to_name}.png"),
					_ => to_name,
				};
				let to_path = temp_dir.child(to_rel_path);
				let to_arg = format!("{to_format}:{}", to_path.to_str().unwrap());
				let options = crate::ConvertOptions {
					symbol_specs_file: &Path::new("symbol_specs/ardu.yaml").to_path_buf(),
				};
				convert_command(&from_arg, &to_arg, &options).unwrap();
			}
		}

		for tile_kind in tile::Kind::iter() {
			for testing_formats in formats.iter().permutations(2) {
				let (from_format, to_format) = (testing_formats[0], testing_formats[1]);
				println!("testing {from_format} ({tile_kind}) -> {to_format}");

				let from_name = format!("{from_format}_{tile_kind}");
				let from_rel_path = match *from_format {
					"djibin" => format!("{from_name}.bin"),
					"tilegrid" | "avatar" => format!("{from_name}.png"),
					_ => from_name,
				};

				let to_name = format!("{to_format}_{tile_kind}");
				let to_rel_path = match *to_format {
					"djibin" => format!("{to_name}_from_{from_format}.bin"),
					"tilegrid" | "avatar" => format!("{to_name}_from_{from_format}.png"),
					_ => format!("{to_name}_from_{from_format}"),
				};

				let from_path = temp_dir.child(from_rel_path);
				let to_path = temp_dir.child(to_rel_path);
				let from_arg = format!("{from_format}:{}", from_path.to_str().unwrap());
				let to_arg = format!("{to_format}:{}", to_path.to_str().unwrap());
				let options = crate::ConvertOptions {
					symbol_specs_file: &Path::new("symbol_specs/ardu.yaml").to_path_buf(),
				};
				convert_command(&from_arg, &to_arg, &options).unwrap();
			}
		}

		for tile_kind in tile::Kind::iter() {
			// DJI BIN
			let original_djibin =
				bin_file::normalized_file_path("test_files/djibinsetnorm", tile_kind, None, FontPart::Base);

			let generated_files = ["avatar", "tilegrid", "tiledir", "symdir"]
				.map(|format| temp_dir.child(format!("djibin_{tile_kind}_from_{format}.bin")));
			let files = [original_djibin]
				.into_iter()
				.chain(generated_files)
				.collect::<Vec<PathBuf>>();
			assert!(files_are_identical(&files));

			// AVATAR
			let generated_files = ["djibin", "tilegrid", "tiledir", "symdir"]
				.map(|format| temp_dir.child(format!("avatar_{tile_kind}_from_{format}.png")));
			assert!(files_are_identical(&generated_files));
		}
	}
}
