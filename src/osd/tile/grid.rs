
use std::fmt::Display;
use std::ops::Index;
use std::path::Path;
use std::io::Error as IOError;

use derive_more::{Error, Deref, Display, From, IntoIterator};
use image::{ImageBuffer, Rgba, GenericImage, ImageError, GenericImageView};
use image::io::Reader as ImageReader;
use strum::IntoEnumIterator;

use super::Tile;
use super::container::{UniqTileKind, TileKindError};
use crate::dimensions;
use crate::osd::tile;

#[derive(Debug, Error)]
pub struct InvalidImageDimensionsError;

impl Display for InvalidImageDimensionsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("image dimensions does not match valid dimensions for any of the tile kinds")
    }
}

#[derive(Debug, From, Error, Display)]
pub enum LoadError {
    IOError(IOError),
    ImageError(ImageError),
    InvalidImageDimensions(InvalidImageDimensionsError)
}

pub type ImageDimensions = dimensions::Dimensions<u32>;

const WIDTH: usize = 16;
const SEPARATOR_THICKNESS: u32 = 2;

pub type Image = ImageBuffer<Rgba<u8>, Vec<u8>>;

#[derive(Deref, IntoIterator)]
pub struct Grid(Vec<Tile>);

impl Grid {

    pub fn index_to_grid_coordinates(index: usize) -> (usize, usize) {
        (index % WIDTH, index / WIDTH)
    }

    fn grid_coordinates_to_index(x: usize, y: usize) -> usize {
        assert!(x < WIDTH);
        x + y * WIDTH
    }

    fn image_tile_position(tile_kind: &tile::Kind, x: u32, y: u32) -> (u32, u32) {
        let tile_dimensions = tile_kind.dimensions();
        (
            x * (SEPARATOR_THICKNESS + tile_dimensions.width()),
            y * (SEPARATOR_THICKNESS + tile_dimensions.height())
        )
    }

    pub fn image_tile_kind_and_grid_height(image_dimensions: ImageDimensions) -> Result<(tile::Kind, usize), InvalidImageDimensionsError> {
        for tile_kind in tile::Kind::iter() {
            let expected_width = (WIDTH as u32 - 1) * SEPARATOR_THICKNESS + WIDTH as u32 * tile_kind.dimensions().width;
            if image_dimensions.width == expected_width {
                if (image_dimensions.height - tile_kind.dimensions().height) % (tile_kind.dimensions().height + SEPARATOR_THICKNESS) == 0 {
                    let grid_height = (image_dimensions.height - tile_kind.dimensions().height) / (tile_kind.dimensions().height + SEPARATOR_THICKNESS) + 1;
                    return Ok((tile_kind, grid_height as usize));
                } else {
                    return Err(InvalidImageDimensionsError)
                }
            }
        }
        Err(InvalidImageDimensionsError)
    }

    pub fn load_from_image<P: AsRef<Path> + Display>(path: P) -> Result<Self, LoadError> {
        let image = ImageReader::open(&path)?.decode()?;
        let (img_dim_width, img_dim_height) = image.dimensions();
        let (tile_kind, grid_height) = Self::image_tile_kind_and_grid_height(ImageDimensions { width: img_dim_width, height: img_dim_height })?;
        log::info!("detected {tile_kind} kind of tiles in a {WIDTH}x{grid_height} grid in {path}");
        let tile_dimensions = tile_kind.dimensions();
        let mut tiles_container = Vec::with_capacity(WIDTH * grid_height);

        for y in 0..grid_height {
            for x in 0..WIDTH {
                let (tile_pos_x, tile_pos_y) = Self::image_tile_position(&tile_kind, x as u32, y as u32);
                let tile_view = image.view(tile_pos_x, tile_pos_y, tile_dimensions.width, tile_dimensions.height).to_image();
                tiles_container.push(Tile::try_from(tile_view.clone()).unwrap());
            }
        }

        Ok(Self(tiles_container))
    }

    fn image_dimensions(tile_kind: &tile::Kind, height: usize) -> ImageDimensions {
        let tile_dimensions = tile_kind.dimensions();
        ImageDimensions {
            width: WIDTH as u32 * tile_dimensions.width() + (WIDTH as u32 - 1) * SEPARATOR_THICKNESS,
            height: height as u32 * tile_dimensions.height() + (height as u32 - 1) * SEPARATOR_THICKNESS
        }
    }

    pub fn height(&self) -> usize {
        let h_full_width = self.0.len() / WIDTH;
        if self.0.len() % WIDTH == 0 {
            h_full_width
        } else {
            h_full_width + 1
        }
    }

    pub fn generate_image(&self) -> Result<Image, TileKindError> {
        let tile_kind = self.0.tile_kind()?;
        let img_dim = Self::image_dimensions(&tile_kind, self.height());
        let mut image = Image::from_pixel(img_dim.width(), img_dim.height(), Rgba::from([0, 0, 0, 255]));

        for (index, tile) in self.0.iter().enumerate() {
            let (x, y) = Self::index_to_grid_coordinates(index);
            let (tile_x_position, tile_y_position) = Self::image_tile_position(&tile_kind, x as u32, y as u32);
            image.copy_from(tile.image(), tile_x_position, tile_y_position).unwrap();
        }

        Ok(image)
    }

}

impl Index<(usize, usize)> for Grid {
    type Output = Tile;

    fn index(&self, index: (usize, usize)) -> &Self::Output {
        &self.0[Self::grid_coordinates_to_index(index.0, index.1)]
    }
}

impl From<Vec<Tile>> for Grid {
    fn from(vec: Vec<Tile>) -> Self {
        Self(vec)
    }
}

impl From<&[Tile]> for Grid {
    fn from(slice: &[Tile]) -> Self {
        Self(slice.into())
    }
}