
use std::error::Error;
use std::fmt::Display;
use std::path::Path;
use std::io::Error as IOError;

use derive_more::{Deref,DerefMut};
use strum::{EnumIter,IntoEnumIterator, Display};
use image::{ImageBuffer, Rgba, GenericImageView, GenericImage};
use image::io::Reader as ImageReader;
use image::error::ImageError;
use super::bin_file::BinFileReader;

pub mod grid;
pub mod containers;

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Dimensions {
    pub width: u32,
    pub height: u32
}

impl From<(u32, u32)> for Dimensions {
    fn from((width, height): (u32, u32)) -> Self {
        Self { width, height }
    }
}

pub const SD_DIMENSIONS: Dimensions = Dimensions { width: 36, height: 54 };
pub const HD_DIMENSIONS: Dimensions = Dimensions { width: 24, height: 36 };

#[derive(Debug)]
pub struct InvalidDimensionsError(Dimensions);
impl Error for InvalidDimensionsError {}

impl Display for InvalidDimensionsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "dimensions do not match any tile kind: {}x{}", self.0.width, self.0.height)
    }
}

#[derive(Debug)]
pub struct InvalidSizeError(pub(crate) usize);
impl Error for InvalidSizeError {}

impl Display for InvalidSizeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "size bytes do not match any tile kind: {}B", self.0)
    }
}

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

    pub const fn raw_rgba_size_bytes(&self) -> usize {
        let Dimensions { width, height } = self.dimensions();
        width as usize * height as usize * 4
    }

    pub fn for_size_bytes(bytes: usize) -> Result<Self, InvalidSizeError> {
        for kind in Self::iter() {
            if bytes == kind.raw_rgba_size_bytes() {
                return Ok(kind);
            }
        }
        Err(InvalidSizeError(bytes))
    }

}

impl TryFrom<Dimensions> for Kind {
    type Error = InvalidDimensionsError;

    fn try_from(dimensions: Dimensions) -> Result<Self, Self::Error> {
        match dimensions {
            SD_DIMENSIONS => Ok(Self::SD),
            HD_DIMENSIONS => Ok(Self::HD),
            _ => Err(InvalidDimensionsError(dimensions))
        }
    }
}

#[derive(Debug)]
pub enum LoadError {
    IOError(IOError),
    ImageError(ImageError),
    InvalidDimensionsError(Dimensions),
    InvalidSizeError(usize)
}

impl Display for LoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use LoadError::*;
        match self {
            IOError(io_error) => io_error.fmt(f),
            ImageError(image_error) => image_error.fmt(f),
            InvalidDimensionsError(dimensions) => write!(f, "invalid tile image size {}x{}", dimensions.width, dimensions.height),
            InvalidSizeError(error) => error.fmt(f),
        }
    }
}

impl From<IOError> for LoadError {
    fn from(error: IOError) -> Self {
        Self::IOError(error)
    }
}

impl From<ImageError> for LoadError {
    fn from(error: ImageError) -> Self {
        Self::ImageError(error)
    }
}

impl From<InvalidDimensionsError> for LoadError {
    fn from(error: InvalidDimensionsError) -> Self {
        Self::InvalidDimensionsError(error.0)
    }
}

impl From<InvalidSizeError> for LoadError {
    fn from(error: InvalidSizeError) -> Self {
        Self::InvalidSizeError(error.0)
    }
}


impl Error for LoadError {}

pub type Bytes = Vec<u8>;
pub type Image = ImageBuffer<Rgba<u8>, Vec<u8>>;

#[derive(Deref,DerefMut,Clone)]
pub struct Tile {
    kind: Kind,

    #[deref]
    #[deref_mut]
    image: Image,
}

impl Tile {

    pub fn new(kind: Kind) -> Self {
        let Dimensions { width, height } = kind.dimensions();
        Self { kind, image: ImageBuffer::new(width, height)}
    }

    pub fn load_image_file<P: AsRef<Path>>(path: P) -> Result<Self, LoadError> {
        let image = ImageReader::open(path)?.decode()?;
        let kind = Kind::try_from(Dimensions::from(image.dimensions()))?;
        Ok(Self { kind, image: image.into_rgba8() })
    }

    pub fn read_from_bin_file(file: &mut BinFileReader) -> Result<Self, LoadError> {
        Ok(Self::try_from(file.read_tile_bytes()?)?)
    }

    pub fn kind(&self) -> Kind {
        self.kind
    }

    pub fn image(&self) -> &Image {
        &self.image
    }

}

impl TryFrom<Bytes> for Tile {
    type Error = InvalidSizeError;

    fn try_from(bytes: Bytes) -> Result<Self, Self::Error> {
        let kind = Kind::for_size_bytes(bytes.len())?;
        Ok(Self { kind, image: ImageBuffer::from_raw(kind.dimensions().width, kind.dimensions().height, bytes).unwrap() })
    }
}

impl TryFrom<ImageBuffer<Rgba<u8>, Vec<u8>>> for Tile {
    type Error = InvalidDimensionsError;

    fn try_from(sub_image: ImageBuffer<Rgba<u8>, Vec<u8>>) -> Result<Self, Self::Error> {
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

pub struct TileIter<'a, T> {
    pub(crate) container: &'a T,
    pub(crate) index: usize
}

// pub struct TileIntoIter<T> {
//     pub(crate) container: T,
//     pub(crate) index: usize
// }