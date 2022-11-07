pub mod tile;
pub mod bin_file;

use std::path::{Path, PathBuf};
use std::io::Error as IOError;

use image::ImageError;

use self::{tile::{Tile, containers::StandardSizeContainer}, bin_file::BinFileWriter};

pub trait SaveTilesToDir {
    fn save_tiles_to_dir<P: AsRef<Path>>(&self, path: P) -> Result<(), ImageError>;
}

impl<T> SaveTilesToDir for T
where
    for<'any> &'any T: IntoIterator<Item = &'any Tile> + StandardSizeContainer,
{
    fn save_tiles_to_dir<P: AsRef<Path>>(&self, path: P) -> Result<(), ImageError> {
        std::fs::create_dir_all(&path).unwrap();

        for (index, tile) in self.into_iter().enumerate() {
            let path: PathBuf = [path.as_ref().to_str().unwrap(), &format!("{:03}.png", index)].iter().collect();
            tile.save(path)?;
        }

        Ok(())
    }
}

pub trait SaveTilesToBinFile {
    fn save_tiles_to_bin_file<P: AsRef<Path>>(&self, path: P) -> Result<(), IOError>;
}

impl<T> SaveTilesToBinFile for T
where
    for<'any> &'any T: IntoIterator<Item = &'any Tile> + StandardSizeContainer,
{
    fn save_tiles_to_bin_file<P: AsRef<Path>>(&self, path: P) -> Result<(), IOError> {
        let mut writer = BinFileWriter::create(path)?;

        for tile in self.into_iter() {
            if let Err(write_error) = writer.write_tile(tile) {
                match write_error {
                    bin_file::TileWriteError::IOError(io_error) => return Err(io_error),
                    _ => Err(write_error).expect("should not happen")
                }
            }
        }

        if let Err(write_error) = writer.finish() {
            match write_error {
                bin_file::TileWriteError::IOError(io_error) => return Err(io_error),
                _ => Err(write_error).expect("should not happen")
            }
        }

        Ok(())
    }
}