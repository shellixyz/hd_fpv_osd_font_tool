
use std::error::Error;
use std::fmt::Display;
use std::ops::Index;
use std::path::Path;
use std::io::Error as IOError;

use image::{ImageBuffer, Rgba, GenericImage, ImageError, GenericImageView};
use image::io::Reader as ImageReader;
use strum::IntoEnumIterator;

use super::containers::{GetTileKind, ReplaceTile, ExtendedSizeArray};
use super::{Tile, TileIter};
// use crate::osd::standard_size_tile_container::{self, StandardSizeTileArray, StandardSizeTileContainer};
use crate::osd::bin_file::{BinFileReader, SeekReadError as BinFileSeekReadError};
use crate::osd::tile::{self, containers::{self, StandardSizeArray, StandardSizeContainer}};

#[derive(Debug)]
pub struct InvalidImageDimensionsError;
impl Error for InvalidImageDimensionsError {}

impl Display for InvalidImageDimensionsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("image dimensions does not match valid dimensions for any of the tile kinds")
    }
}

#[derive(Debug)]
pub enum LoadError {
    IOError(IOError),
    ImageError(ImageError),
    InvalidImageDimensions(InvalidImageDimensionsError)
}

impl Error for LoadError {}

impl Display for LoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use LoadError::*;
        match self {
            IOError(io_error) => io_error.fmt(f),
            ImageError(image_error) => image_error.fmt(f),
            InvalidImageDimensions(error) => error.fmt(f),
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

impl From<InvalidImageDimensionsError> for LoadError {
    fn from(error: InvalidImageDimensionsError) -> Self {
        Self::InvalidImageDimensions(error)
    }
}

#[derive(PartialEq, Eq)]
pub struct ImageDimensions {
    width: u32,
    height: u32
}

struct Dimensions {
    width: usize,
    height: usize
}

const DIMENSIONS: Dimensions = Dimensions { width: 16, height: 16 };
const SEPARATOR_THICKNESS: u32 = 2;

impl tile::Kind {

    pub fn for_grid_image_dimensions(image_dimensions: ImageDimensions) -> Result<Self, InvalidImageDimensionsError> {
        for kind in Self::iter() {
            let image_dimensions_for_kind = ImageDimensions {
                width: (DIMENSIONS.width as u32 - 1) * SEPARATOR_THICKNESS + DIMENSIONS.width as u32 * kind.dimensions().width,
                height: (DIMENSIONS.height as u32 - 1) * SEPARATOR_THICKNESS + DIMENSIONS.height as u32 * kind.dimensions().height
            };
            if image_dimensions == image_dimensions_for_kind {
                return Ok(kind);
            }
        }
        Err(InvalidImageDimensionsError)
    }

}

pub type Image = ImageBuffer<Rgba<u8>, Vec<u8>>;

macro_rules! grid {
    ($type_name:ident, $container:ty, $container_size:expr) => {

        pub struct $type_name($container);

        impl $type_name {

            pub fn new(tile_kind: tile::Kind) -> Self {
                Self(<$container>::new(tile_kind))
            }

            pub fn index_linear_to_grid(index: usize) -> (usize, usize) {
                assert!(index < $container_size);
                (index % DIMENSIONS.width, index / DIMENSIONS.width)
            }

            fn grid_coordinates_to_index(x: usize, y: usize) -> usize {
                assert!(x < DIMENSIONS.width && y < DIMENSIONS.height);
                x + y * DIMENSIONS.width
            }

            pub fn tile_kind(&self) -> tile::Kind {
                self.0.tile_kind()
            }

            pub fn replace_tile(&mut self, x: usize, y: usize, tile: Tile) -> Result<(), containers::TileKindMismatchError> {
                self.0.replace_tile(Self::grid_coordinates_to_index(x, y), tile)
            }

            fn image_tile_position(tile_kind: &tile::Kind, x: u32, y: u32) -> (u32, u32) {
                let tile_dimensions = tile_kind.dimensions();
                (
                    x as u32 * (SEPARATOR_THICKNESS + tile_dimensions.width),
                    y as u32 * (SEPARATOR_THICKNESS + tile_dimensions.height)
                )
            }

            pub fn load_from_image<P: AsRef<Path> + Display>(path: P) -> Result<Self, LoadError> {
                let image = ImageReader::open(&path)?.decode()?;
                let (img_dim_width, img_dim_height) = image.dimensions();
                let tile_kind = tile::Kind::for_grid_image_dimensions(ImageDimensions { width: img_dim_width, height: img_dim_height })?;
                log::info!("detected {} kind of tiles in {}", tile_kind, path);
                let tile_dimensions = tile_kind.dimensions();
                let mut grid = Self::new(tile_kind);

                for y in 0..DIMENSIONS.height {
                    for x in 0..DIMENSIONS.width {
                        let (tile_pos_x, tile_pos_y) = Self::image_tile_position(&tile_kind, x as u32, y as u32);
                        let sub_image = image.view(tile_pos_x, tile_pos_y, tile_dimensions.width, tile_dimensions.height).to_image();
                        // let tile = Tile::try_from(sub_image).unwrap(); // "safer" but slower
                        // grid.replace_tile(x, y, tile).unwrap();
                        grid.0.0[Self::grid_coordinates_to_index(x, y)].copy_from(&sub_image, 0, 0).unwrap();
                    }
                }

                Ok(grid)
            }

            pub fn image_dimensions(&self) -> ImageDimensions {
                let tile_dimensions = self.tile_kind().dimensions();
                ImageDimensions {
                    width: DIMENSIONS.width as u32 * tile_dimensions.width + (DIMENSIONS.width as u32 - 1) * SEPARATOR_THICKNESS,
                    height: DIMENSIONS.height as u32 * tile_dimensions.height + (DIMENSIONS.height as u32 - 1) * SEPARATOR_THICKNESS
                }
            }

            pub fn image(&self) -> Image {
                let img_dim = self.image_dimensions();
                let mut image = Image::from_pixel(img_dim.width, img_dim.height, Rgba::from([0, 0, 0, 255]));

                let tile_dimensions = self.tile_kind().dimensions();
                for x in 0..DIMENSIONS.width {
                    for y in 0..DIMENSIONS.height {
                        let (tile_x_position, tile_y_position) = Self::image_tile_position(&self.tile_kind(), x as u32, y as u32);
                        let mut sub_image = image.sub_image(tile_x_position, tile_y_position, tile_dimensions.width, tile_dimensions.height);
                        for (pixel_x, pixel_y, pixel) in self[(x, y)].enumerate_pixels() {
                            sub_image.put_pixel(pixel_x, pixel_y, *pixel);
                        }
                    }
                }

                image
            }

            pub fn iter(&self) -> TileIter<Self> {
                self.into_iter()
            }

        }

        impl Index<(usize, usize)> for $type_name {
            type Output = Tile;

            fn index(&self, index: (usize, usize)) -> &Self::Output {
                &self.0[Self::grid_coordinates_to_index(index.0, index.1)]
            }
        }

        impl From<$container> for $type_name {
            fn from(tile_array: $container) -> Self {
                Self(tile_array)
            }
        }

        impl StandardSizeContainer for $type_name {}
        impl StandardSizeContainer for &$type_name {}

        // impl Iterator for TileIntoIter<TileGrid> {
        //     type Item = Tile;

        //     fn next(&mut self) -> Option<Self::Item> {
        //         if self.index >= $container_size {
        //             return None;
        //         }
        //         let tile = self.container.0[self.index];
        //         self.index += 1;
        //         Some(tile)
        //     }
        // }

        impl<'a> Iterator for TileIter<'a, $type_name> {
            type Item = &'a Tile;

            fn next(&mut self) -> Option<Self::Item> {
                if self.index >= $container_size {
                    return None;
                }
                let tile = &self.container.0[self.index];
                self.index += 1;
                Some(tile)
            }
        }

        // impl IntoIterator for TileGrid {
        //     type Item = Tile;

        //     type IntoIter = TileIntoIter<Self>;

        //     fn into_iter(self) -> Self::IntoIter {
        //         Self::IntoIter { container: self, index: 0 }
        //     }
        // }

        impl<'a> IntoIterator for &'a $type_name {
            type Item = &'a Tile;

            type IntoIter = TileIter<'a, $type_name>;

            fn into_iter(self) -> Self::IntoIter {
                Self::IntoIter { container: self, index: 0 }
            }
        }

    }
}

grid!(StandardSizeGrid, StandardSizeArray, containers::STANDARD_TILE_COUNT);
grid!(ExtendedSizeGrid, ExtendedSizeArray, containers::EXTENDED_TILE_COUNT);

impl From<ExtendedSizeArray> for StandardSizeGrid {
    fn from(tile_array: ExtendedSizeArray) -> Self {
        Self(tile_array.first_half())
    }
}

impl TryFrom<&mut BinFileReader> for StandardSizeGrid {
    type Error = BinFileSeekReadError;

    fn try_from(file: &mut BinFileReader) -> Result<Self, Self::Error> {
        Ok(Self(file.tile_array()?))
    }
}
