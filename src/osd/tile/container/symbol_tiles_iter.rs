use super::symbol::Symbol;
use crate::osd::tile::Tile;

pub struct SymbolTilesIter<'a> {
	symbols: &'a [Symbol],
	symbol_index: usize,
	symbol_tile_index: usize,
}

impl<'a> SymbolTilesIter<'a> {
	pub fn new(symbols: &'a [Symbol]) -> Self {
		Self {
			symbols,
			symbol_index: 0,
			symbol_tile_index: 0,
		}
	}
}

impl<'a> Iterator for SymbolTilesIter<'a> {
	type Item = &'a Tile;

	fn next(&mut self) -> Option<Self::Item> {
		if self.symbol_index == self.symbols.len() {
			return None;
		}
		let symbol_tiles = self.symbols[self.symbol_index].tiles();
		let tile = &symbol_tiles[self.symbol_tile_index];
		if self.symbol_tile_index == symbol_tiles.len() - 1 {
			self.symbol_tile_index = 0;
			self.symbol_index += 1;
		} else {
			self.symbol_tile_index += 1;
		}
		Some(tile)
	}
}

pub trait IntoSymbolsTilesIter {
	fn tiles_iter(&'_ self) -> SymbolTilesIter<'_>;
}

impl IntoSymbolsTilesIter for &[Symbol] {
	fn tiles_iter(&'_ self) -> SymbolTilesIter<'_> {
		SymbolTilesIter::new(self)
	}
}
