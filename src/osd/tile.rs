
pub mod grid;
pub mod container;

use std::path::{Path, PathBuf};

use derive_more::{Deref,DerefMut, From};
use getset::{Getters, CopyGetters};
use strum::{EnumIter,IntoEnumIterator, Display};
use image::{ImageBuffer, Rgba, GenericImageView, GenericImage};
use thiserror::Error;

use crate::{
    dimensions,
    file::Error as FileError,
    image::{
        read_image_file,
        ReadError as ImageReadError,
    }
};

use super::bin_file::BinFileReader;


pub type Dimensions = dimensions::Dimensions<u32>;

pub const SD_DIMENSIONS: Dimensions = Dimensions::new(36, 54);
pub const HD_DIMENSIONS: Dimensions = Dimensions::new(24, 36);

#[derive(Debug, Error, Getters)]
#[getset(get = "pub")]
#[error("dimensions do not match any known tile kind: {dimensions}")]
pub struct InvalidDimensionsError { dimensions: Dimensions }

#[derive(Debug, Error)]
#[error("number of RGBA bytes does not match any tile kind: {0}B")]
pub struct InvalidSizeError(pub u64);

#[derive(Debug, Error)]
#[error("height does not match any tile kind: {0}")]
pub struct InvalidHeightError(pub u32);

#[derive(Debug, Copy, Clone, EnumIter, PartialEq, Eq, Display)]
pub enum Kind {
    SD,
    HD
}

impl Kind {

    pub const fn dimensions(&self) -> Dimensions {
        match self {
            Kind::SD => SD_DIMENSIONS,
            Kind::HD => HD_DIMENSIONS,
        }
    }

    pub const fn set_dir_name(&self) -> &'static str {
        match self {
            Kind::SD => "SD",
            Kind::HD => "HD",
        }
    }

    pub fn set_dir_path<P: AsRef<Path>>(&self, base_dir: P) -> PathBuf {
        [base_dir.as_ref(), Path::new(self.set_dir_name())].iter().collect()
    }

    pub const fn raw_rgba_size_bytes(&self) -> usize {
        let Dimensions { width, height } = self.dimensions();
        width as usize * height as usize * 4
    }

    pub fn for_size_bytes(bytes: u64) -> Result<Self, InvalidSizeError> {
        for kind in Self::iter() {
            if bytes == kind.raw_rgba_size_bytes() as u64 {
                return Ok(kind);
            }
        }
        Err(InvalidSizeError(bytes))
    }

    pub fn for_height(height: u32) -> Result<Self, InvalidHeightError> {
        for kind in Self::iter() {
            if height == kind.dimensions().height {
                return Ok(kind);
            }
        }
        Err(InvalidHeightError(height))
    }


}

impl TryFrom<Dimensions> for Kind {
    type Error = InvalidDimensionsError;

    fn try_from(dimensions: Dimensions) -> Result<Self, Self::Error> {
        match dimensions {
            SD_DIMENSIONS => Ok(Self::SD),
            HD_DIMENSIONS => Ok(Self::HD),
            _ => Err(InvalidDimensionsError { dimensions })
        }
    }
}

#[derive(Debug, From, Error)]
pub enum LoadError {
    #[error(transparent)]
    FileError(FileError),
    #[error(transparent)]
    ImageReadError(ImageReadError),
    #[error("invalid tile image size in file {file_path}: {dimensions}")]
    InvalidDimensionsError { file_path: PathBuf, dimensions: Dimensions },
}

impl LoadError {
    pub fn invalid_dimensions<P: AsRef<Path>>(file_path: P, dimensions: Dimensions) -> Self {
        Self::InvalidDimensionsError { file_path: file_path.as_ref().to_path_buf(), dimensions }
    }
}

pub type Bytes = Vec<u8>;
pub type Image = ImageBuffer<Rgba<u8>, Vec<u8>>;

#[derive(Deref, DerefMut, Clone, Debug, Getters, CopyGetters)]
pub struct Tile {
    #[getset(get_copy = "pub")]
    kind: Kind,

    #[deref]
    #[deref_mut]
    #[getset(get = "pub")]
    image: Image,
}

impl Tile {

    pub fn new(kind: Kind) -> Self {
        let Dimensions { width, height } = kind.dimensions();
        Self { kind, image: ImageBuffer::new(width, height)}
    }

    pub fn load_image_file<P: AsRef<Path>>(path: P) -> Result<Self, LoadError> {
        let image = read_image_file(&path)?;
        let kind = Kind::try_from(Dimensions::from(image.dimensions()))
            .map_err(|error| {
                let InvalidDimensionsError { dimensions } = error;
                LoadError::invalid_dimensions(&path, dimensions)
            })?;
        Ok(Self { kind, image: image.into_rgba8() })
    }

    pub fn read_from_bin_file(file: &mut BinFileReader) -> Result<Self, LoadError> {
        Ok(Self::try_from(file.read_tile_bytes()?).expect("did not read the right number of bytes"))
    }

}

impl TryFrom<Bytes> for Tile {
    type Error = InvalidSizeError;

    fn try_from(bytes: Bytes) -> Result<Self, Self::Error> {
        let kind = Kind::for_size_bytes(bytes.len() as u64)?;
        Ok(Self { kind, image: ImageBuffer::from_raw(kind.dimensions().width(), kind.dimensions().height(), bytes).unwrap() })
    }
}

impl TryFrom<Image> for Tile {
    type Error = InvalidDimensionsError;

    fn try_from(sub_image: Image) -> Result<Self, Self::Error> {
        let (width, height) = sub_image.dimensions();
        let kind = Kind::try_from(Dimensions { width, height })?;
        let mut tile = Self::new(kind);
        tile.image.copy_from(&sub_image, 0, 0).unwrap();
        Ok(tile)
    }
}

impl TryFrom<&mut BinFileReader> for Tile {
    type Error = LoadError;

    fn try_from(file: &mut BinFileReader) -> Result<Self, Self::Error> {
        Self::read_from_bin_file(file)
    }
}