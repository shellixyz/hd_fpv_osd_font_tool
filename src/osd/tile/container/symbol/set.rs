
use std::ops::Index;
use std::path::Path;

use derive_more::{From, Display, Error};
use getset::Getters;
use strum::IntoEnumIterator;

use crate::osd::tile::Kind as TileKind;
use crate::osd::tile::container::load_symbols_from_dir::{load_symbols_from_dir, LoadSymbolsFromDirError};
use crate::osd::tile::container::save_symbols_to_dir::SaveSymbolsToDirError;
use crate::osd::tile::container::uniq_tile_kind::{UniqTileKind, TileKindError};
use crate::prelude::SaveSymbolsToDir;
use super::Symbol;


#[derive(Debug, Error, Display, From)]
pub enum LoadFromDirError {
    LoadSymbolsFromDirError(LoadSymbolsFromDirError),
    TileKindError(TileKindError),
}

#[derive(Getters)]
#[getset(get = "pub")]
pub struct Set {
    pub(crate) sd_symbols: Vec<Symbol>,
    pub(crate) hd_symbols: Vec<Symbol>,
}

impl Set {

    fn check_collection_kind(tiles: &[Symbol], expected_tile_kind: TileKind) -> Result<(), TileKindError> {
        let tile_kind = tiles.tile_kind()?;
        if tile_kind != expected_tile_kind {
            return Err(TileKindError::LoadedDoesNotMatchRequested { requested: expected_tile_kind, loaded: tile_kind })
        }
        Ok(())
    }

    pub fn try_from_symbols(sd_symbols: Vec<Symbol>, hd_symbols: Vec<Symbol>) -> Result<Self, TileKindError> {
        Self::check_collection_kind(&sd_symbols, TileKind::SD)?;
        Self::check_collection_kind(&hd_symbols, TileKind::HD)?;
        Ok(Self { sd_symbols, hd_symbols })
    }

    pub fn save_to_dir<P: AsRef<Path>>(&self, dir: P) -> Result<(), SaveSymbolsToDirError> {
        for tile_kind in TileKind::iter() {
            self[tile_kind].save_to_dir(tile_kind.set_dir_path(&dir))?;
        }
        Ok(())
    }

    pub fn load_from_dir<P: AsRef<Path>>(dir_path: P, max_symbols: usize) -> Result<Self, LoadFromDirError> {
        let sd_symbols = load_symbols_from_dir(TileKind::SD.set_dir_path(&dir_path), max_symbols)?;
        let hd_symbols = load_symbols_from_dir(TileKind::HD.set_dir_path(&dir_path), max_symbols)?;
        Ok(Self::try_from_symbols(sd_symbols, hd_symbols)?)
    }

}

impl Index<TileKind> for Set {
    type Output = Vec<Symbol>;

    fn index(&self, tile_kind: TileKind) -> &Self::Output {
        match tile_kind {
            TileKind::SD => &self.sd_symbols,
            TileKind::HD => &self.hd_symbols,
        }
    }
}
