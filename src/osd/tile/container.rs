
pub mod symbol;
pub mod save_tiles_to_dir;
pub mod tile_set;
pub mod uniq_tile_kind;
pub mod save_to_bin_file;
pub mod into_tile_grid;
pub mod load_tiles_from_dir;
pub mod save_symbols_to_dir;
pub mod symbol_tiles_iter;
pub mod load_symbols_from_dir;
pub mod save_to_grid;

use tap::Tap;

use symbol::{
    Symbol,
    spec::Specs as SymbolSpecs
};

use symbol_tiles_iter::IntoSymbolsTilesIter;
use uniq_tile_kind::{TileKindError, UniqTileKind};

use super::Tile;


pub trait IntoTilesVec {
    fn into_tiles_vec(self) -> Vec<Tile>;
}

impl IntoTilesVec for Vec<Symbol> {
    fn into_tiles_vec(self) -> Vec<Tile> {
        self.into_iter().flat_map(Symbol::into_tiles).collect()
    }
}

pub trait AsTilesVec<'a> {
    fn as_tiles_vec(&'a self) -> Vec<&'a Tile>;
}

impl<'a> AsTilesVec<'a> for &[Symbol] {
    fn as_tiles_vec(&'a self) -> Vec<&'a Tile> {
        self.tiles_iter().collect()
    }
}

pub trait ToSymbols {
    fn to_symbols(&self, specs: &SymbolSpecs) -> Result<Vec<Symbol>, TileKindError>;
}

impl ToSymbols for &[Tile] {
    fn to_symbols(&self, specs: &SymbolSpecs) -> Result<Vec<Symbol>, TileKindError> {
        let mut tile_index = 0;
        let mut symbols = vec![];
        while tile_index < self.len() {
            let symbol = match specs.find_start_index(tile_index) {
                Some(sym_spec) =>
                    Symbol::try_from(Vec::from(&self[sym_spec.tile_index_range()]))?
                        .tap(|_| tile_index += sym_spec.span()),
                None =>
                    Symbol::from(self[tile_index].clone())
                        .tap(|_| tile_index += 1),
            };
            symbols.push(symbol);
        }
        Ok(symbols)
    }
}

impl ToSymbols for Vec<Tile> {
    fn to_symbols(&self, specs: &SymbolSpecs) -> Result<Vec<Symbol>, TileKindError> {
        self.as_slice().to_symbols(specs)
    }
}
