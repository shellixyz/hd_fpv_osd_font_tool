use std::path::Path;

use crate::{
	osd::tile::{Tile, grid::SaveImageError as SaveGridImageError},
	prelude::IntoTileGrid,
};

pub trait SaveToGridImage {
	/// Saves tiles to a grid image file
	///
	/// # Errors
	/// Returns `SaveGridImageError` if saving fails
	fn save_to_grid_image<P: AsRef<Path>>(&self, path: P) -> Result<(), SaveGridImageError>;

	/// Saves tiles to a grid image file with normalized naming
	///
	/// # Errors
	/// Returns `SaveGridImageError` if saving fails
	fn save_to_grid_image_norm<P: AsRef<Path>>(&self, dir: P, ident: Option<&str>) -> Result<(), SaveGridImageError>;
}

impl SaveToGridImage for Vec<Tile> {
	fn save_to_grid_image<P: AsRef<Path>>(&self, path: P) -> Result<(), SaveGridImageError> {
		self.into_tile_grid().save_image(path)?;
		Ok(())
	}

	fn save_to_grid_image_norm<P: AsRef<Path>>(&self, dir: P, ident: Option<&str>) -> Result<(), SaveGridImageError> {
		self.into_tile_grid().save_image_norm(dir, ident)
	}
}

impl SaveToGridImage for &[Tile] {
	fn save_to_grid_image<P: AsRef<Path>>(&self, path: P) -> Result<(), SaveGridImageError> {
		self.to_vec().save_to_grid_image(path)
	}

	fn save_to_grid_image_norm<P: AsRef<Path>>(&self, dir: P, ident: Option<&str>) -> Result<(), SaveGridImageError> {
		self.to_vec().save_to_grid_image_norm(dir, ident)
	}
}
