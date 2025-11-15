use thiserror::Error;

use super::{IntoSymbolsTilesIter, symbol::Symbol};
use crate::osd::tile::{Kind as TileKind, Tile};

#[derive(Debug, Error)]
pub enum TileKindError {
	#[error("cannot determine tile kind from empty container")]
	EmptyContainer,
	#[error("container includes multiple tile kinds")]
	MultipleTileKinds,
	#[error("loaded kind does not match requested: loaded {loaded}, requested {requested}")]
	LoadedDoesNotMatchRequested { requested: TileKind, loaded: TileKind },
}

pub trait TilesIterUniqTileKind {
	/// Determines the unique tile kind of the tiles in the iterator
	///
	/// # Errors
	/// Returns `TileKindError::EmptyContainer` if the iterator is empty
	/// Returns `TileKindError::MultipleTileKinds` if multiple tile kinds are found
	fn tile_kind(&mut self) -> Result<TileKind, TileKindError>;
}

impl<'a, T> TilesIterUniqTileKind for T
where
	T: Iterator<Item = &'a Tile>,
{
	fn tile_kind(&mut self) -> Result<TileKind, TileKindError> {
		let first_tile_kind = self.next().ok_or(TileKindError::EmptyContainer)?.kind();
		if !self.all(|tile| tile.kind() == first_tile_kind) {
			return Err(TileKindError::MultipleTileKinds);
		}
		Ok(first_tile_kind)
	}
}

pub trait SymbolsIterUniqTileKind {
	/// Determines the unique tile kind of the symbols in the iterator
	///
	/// # Errors
	/// Returns `TileKindError::EmptyContainer` if the iterator is empty
	/// Returns `TileKindError::MultipleTileKinds` if multiple tile kinds are found
	fn tile_kind(&mut self) -> Result<TileKind, TileKindError>;
}

impl<'a, B> SymbolsIterUniqTileKind for B
where
	B: Iterator<Item = &'a Symbol>,
{
	fn tile_kind(&mut self) -> Result<TileKind, TileKindError> {
		let first_tile_kind = self.next().ok_or(TileKindError::EmptyContainer)?.tile_kind();
		if !self.all(|symbol| symbol.tile_kind() == first_tile_kind) {
			return Err(TileKindError::MultipleTileKinds);
		}
		Ok(first_tile_kind)
	}
}

pub trait UniqTileKind {
	/// Determines the unique tile kind of the container
	///
	/// # Errors
	/// Returns `TileKindError::EmptyContainer` if the container is empty
	/// Returns `TileKindError::MultipleTileKinds` if multiple tile kinds are found
	fn tile_kind(&self) -> Result<TileKind, TileKindError>;
}

impl UniqTileKind for &[Tile] {
	fn tile_kind(&self) -> Result<TileKind, TileKindError> {
		self.iter().tile_kind()
	}
}

impl UniqTileKind for Vec<Tile> {
	fn tile_kind(&self) -> Result<TileKind, TileKindError> {
		self.as_slice().tile_kind()
	}
}

impl UniqTileKind for &[Symbol] {
	fn tile_kind(&self) -> Result<TileKind, TileKindError> {
		self.tiles_iter().tile_kind()
	}
}
impl UniqTileKind for Vec<Symbol> {
	fn tile_kind(&self) -> Result<TileKind, TileKindError> {
		self.as_slice().tile_kind()
	}
}
