use derive_more::{Display, Error, From};
use image::ImageError;
use std::path::{Path, PathBuf};

use crate::{
	create_path::{CreatePathError, create_path},
	osd::tile::Tile,
};

#[derive(Debug, Error, Display, From)]
pub enum SaveTilesToDirError {
	CreatePathError(CreatePathError),
	ImageError(ImageError),
}

pub trait SaveTilesToDir {
	/// Saves tiles to the provided directory
	///
	/// # Errors
	/// Returns `SaveTilesToDirError` if saving fails
	fn save_tiles_to_dir<P: AsRef<Path>>(&self, path: P) -> Result<(), SaveTilesToDirError>;
}

impl<T> SaveTilesToDir for T
where
	for<'any> &'any T: IntoIterator<Item = &'any Tile>,
{
	fn save_tiles_to_dir<P: AsRef<Path>>(&self, path: P) -> Result<(), SaveTilesToDirError> {
		create_path(&path)?;

		for (index, tile) in self.into_iter().enumerate() {
			let path: PathBuf = [path.as_ref(), Path::new(&format!("{index:03}.png"))].iter().collect();
			tile.save(path)?;
		}

		Ok(())
	}
}
