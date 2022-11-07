
use std::{path::Path, fmt::Display, error::Error};

use super::{Tile, TileIter, grid::{StandardSizeGrid, ExtendedSizeGrid}, LoadError as TileLoadError, Kind as TileKind};
use crate::osd::bin_file::{BinFileReader, SeekReadError as BinFileSeekReadError};
use array_macro::array;
use derive_more::Index;
use paste::paste;


pub const STANDARD_TILE_COUNT: usize = 256;
pub const EXTENDED_TILE_COUNT: usize = STANDARD_TILE_COUNT * 2;
pub trait StandardTileContainer {}
pub trait StandardSizeContainer {}
pub trait ExtendedSizeContainer {
    fn first_half(&self) -> StandardSizeArray;
    fn second_half(&self) -> StandardSizeArray;
}

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
pub struct TileKindMismatchError { pub container_tile_kind: TileKind, pub received_tile_kind: TileKind }
impl Error for TileKindMismatchError {}

impl Display for TileKindMismatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "mismatched tile kind: container is for {} tile kind, received {} tile kind ", self.container_tile_kind, self.received_tile_kind)
    }
}

mod array_utils {
    use std::{path::{Path, PathBuf}, fmt::Display};
    use crate::osd::tile::Tile;
    use super::{LoadFromDirError, TileLoadError};

    pub(super) fn load_from_dir<P: AsRef<Path> + Display>(path: P, tile_count: usize) -> Result<Vec<Tile>, LoadFromDirError> {
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

}

pub trait GetTileKind {
    // returns the kind of tiles this container can store
    fn tile_kind(&self) -> TileKind;
}

pub trait ReplaceTile: GetTileKind {
    fn replace_tile(&mut self, index: usize, tile: Tile) -> Result<(), TileKindMismatchError>;
}

macro_rules! container {
    ($type_name:ident, $size_ident_trait:ty, $size:expr) => {

        paste! {
            type [<$type_name Inner>] = [Tile; $size];

            #[derive(Index)]
            pub struct $type_name(pub(crate) [<$type_name Inner>]);
        }

        impl $type_name {

            pub fn new(tile_kind: TileKind) -> Self {
                Self(array![Tile::new(tile_kind); $size])
            }

            paste! {
                // Load at most 256 tiles from the specified directory, all the tiles must be of the same kind.
                // The name of the files must be in the format "{:03}.png"
                pub fn load_from_dir<P: AsRef<Path> + Display>(path: P) -> Result<Self, LoadFromDirError> {
                    let tiles = array_utils::load_from_dir(path, $size)?;
                    Ok(Self([<$type_name Inner>]::try_from(tiles).unwrap()))
                }
            }

            pub fn iter(&self) -> TileIter<Self> {
                self.into_iter()
            }

        }

        impl GetTileKind for $type_name {

            fn tile_kind(&self) -> TileKind {
                // we can just return the kind of the first tile since the container can only contain one kind of tile
                self.0[0].kind()
            }

        }

        impl ReplaceTile for $type_name {

            fn replace_tile(&mut self, index: usize, tile: Tile) -> Result<(), TileKindMismatchError> {
                if self.tile_kind() != tile.kind() {
                    return Err(TileKindMismatchError { container_tile_kind: self.tile_kind(), received_tile_kind: tile.kind() });
                }
                self.0[index] = tile;
                Ok(())
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

        impl<'a> Iterator for TileIter<'a, $type_name> {
            type Item = &'a Tile;

            fn next(&mut self) -> Option<Self::Item> {
                if self.index >= $size {
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

        impl<'a> IntoIterator for &'a $type_name {
            type Item = &'a Tile;

            type IntoIter = TileIter<'a, $type_name>;

            fn into_iter(self) -> Self::IntoIter {
                Self::IntoIter { container: self, index: 0 }
            }
        }

    };
}

container!(StandardSizeArray, StandardSizeContainer, STANDARD_TILE_COUNT);
container!(ExtendedSizeArray, ExtendedSizeContainer, EXTENDED_TILE_COUNT);

impl StandardTileContainer for StandardSizeArray {}
impl StandardTileContainer for &StandardSizeArray {}
impl StandardSizeContainer for StandardSizeArray {}
impl StandardSizeContainer for &StandardSizeArray {}

impl StandardSizeArray {

    pub fn extend(self, other: Self) -> Result<ExtendedSizeArray, TileKindMismatchError> {
        if self.tile_kind() != other.tile_kind() {
            return Err(TileKindMismatchError { container_tile_kind: self.tile_kind(), received_tile_kind: other.tile_kind() });
        }
        let mut array = ExtendedSizeArray::new(self.tile_kind());
        let tiles = self.0.into_iter().chain(other.0.into_iter()).collect::<Vec<Tile>>();
        array.0.clone_from_slice(&tiles);
        Ok(array)
    }

    pub fn into_grid(self) -> StandardSizeGrid {
        StandardSizeGrid::from(self)
    }

}

impl StandardTileContainer for ExtendedSizeArray {}
impl StandardTileContainer for &ExtendedSizeArray {}

impl ExtendedSizeArray {
    pub fn into_grid(self) -> ExtendedSizeGrid {
        ExtendedSizeGrid::from(self)
    }
}

impl ExtendedSizeContainer for &ExtendedSizeArray {
    fn first_half(&self) -> StandardSizeArray {
        (**self).first_half()
    }

    fn second_half(&self) -> StandardSizeArray {
        (**self).second_half()
    }
}

impl ExtendedSizeContainer for ExtendedSizeArray {

    fn first_half(&self) -> StandardSizeArray {
        let mut array = StandardSizeArray::new(self.tile_kind());
        array.0.clone_from_slice(&self.0[0..STANDARD_TILE_COUNT]);
        array
    }

    fn second_half(&self) -> StandardSizeArray {
        let mut array = StandardSizeArray::new(self.tile_kind());
        array.0.clone_from_slice(&self.0[STANDARD_TILE_COUNT..EXTENDED_TILE_COUNT]);
        array
    }

}

impl TryFrom<&mut BinFileReader> for StandardSizeArray {
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

impl From<&StandardSizeGrid> for StandardSizeArray {
    fn from(tile_grid: &StandardSizeGrid) -> Self {
        let mut array = Self::new(tile_grid.tile_kind());
        let mut tile_grid_iterator = tile_grid.iter();
        for tile in &mut array.0 {
            *tile = tile_grid_iterator.next().unwrap().clone();
        }
        array
    }
}