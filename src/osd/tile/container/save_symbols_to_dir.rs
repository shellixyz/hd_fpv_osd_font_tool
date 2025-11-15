use derive_more::{Display, Error, From};
use std::path::{Path, PathBuf};

use super::symbol::Symbol;

use crate::create_path::{CreatePathError, create_path};
use crate::image::{WriteError as ImageWriteError, WriteImageFile};

#[derive(Debug, Error, Display, From)]
pub enum SaveSymbolsToDirError {
	CreatePathError(CreatePathError),
	ImageWriteError(ImageWriteError),
}

pub trait SaveSymbolsToDir {
	/// Saves symbols to the provided directory
	///
	/// # Errors
	/// Returns `SaveSymbolsToDirError` if saving fails
	fn save_to_dir<P: AsRef<Path>>(&self, path: P) -> Result<(), SaveSymbolsToDirError>;
}

impl<T> SaveSymbolsToDir for T
where
	for<'any> &'any T: IntoIterator<Item = &'any Symbol>,
{
	fn save_to_dir<P: AsRef<Path>>(&self, path: P) -> Result<(), SaveSymbolsToDirError> {
		create_path(&path)?;
		let mut tile_index = 0;
		for symbol in self {
			let file_name = match symbol.span() {
				1 => format!("{tile_index:03}.png"),
				span => format!("{tile_index:03}-{:03}.png", tile_index + span - 1),
			};
			let file_path: PathBuf = [path.as_ref(), Path::new(&file_name)].iter().collect();
			symbol.generate_image().write_image_file(file_path)?;
			tile_index += symbol.span();
		}
		Ok(())
	}
}
