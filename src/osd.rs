pub mod tile;
pub mod bin_file;

use std::path::{Path, PathBuf};
use std::io::Error as IOError;

use image::ImageError;

use self::tile::containers::{ExtendedSizeContainer, StandardTileContainer};
use self::{tile::{Tile, containers::StandardSizeContainer}, bin_file::BinFileWriter};

pub trait SaveTilesToDir {
    fn save_tiles_to_dir<P: AsRef<Path>>(&self, path: P) -> Result<(), ImageError>;
}

impl<T> SaveTilesToDir for T
where
    for<'any> &'any T: IntoIterator<Item = &'any Tile> + StandardTileContainer,
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

pub trait SaveToBinFiles {
    fn save_to_bin_files<P: AsRef<Path>>(&self, path1: P, path2: P) -> Result<(), IOError>;
}

impl<T> SaveToBinFiles for T
where
    for<'any> &'any T: ExtendedSizeContainer
{
    fn save_to_bin_files<P: AsRef<Path>>(&self, path1: P, path2: P) -> Result<(), IOError> {
        self.first_half().save_tiles_to_bin_file(path1)?;
        self.second_half().save_tiles_to_bin_file(path2)
    }
}