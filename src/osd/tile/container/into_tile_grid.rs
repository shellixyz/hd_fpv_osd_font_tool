
use crate::osd::tile::{grid::Grid as TileGrid, Tile};

pub trait IntoTileGrid {
    fn into_tile_grid(self) -> TileGrid;
}

impl IntoTileGrid for &[Tile] {
    fn into_tile_grid(self) -> TileGrid {
        TileGrid::from(self)
    }
}
