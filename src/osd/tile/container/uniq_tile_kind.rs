
use std::fmt::Display;

use derive_more::Error;
use crate::osd::tile::{Kind as TileKind, Tile};

use super::{symbol::Symbol, IntoSymbolsTilesIter};


#[derive(Debug, Error)]
pub enum TileKindError {
    EmptyContainer,
    MultipleTileKinds,
    LoadedDoesNotMatchRequested {
        requested: TileKind,
        loaded: TileKind,
    }
}

impl Display for TileKindError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use TileKindError::*;
        match self {
            EmptyContainer => f.write_str("cannot determine tile kind from empty container"),
            MultipleTileKinds => f.write_str("container includes multiple tile kinds"),
            LoadedDoesNotMatchRequested { requested, loaded } => write!(f, "loaded kind does not match requested: loaded {loaded}, requested {requested}"),
        }
    }
}

pub trait IterUniqTileKind {
    fn tile_kind(&mut self) -> Result<TileKind, TileKindError>;
}

impl<'a, T> IterUniqTileKind for T
where
    T: Iterator<Item = &'a Tile>
{
    fn tile_kind(&mut self) -> Result<TileKind, TileKindError> {
        let first_tile_kind = self.next().ok_or(TileKindError::EmptyContainer)?.kind();
        if ! self.all(|tile| tile.kind() == first_tile_kind) {
            return Err(TileKindError::MultipleTileKinds)
        }
        Ok(first_tile_kind)
    }
}

pub trait UniqTileKind {
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
