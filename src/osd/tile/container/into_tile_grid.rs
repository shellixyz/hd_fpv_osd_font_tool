use crate::osd::tile::{Tile, grid::Grid as TileGrid};

pub trait IntoTileGrid {
	fn into_tile_grid(self) -> TileGrid;
}

impl IntoTileGrid for &[Tile] {
	fn into_tile_grid(self) -> TileGrid {
		TileGrid::from(self)
	}
}
