
use derive_more::{Error, Display, From};
use image::ImageError;
use std::{io::Error as IOError, path::{Path, PathBuf}};

use super::symbol::Symbol;


#[derive(Debug, Error, Display, From)]
pub enum SaveSymbolsToDirError {
    IOError(IOError),
    ImageError(ImageError),
}

pub trait SaveSymbolsToDir {
    fn save_to_dir<P: AsRef<Path>>(&self, path: P) -> Result<(), SaveSymbolsToDirError>;
}

impl<T> SaveSymbolsToDir for T
where
    for<'any> &'any T: IntoIterator<Item = &'any Symbol>,
{
    fn save_to_dir<P: AsRef<Path>>(&self, path: P) -> Result<(), SaveSymbolsToDirError> {
        std::fs::create_dir_all(&path)?;
        let mut tile_index = 0;
        for symbol in self {
            let file_name = match symbol.span() {
                1 => format!("{tile_index:03}.png"),
                span => format!("{tile_index:03}-{:03}.png", tile_index + span - 1)
            };
            let file_path: PathBuf = [path.as_ref(), Path::new(&file_name)].iter().collect();
            symbol.generate_image().save(file_path)?;
            tile_index += symbol.span();
        }
        Ok(())
    }
}
