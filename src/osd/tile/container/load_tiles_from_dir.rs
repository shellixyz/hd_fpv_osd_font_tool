
use std::path::{Path, PathBuf};

use thiserror::Error;

use crate::osd::tile::{LoadError as TileLoadError, Tile};
use crate::image::ReadError as ImageReadError;


#[derive(Debug, Error)]
pub enum LoadTilesFromDirError {
    #[error("error loading tile: {0}")]
    TileLoadError(TileLoadError),
    #[error("no tile found in directory: {0}")]
    NoTileFound(PathBuf),
    #[error("directory should contain a single kind of tile: {0}")]
    KindMismatch(PathBuf)
}

impl LoadTilesFromDirError {
    pub fn kind_mismatch<P: AsRef<Path>>(dir_path: P) -> Self {
        Self::KindMismatch(dir_path.as_ref().to_path_buf())
    }

    pub fn no_tile_found<P: AsRef<Path>>(dir_path: P) -> Self {
        Self::NoTileFound(dir_path.as_ref().to_path_buf())
    }
}

impl From<TileLoadError> for LoadTilesFromDirError {
    fn from(error: TileLoadError) -> Self {
        Self::TileLoadError(error)
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
                TileLoadError::ImageReadError(ImageReadError::OpenError { file_path: _, error: open_error }) =>
                    match open_error.kind() {
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
                return Err(LoadTilesFromDirError::kind_mismatch(&path))
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
        None => return Err(LoadTilesFromDirError::no_tile_found(&path)),
    };

    Ok(tiles)
}
