
use derive_more::{Error, Display, From};
use image::ImageError;
use std::path::{Path, PathBuf};

use crate::{osd::tile::Tile, create_path::{create_path, CreatePathError}};


#[derive(Debug, Error, Display, From)]
pub enum SaveTilesToDirError {
    CreatePathError(CreatePathError),
    ImageError(ImageError),
}

pub trait SaveTilesToDir {
    fn save_tiles_to_dir<P: AsRef<Path>>(&self, path: P) -> Result<(), SaveTilesToDirError>;
}

impl<T> SaveTilesToDir for T
where
    for<'any> &'any T: IntoIterator<Item = &'any Tile>,
{
    fn save_tiles_to_dir<P: AsRef<Path>>(&self, path: P) -> Result<(), SaveTilesToDirError> {
        create_path(&path)?;

        for (index, tile) in self.into_iter().enumerate() {
            let path: PathBuf = [path.as_ref(), Path::new(&format!("{:03}.png", index))].iter().collect();
            tile.save(path)?;
        }

        Ok(())
    }
}