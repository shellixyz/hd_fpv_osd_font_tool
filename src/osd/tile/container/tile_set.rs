
use std::ops::Index;
use std::path::Path;

use derive_more::{Display, Error, From};
use getset::Getters;
use strum::IntoEnumIterator;

use crate::osd::tile::container::UniqTileKind;
use crate::osd::tile::{Kind as TileKind, Tile};
use crate::osd::tile::grid::{Grid as TileGrid, LoadError as GridLoadError};
use super::save_to_bin_file::{SaveToBinFiles, SaveTilesToBinFileError};
use super::save_to_grid::SaveToGridImage;
use super::uniq_tile_kind::TileKindError;
use super::{IntoTilesVec, ToSymbols};
use super::load_tiles_from_dir::{load_tiles_from_dir, LoadTilesFromDirError};
use super::save_tiles_to_dir::{SaveTilesToDir, SaveTilesToDirError};
use super::symbol::set::Set as SymbolSet;
use super::symbol::spec::Specs as SymbolSpecs;
use crate::osd::tile::grid::SaveImageError as SaveGridImageError;


#[derive(Debug, Display, Error, From)]
pub enum LoadTileSetTilesFromDirError {
    LoadTilesFromDirError(LoadTilesFromDirError),
    TileKindError(TileKindError),
}

#[derive(Debug, Display, Error, From)]
pub enum LoadFromTileGridsError {
    GridImageLoadError(GridLoadError),
    TileKindError(TileKindError),
}

#[derive(Getters)]
#[getset(get = "pub")]
pub struct TileSet {
    pub(crate) sd_tiles: Vec<Tile>,
    pub(crate) hd_tiles: Vec<Tile>,
}

impl TileSet {

    fn check_collection_kind(tiles: &[Tile], expected_tile_kind: TileKind) -> Result<(), TileKindError> {
        let tile_kind = tiles.tile_kind()?;
        if tile_kind != expected_tile_kind {
            return Err(TileKindError::LoadedDoesNotMatchRequested { requested: expected_tile_kind, loaded: tile_kind })
        }
        Ok(())
    }

    pub fn try_from_tiles(sd_tiles: Vec<Tile>, hd_tiles: Vec<Tile>) -> Result<Self, TileKindError> {
        Self::check_collection_kind(&sd_tiles, TileKind::SD)?;
        Self::check_collection_kind(&hd_tiles, TileKind::HD)?;
        Ok(Self { sd_tiles, hd_tiles })
    }

    pub fn load_from_dir<P: AsRef<Path>>(path: P, max_tiles: usize) -> Result<Self, LoadTileSetTilesFromDirError> {
        let sd_tiles = load_tiles_from_dir(TileKind::SD.set_dir_path(&path), max_tiles)?;
        let hd_tiles = load_tiles_from_dir(TileKind::HD.set_dir_path(&path), max_tiles)?;
        Ok(Self::try_from_tiles(sd_tiles, hd_tiles)?)
    }

    pub fn load_from_tile_grids<P: AsRef<Path>>(sd_grid_path: P, hd_grid_path: P) -> Result<Self, LoadFromTileGridsError> {
        let sd_tiles = TileGrid::load_from_image(sd_grid_path)?.to_vec();
        let hd_tiles = TileGrid::load_from_image(hd_grid_path)?.to_vec();
        Ok(Self::try_from_tiles(sd_tiles, hd_tiles)?)
    }

    pub fn into_symbol_set(self, specs: &SymbolSpecs) -> Result<SymbolSet, TileKindError> {
        Ok(SymbolSet {
            sd_symbols: self.sd_tiles.to_symbols(specs)?,
            hd_symbols: self.hd_tiles.to_symbols(specs)?
        })
    }

    pub fn save_to_bin_files<P: AsRef<Path>>(&self, sd_path: P, sd_2_path: P, hd_path: P, hd_2_path: P) -> Result<(), SaveTilesToBinFileError> {
        self.sd_tiles.save_to_bin_files(sd_path, sd_2_path)?;
        self.hd_tiles.save_to_bin_files(hd_path, hd_2_path)
    }

    pub fn save_to_bin_files_norm<P: AsRef<Path>>(&self, dir: P, ident: &Option<&str>) -> Result<(), SaveTilesToBinFileError> {
        self.sd_tiles.save_to_bin_files_norm(&dir, ident)?;
        self.hd_tiles.save_to_bin_files_norm(&dir, ident)
    }

    pub fn save_to_grids<P: AsRef<Path>>(&self, sd_path: P, hd_path: P) -> Result<(), SaveGridImageError> {
        self.sd_tiles.save_to_grid_image(sd_path)?;
        self.hd_tiles.save_to_grid_image(hd_path)
    }

    pub fn save_to_grids_norm<P: AsRef<Path>>(&self, dir: P, ident: &Option<&str>) -> Result<(), SaveGridImageError> {
        self.sd_tiles.save_to_grid_image_norm(&dir, ident)?;
        self.hd_tiles.save_to_grid_image_norm(&dir, ident)
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

impl From<SymbolSet> for TileSet {
    fn from(symbol_set: SymbolSet) -> Self {
        Self {
            sd_tiles: symbol_set.sd_symbols.into_tiles_vec(),
            hd_tiles: symbol_set.hd_symbols.into_tiles_vec()
        }
    }
}
