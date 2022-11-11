
use std::{io::Error as IOError, path::Path};

use derive_more::{Error, Display, From};
use crate::osd::{tile::{Tile, grid::Grid as TileGrid}, bin_file::{self, BinFileWriter}};
use super::uniq_tile_kind::{TileKindError, UniqTileKind};


#[derive(Debug, Error, Display, From)]
pub enum SaveTilesToBinFileError {
    CreateError(IOError),
    TileKindError(TileKindError),
    TileWriteError(bin_file::TileWriteError),
    FillRemainingSpaceError(bin_file::FillRemainingSpaceError)
}

pub trait SaveToBinFile {
    fn save_to_bin_file<P: AsRef<Path>>(&self, path: P) -> Result<(), SaveTilesToBinFileError>;
}

impl SaveToBinFile for &[Tile] {
    fn save_to_bin_file<P: AsRef<Path>>(&self, path: P) -> Result<(), SaveTilesToBinFileError> {
        self.tile_kind()?;
        let mut writer = BinFileWriter::create(path)?;

        for tile in self.iter() {
            writer.write_tile(tile)?;
        }

        writer.fill_remaining_space()?;
        writer.finish()?;
        Ok(())
    }
}

impl SaveToBinFile for Vec<Tile> {
    fn save_to_bin_file<P: AsRef<Path>>(&self, path: P) -> Result<(), SaveTilesToBinFileError> {
        self.as_slice().save_to_bin_file(path)
    }
}

pub trait SaveTilesToBinFile {
    fn save_tiles_to_bin_file<P: AsRef<Path>>(&self, path: P) -> Result<(), SaveTilesToBinFileError>;
}

impl SaveTilesToBinFile for TileGrid {
    fn save_tiles_to_bin_file<P: AsRef<Path>>(&self, path: P) -> Result<(), SaveTilesToBinFileError> {
        self.as_slice().save_to_bin_file(path)
    }
}

pub trait SaveToBinFiles {
    fn save_to_bin_files<P: AsRef<Path>>(&self, path1: P, path2: P) -> Result<(), SaveTilesToBinFileError>;
}

impl SaveToBinFiles for &[Tile] {
    fn save_to_bin_files<P: AsRef<Path>>(&self, path1: P, path2: P) -> Result<(), SaveTilesToBinFileError> {
        (&self[0..bin_file::TILE_COUNT]).save_to_bin_file(path1)?;
        (&self[bin_file::TILE_COUNT..2 * bin_file::TILE_COUNT]).save_to_bin_file(path2)
    }
}

impl SaveToBinFiles for Vec<Tile> {
    fn save_to_bin_files<P: AsRef<Path>>(&self, path1: P, path2: P) -> Result<(), SaveTilesToBinFileError> {
        self.as_slice().save_to_bin_files(path1, path2)
    }
}
