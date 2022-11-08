use std::fmt::Display;
use std::path::{Path, PathBuf};
use std::io::Error as IOError;

use derive_more::{Error, Display, From};
use image::ImageError;

use crate::osd::bin_file::{BinFileWriter, self, TileWriteError};

use super::Tile;
use super::LoadError as TileLoadError;
use super::grid::Grid as TileGrid;


pub trait SaveTilesToDir {
    fn save_tiles_to_dir<P: AsRef<Path>>(&self, path: P) -> Result<(), ImageError>;
}

impl<T> SaveTilesToDir for T
where
    for<'any> &'any T: IntoIterator<Item = &'any Tile>,
{
    fn save_tiles_to_dir<P: AsRef<Path>>(&self, path: P) -> Result<(), ImageError> {
        std::fs::create_dir_all(&path).unwrap();

        for (index, tile) in self.into_iter().enumerate() {
            let path: PathBuf = [path.as_ref().to_str().unwrap(), &format!("{:03}.png", index)].iter().collect();
            tile.save(path)?;
        }

        Ok(())
    }
}

#[derive(Debug, Error)]
pub enum TileKindError {
    EmptyContainer,
    MultipleTileKinds
}

impl Display for TileKindError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use TileKindError::*;
        match self {
            EmptyContainer => f.write_str("cannot create grid with empty container"),
            MultipleTileKinds => f.write_str("container includes multiple tile kinds"),
        }
    }
}

pub trait UniqTileKind {
    fn tile_kind(&self) -> Result<super::Kind, TileKindError>;
}

impl UniqTileKind for &[Tile] {
    fn tile_kind(&self) -> Result<super::Kind, TileKindError> {
        let first_tile_kind = self.first().ok_or(TileKindError::EmptyContainer)?.kind();
        if ! self.iter().all(|tile| tile.kind() == first_tile_kind) {
            return Err(TileKindError::MultipleTileKinds)
        }
        Ok(first_tile_kind)
    }
}

impl UniqTileKind for Vec<Tile> {
    fn tile_kind(&self) -> Result<super::Kind, TileKindError> {
        self.as_slice().tile_kind()
    }
}

#[derive(Debug, Error, Display, From)]
pub enum SaveTilesToBinFileError {
    CreateError(IOError),
    TileKindError(TileKindError),
    TileWriteError(TileWriteError),
}

pub trait SaveTilesToBinFile {
    fn save_tiles_to_bin_file<P: AsRef<Path>>(&self, path: P) -> Result<(), SaveTilesToBinFileError>;
}

impl SaveTilesToBinFile for &[Tile] {
    fn save_tiles_to_bin_file<P: AsRef<Path>>(&self, path: P) -> Result<(), SaveTilesToBinFileError> {
        self.tile_kind()?;
        let mut writer = BinFileWriter::create(path)?;

        for tile in self.iter() {
            writer.write_tile(tile)?;
        }

        writer.finish()?;

        Ok(())
    }
}

impl SaveTilesToBinFile for TileGrid {
    fn save_tiles_to_bin_file<P: AsRef<Path>>(&self, path: P) -> Result<(), SaveTilesToBinFileError> {
        self.as_slice().save_tiles_to_bin_file(path)
    }
}

impl SaveTilesToBinFile for Vec<Tile> {
    fn save_tiles_to_bin_file<P: AsRef<Path>>(&self, path: P) -> Result<(), SaveTilesToBinFileError> {
        self.as_slice().save_tiles_to_bin_file(path)
    }
}

pub trait SaveToBinFiles {
    fn save_to_bin_files<P: AsRef<Path>>(&self, path1: P, path2: P) -> Result<(), SaveTilesToBinFileError>;
}

impl SaveToBinFiles for &[Tile] {
    fn save_to_bin_files<P: AsRef<Path>>(&self, path1: P, path2: P) -> Result<(), SaveTilesToBinFileError> {
        (&self[0..bin_file::TILE_COUNT]).save_tiles_to_bin_file(path1)?;
        (&self[bin_file::TILE_COUNT..2 * bin_file::TILE_COUNT]).save_tiles_to_bin_file(path2)
    }
}

pub trait IntoTileGrid {
    fn into_tile_grid(self) -> TileGrid;
}

impl IntoTileGrid for &[Tile] {
    fn into_tile_grid(self) -> TileGrid {
        TileGrid::from(self)
    }
}

#[derive(Debug, Error, From)]
pub enum LoadFromDirError {
    LoadError(TileLoadError),
    NoTileFound,
    KindMismatchError
}

impl Display for LoadFromDirError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LoadFromDirError::LoadError(load_error) => load_error.fmt(f),
            LoadFromDirError::KindMismatchError => f.write_str("directory contains different kinds of tiles"),
            LoadFromDirError::NoTileFound => f.write_str("no tile found"),
        }
    }
}

pub fn load_from_dir<P: AsRef<Path> + Display>(path: P, tile_count: usize) -> Result<Vec<Tile>, LoadFromDirError> {
    let mut tiles = vec![];
    let mut tile_kind = None;

    for index in 0..tile_count {
        let tile_path: PathBuf = [path.as_ref().to_str().unwrap(), &format!("{:03}.png", index)].iter().collect();
        let tile = match Tile::load_image_file(tile_path) {
            Ok(loaded_tile) => Some(loaded_tile),
            Err(error) =>
                match &error {
                    TileLoadError::IOError(io_error) =>
                        match io_error.kind() {
                            std::io::ErrorKind::NotFound => None,
                            _ => return Err(error.into()),
                        },
                    _ => return Err(error.into())
                },
        };

        match (&tile, &tile_kind) {

            // first loaded tile: record the kind of tile
            (Some(tile), None) => {
                log::info!("detected {} kind of tiles in {}", tile.kind(), path);
                tile_kind = Some(tile.kind());
            },

            // we have already loaded a tile before, check that the new tile kind is matching what had been recorded
            (Some(tile), Some(tile_kind)) => if tile.kind() != *tile_kind {
                return Err(LoadFromDirError::KindMismatchError)
            },

            _ => {}
        }

        tiles.push(tile);
    }

    let tiles = match tile_kind {
        Some(tile_kind) => tiles.into_iter().map(|tile| tile.unwrap_or_else(|| Tile::new(tile_kind))).collect(),
        None => return Err(LoadFromDirError::NoTileFound),
    };

    Ok(tiles)
}