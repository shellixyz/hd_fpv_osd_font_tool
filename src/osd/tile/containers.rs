
use std::{path::{Path, PathBuf}, fmt::Display, error::Error};

use super::{Tile, TileIter, grid::TileGrid, LoadError as TileLoadError, Kind as TileKind};
use crate::osd::bin_file::{BinFileReader, SeekReadError as BinFileSeekReadError};
use array_macro::array;
use derive_more::Index;


pub const STANDARD_TILE_COUNT: usize = 256;
pub trait StandardSizeTileContainer {}

#[derive(Debug)]
pub enum LoadFromDirError {
    LoadError(TileLoadError),
    NoTileFound,
    KindMismatchError
}

impl Error for LoadFromDirError {}

impl Display for LoadFromDirError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LoadFromDirError::LoadError(load_error) => load_error.fmt(f),
            LoadFromDirError::KindMismatchError => f.write_str("directory contains different kinds of tiles"),
            LoadFromDirError::NoTileFound => f.write_str("no tile found"),
        }
    }
}

impl From<TileLoadError> for LoadFromDirError {
    fn from(load_error: TileLoadError) -> Self {
        Self::LoadError(load_error)
    }
}

#[derive(Debug)]
pub struct WrongTileKindError(TileKind);
impl Error for WrongTileKindError {}

impl Display for WrongTileKindError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "wrong tile kind: {:?}", self.0)
    }
}

type StandardSizeTileArrayInner = [Tile; STANDARD_TILE_COUNT];

#[derive(Index)]
pub struct StandardSizeTileArray(pub(crate) StandardSizeTileArrayInner);

impl StandardSizeTileContainer for StandardSizeTileArray {}
impl StandardSizeTileContainer for &StandardSizeTileArray {}

impl StandardSizeTileArray {

    pub fn new(tile_kind: TileKind) -> Self {
        Self(array![Tile::new(tile_kind); STANDARD_TILE_COUNT])
    }

    pub fn into_grid(self) -> TileGrid {
        TileGrid::from(self)
    }

    // Load at most 256 tiles from the specified directory, all the tiles must be of the same kind.
    // The name of the files must be in the format "{:03}.png"
    pub fn load_from_dir<P: AsRef<Path> + std::fmt::Display>(path: P) -> Result<Self, LoadFromDirError> {
        let mut tiles = array![None; STANDARD_TILE_COUNT];
        let mut tile_kind = None;

        for (index, array_tile) in tiles.iter_mut().enumerate() {
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

            *array_tile = tile;
        }

        let tiles = match tile_kind {
            Some(tile_kind) => tiles.map(|tile| tile.unwrap_or_else(|| Tile::new(tile_kind))),
            None => return Err(LoadFromDirError::NoTileFound),
        };

        Ok(Self(tiles))
    }

    // returns the kind of tiles this container can store
    pub fn tile_kind(&self) -> TileKind {
        // we can just return the kind of the first tile since the container can only contain one kind of tile
        self.0[0].kind()
    }

    pub fn replace_tile(&mut self, index: usize, tile: Tile) -> Result<(), WrongTileKindError> {
        if self.tile_kind() != tile.kind() {
            return Err(WrongTileKindError(tile.kind()));
        }
        self.0[index] = tile;
        Ok(())
    }

    pub fn iter(&self) -> TileIter<Self> {
        self.into_iter()
    }

}

// impl Iterator for TileIntoIter<StandardSizeTileArray> {
//     type Item = Tile;

//     fn next(&mut self) -> Option<Self::Item> {
//         if self.index >= TILE_COUNT {
//             return None;
//         }
//         let tile = self.container.0[self.index].clone();
//         self.index += 1;
//         Some(tile)
//     }
// }

impl<'a> Iterator for TileIter<'a, StandardSizeTileArray> {
    type Item = &'a Tile;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= STANDARD_TILE_COUNT {
            return None;
        }
        let tile = &self.container.0[self.index];
        self.index += 1;
        Some(tile)
    }
}

// impl IntoIterator for StandardSizeTileArray {
//     type Item = Tile;

//     type IntoIter = TileIntoIter<Self>;

//     fn into_iter(self) -> Self::IntoIter {
//         Self::IntoIter { container: self, index: 0 }
//     }
// }

impl<'a> IntoIterator for &'a StandardSizeTileArray {
    type Item = &'a Tile;

    type IntoIter = TileIter<'a, StandardSizeTileArray>;

    fn into_iter(self) -> Self::IntoIter {
        Self::IntoIter { container: self, index: 0 }
    }
}

impl TryFrom<&mut BinFileReader> for StandardSizeTileArray {
    type Error = BinFileSeekReadError;

    fn try_from(file: &mut BinFileReader) -> Result<Self, Self::Error> {
        file.rewind().map_err(BinFileSeekReadError::SeekError)?;
        let first_tile = file.read_tile()?;
        let mut array = Self::new(first_tile.kind());
        array.0[0] = first_tile;
        for tile in &mut array.0[1..] {
            *tile = file.read_tile()?;
        }
        Ok(array)
    }
}

impl From<&TileGrid> for StandardSizeTileArray {
    fn from(tile_grid: &TileGrid) -> Self {
        let mut array = Self::new(tile_grid.tile_kind());
        let mut tile_grid_iterator = tile_grid.iter();
        for tile in &mut array.0 {
            *tile = tile_grid_iterator.next().unwrap().clone();
        }
        array
    }
}