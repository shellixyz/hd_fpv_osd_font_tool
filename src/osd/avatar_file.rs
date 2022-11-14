
use std::path::{Path, PathBuf};

use derive_more::From;
use image::{GenericImageView, GenericImage, ImageBuffer, Rgba};
use thiserror::Error;
use strum::IntoEnumIterator;

use super::tile::{
    Tile,
    Kind as TileKind,
    container::uniq_tile_kind::{TileKindError, UniqTileKind},
};

use crate::{
    dimensions,
    image::{
        read_image_file,
        ReadError as ImageReadError,
        WriteImageFile,
        WriteError as ImageWriteError,
    },
    file::Error as FileError,
    osd::tile::InvalidDimensionsError,
};

pub type Image = ImageBuffer<Rgba<u8>, Vec<u8>>;
pub type ImageDimensions = dimensions::Dimensions<u32>;

pub const TILE_COUNT: usize = 256;

impl TileKind {

    pub const fn avatar_image_dimensions(&self) -> ImageDimensions {
        let tile_dimensions = self.dimensions();
        ImageDimensions { width: tile_dimensions.width, height: TILE_COUNT as u32 * tile_dimensions.height }
    }

    pub fn for_avatar_image_dimensions(dimensions: ImageDimensions) -> Result<Self, InvalidDimensionsError> {
        for kind in Self::iter() {
            if dimensions.width == kind.dimensions().width && dimensions.height == TILE_COUNT as u32 * kind.dimensions().height {
                return Ok(kind);
            }
        }
        Err(InvalidDimensionsError { dimensions })
    }

}

#[derive(Debug, From, Error)]
pub enum LoadError {
    #[error(transparent)]
    FileError(FileError),
    #[error(transparent)]
    ImageReadError(ImageReadError),
    #[from(ignore)]
    #[error("file {file_path} has dimensions ({dimensions}) which do not match any known tile kind")]
    InvalidDimensionsError {
        file_path: PathBuf,
        dimensions: ImageDimensions
    }
}

impl LoadError {
    pub fn invalid_dimensions<P: AsRef<Path>>(file_path: P, dimensions: ImageDimensions) -> Self {
        Self::InvalidDimensionsError { file_path: file_path.as_ref().to_path_buf(), dimensions }
    }
}

pub fn load<P: AsRef<Path>>(path: P) -> Result<Vec<Tile>, LoadError> {
    let image = read_image_file(&path)?;
    let tile_kind = TileKind::for_avatar_image_dimensions(image.dimensions().into())
            .map_err(|error| {
                let InvalidDimensionsError { dimensions } = error;
                LoadError::invalid_dimensions(&path, dimensions)
            })?;
    let tile_dimensions = tile_kind.dimensions();
    let mut tiles = vec![Tile::new(tile_kind); TILE_COUNT];
    for (tile_index, tile) in tiles.iter_mut().enumerate() {
        let tile_y = tile_index as u32 * tile_dimensions.height;
        let tile_from_image = image.view(0, tile_y, tile_dimensions.width, tile_dimensions.height).to_image();
        tile.copy_from(&tile_from_image, 0, 0).unwrap();
    }
    Ok(tiles)
}

#[derive(Debug, From, Error)]
pub enum SaveError {
    #[error(transparent)]
    FileError(FileError),
    #[error(transparent)]
    TileKindError(TileKindError),
    #[error(transparent)]
    ImageWriteError(ImageWriteError),
    #[error("wrong tile collection size, an Avatar file must contain exactly 256 tiles: {0}")]
    WrongCollectionSize(usize),
}

pub fn save<P: AsRef<Path>>(tiles: &[Tile], path: P) -> Result<(), SaveError> {
    if tiles.len() != TILE_COUNT {
        return Err(SaveError::WrongCollectionSize(tiles.len()));
    }
    let tile_kind = tiles.tile_kind()?;
    let img_dim = tile_kind.avatar_image_dimensions();
    let mut image = Image::new(img_dim.width(), img_dim.height());
    for (tile_index, tile) in tiles.iter().enumerate() {
        let tile_y = tile_index as u32 * tile_kind.dimensions().height;
        // let image_tile_view = image.sub_image(0, tile_y, tile_dimensions.width, tile_dimensions.height);
        image.copy_from(tile.image(), 0, tile_y).unwrap();
    }
    image.write_image_file(path)?;
    Ok(())
}