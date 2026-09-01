use std::{io::Error as IOError, path::Path};

use derive_more::{Display, Error, From};

use super::uniq_tile_kind::{TileKindError, UniqTileKind};
use crate::{
	create_path::{CreatePathError, create_path},
	osd::{
		bin_file::{self, BinFileWriter},
		tile::{Tile, grid::Grid as TileGrid},
	},
	prelude::bin_file::FontPart,
};

#[derive(Debug, Error, Display, From)]
pub enum SaveTilesToBinFileError {
	CreatePathError(CreatePathError),
	CreateError(IOError),
	TileKindError(TileKindError),
	TileWriteError(bin_file::TileWriteError),
	FillRemainingSpaceError(bin_file::FillRemainingSpaceError),
}

pub trait SaveToBinFile {
	/// Saves tiles to a .bin OSD font file
	///
	/// # Errors
	/// Returns `SaveTilesToBinFileError` if saving fails
	fn save_to_bin_file<P: AsRef<Path>>(&self, path: P) -> Result<(), SaveTilesToBinFileError>;

	/// Saves tiles to a .bin OSD font file with normalized naming
	///
	/// # Errors
	/// Returns `SaveTilesToBinFileError` if saving fails
	fn save_to_bin_file_norm<P: AsRef<Path>>(
		&self,
		dir: P,
		ident: Option<&str>,
		part: FontPart,
	) -> Result<(), SaveTilesToBinFileError>;
}

impl SaveToBinFile for &[Tile] {
	fn save_to_bin_file<P: AsRef<Path>>(&self, path: P) -> Result<(), SaveTilesToBinFileError> {
		self.tile_kind()?;
		let mut writer = BinFileWriter::create(path)?;

		for tile in *self {
			writer.write_tile(tile)?;
		}

		writer.fill_remaining_space()?;
		writer.finish()?;
		Ok(())
	}

	fn save_to_bin_file_norm<P: AsRef<Path>>(
		&self,
		dir: P,
		ident: Option<&str>,
		part: FontPart,
	) -> Result<(), SaveTilesToBinFileError> {
		create_path(&dir)?;
		self.save_to_bin_file(bin_file::normalized_file_path(dir, self.tile_kind()?, ident, part))
	}
}

impl SaveToBinFile for Vec<Tile> {
	fn save_to_bin_file<P: AsRef<Path>>(&self, path: P) -> Result<(), SaveTilesToBinFileError> {
		self.as_slice().save_to_bin_file(path)
	}

	fn save_to_bin_file_norm<P: AsRef<Path>>(
		&self,
		dir: P,
		ident: Option<&str>,
		part: FontPart,
	) -> Result<(), SaveTilesToBinFileError> {
		self.as_slice().save_to_bin_file_norm(dir, ident, part)
	}
}

pub trait SaveTilesToBinFile {
	/// Saves tiles to a .bin OSD font file
	///
	/// # Errors
	/// Returns `SaveTilesToBinFileError` if saving fails
	fn save_tiles_to_bin_file<P: AsRef<Path>>(&self, path: P) -> Result<(), SaveTilesToBinFileError>;
}

impl SaveTilesToBinFile for TileGrid {
	fn save_tiles_to_bin_file<P: AsRef<Path>>(&self, path: P) -> Result<(), SaveTilesToBinFileError> {
		self.as_slice().save_to_bin_file(path)
	}
}

pub trait SaveToBinFiles {
	/// Saves tiles to two .bin OSD font files (one for the first 256 tiles, one for the last 256
	/// tiles)
	///
	/// # Errors
	/// Returns `SaveTilesToBinFileError` if saving fails
	fn save_to_bin_files<P: AsRef<Path>>(&self, path1: P, path2: P) -> Result<(), SaveTilesToBinFileError>;

	/// Saves tiles to two .bin OSD font files with normalized naming
	///
	/// # Errors
	/// Returns `SaveTilesToBinFileError` if saving fails
	fn save_to_bin_files_norm<P: AsRef<Path>>(
		&self,
		dir: P,
		ident: Option<&str>,
	) -> Result<(), SaveTilesToBinFileError>;
}

impl SaveToBinFiles for &[Tile] {
	fn save_to_bin_files<P: AsRef<Path>>(&self, path1: P, path2: P) -> Result<(), SaveTilesToBinFileError> {
		(&self[0..bin_file::TILE_COUNT]).save_to_bin_file(path1)?;
		(&self[bin_file::TILE_COUNT..2 * bin_file::TILE_COUNT]).save_to_bin_file(path2)
	}

	fn save_to_bin_files_norm<P: AsRef<Path>>(
		&self,
		dir: P,
		ident: Option<&str>,
	) -> Result<(), SaveTilesToBinFileError> {
		(&self[0..bin_file::TILE_COUNT]).save_to_bin_file_norm(&dir, ident, FontPart::Base)?;
		(&self[bin_file::TILE_COUNT..2 * bin_file::TILE_COUNT]).save_to_bin_file_norm(&dir, ident, FontPart::Ext)
	}
}

impl SaveToBinFiles for Vec<Tile> {
	fn save_to_bin_files<P: AsRef<Path>>(&self, path1: P, path2: P) -> Result<(), SaveTilesToBinFileError> {
		self.as_slice().save_to_bin_files(path1, path2)
	}

	fn save_to_bin_files_norm<P: AsRef<Path>>(
		&self,
		dir: P,
		ident: Option<&str>,
	) -> Result<(), SaveTilesToBinFileError> {
		self.as_slice().save_to_bin_files_norm(dir, ident)
	}
}
