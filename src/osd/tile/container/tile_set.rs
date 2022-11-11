
use std::fmt::Display;
use std::ops::Index;
use std::path::Path;

use derive_more::{Display, Error, From};
use getset::Getters;
use strum::IntoEnumIterator;

use crate::osd::tile::container::UniqTileKind;
use crate::osd::tile::{Kind as TileKind, Tile};
use crate::osd::tile::grid::{Grid as TileGrid, LoadError as GridLoadError};

use super::load_tiles_from_dir::{load_tiles_from_dir, LoadTilesFromDirError};
use super::save_tiles_to_dir::{SaveTilesToDir, SaveTilesToDirError};

#[derive(Debug)]
pub enum TileSetFromError {
    TileKindMismatch(TileKind),
    WrongTileKind(TileKind),
}

impl std::error::Error for TileSetFromError {}

impl Display for TileSetFromError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use TileSetFromError::*;
        match self {
            TileKindMismatch(collection_kind) => write!(f, "mismatched tile kinds in {collection_kind} collection"),
            WrongTileKind(collection_kind) => write!(f, "wrong tile kind in {collection_kind} collection"),
        }
    }
}

#[derive(Debug, Display, Error, From)]
pub enum LoadTileSetTilesFromDirError {
    LoadTilesFromDirError(LoadTilesFromDirError),
    TileSetFromError(TileSetFromError),
}

#[derive(Debug, Display, Error, From)]
pub enum LoadFromTileGridsError {
    GridImageLoadError(GridLoadError),
    TileSetFromError(TileSetFromError),
}

#[derive(Getters)]
#[getset(get = "pub")]
pub struct TileSet {
    pub(crate) sd_tiles: Vec<Tile>,
    pub(crate) hd_tiles: Vec<Tile>,
}

impl TileSet {

    pub fn try_from(sd_tiles: Vec<Tile>, hd_tiles: Vec<Tile>) -> Result<Self, TileSetFromError> {
        use TileSetFromError::*;
        let sd_collection_kind = sd_tiles.tile_kind().map_err(|_| TileKindMismatch(TileKind::SD))?;
        if sd_collection_kind != TileKind::SD {
            return Err(WrongTileKind(TileKind::SD))
        }
        let hd_collection_kind = hd_tiles.tile_kind().map_err(|_| TileKindMismatch(TileKind::HD))?;
        if hd_collection_kind != TileKind::HD {
            return Err(WrongTileKind(TileKind::HD))
        }
        Ok(Self { sd_tiles, hd_tiles })
    }

    pub fn load_tiles_from_dir<P: AsRef<Path>>(path: P, max_tiles: usize) -> Result<Self, LoadTileSetTilesFromDirError> {
        let sd_tiles = load_tiles_from_dir(TileKind::SD.set_dir_path(&path), max_tiles)?;
        let hd_tiles = load_tiles_from_dir(TileKind::HD.set_dir_path(&path), max_tiles)?;
        Ok(Self::try_from(sd_tiles, hd_tiles)?)
    }

    pub fn load_from_tile_grids<P: AsRef<Path>>(sd_grid_path: P, hd_grid_path: P) -> Result<Self, LoadFromTileGridsError> {
        let sd_tiles = TileGrid::load_from_image(sd_grid_path)?.to_vec();
        let hd_tiles = TileGrid::load_from_image(hd_grid_path)?.to_vec();
        Ok(Self::try_from(sd_tiles, hd_tiles)?)
    }

}

impl Index<TileKind> for TileSet {
    type Output = Vec<Tile>;

    fn index(&self, tile_kind: TileKind) -> &Self::Output {
        match tile_kind {
            TileKind::SD => &self.sd_tiles,
            TileKind::HD => &self.hd_tiles,
        }
    }
}

impl SaveTilesToDir for TileSet {

    fn save_tiles_to_dir<P: AsRef<Path>>(&self, path: P) -> Result<(), SaveTilesToDirError> {
        for tile_kind in TileKind::iter() {
            self[tile_kind].save_tiles_to_dir(tile_kind.set_dir_path(&path))?;
        }
        Ok(())
    }

}
