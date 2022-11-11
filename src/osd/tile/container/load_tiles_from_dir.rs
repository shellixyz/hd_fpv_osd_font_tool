
use std::{fmt::Display, path::{Path, PathBuf}};

use derive_more::{Error, From};

use crate::osd::tile::{LoadError as TileLoadError, Tile};

#[derive(Debug, Error, From)]
pub enum LoadTilesFromDirError {
    LoadError(TileLoadError),
    NoTileFound,
    KindMismatchError
}

impl Display for LoadTilesFromDirError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use LoadTilesFromDirError::*;
        match self {
            LoadError(load_error) => load_error.fmt(f),
            KindMismatchError => f.write_str("directory contains different kinds of tiles"),
            NoTileFound => f.write_str("no tile found"),
        }
    }
}

pub fn load_tiles_from_dir<P: AsRef<Path>>(path: P, max_tiles: usize) -> Result<Vec<Tile>, LoadTilesFromDirError> {
    let mut tiles = vec![];
    let mut tile_kind = None;

    for index in 0..max_tiles {
        let tile_path: PathBuf = [path.as_ref(), Path::new(&format!("{:03}.png", index))].iter().collect();
        let tile = match Tile::load_image_file(tile_path) {
            Ok(loaded_tile) => Some(loaded_tile),
            Err(error) => match &error {
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
                log::info!("detected {} kind of tiles in {}", tile.kind(), path.as_ref().to_string_lossy());
                tile_kind = Some(tile.kind());
            },

            // we have already loaded a tile before, check that the new tile kind is matching what had recorded
            (Some(tile), Some(tile_kind)) => if tile.kind() != *tile_kind {
                return Err(LoadTilesFromDirError::KindMismatchError)
            },

            _ => {}

        }

        tiles.push(tile);
    }

    let tiles = match tile_kind {
        Some(tile_kind) => {
            let last_some_index = tiles.iter().rposition(Option::is_some).unwrap();
            tiles[0..=last_some_index].iter().map(|tile| tile.clone().unwrap_or_else(|| Tile::new(tile_kind))).collect()
        }
        None => return Err(LoadTilesFromDirError::NoTileFound),
    };

    Ok(tiles)
}
