use std::{
	// io::Error as IOError,
	path::{Path, PathBuf},
};

use derive_more::From;
use image::{GenericImage, GenericImageView, ImageBuffer, Rgba};
use strum::IntoEnumIterator;
use thiserror::Error;

use super::tile::{
	Kind as TileKind, Tile,
	container::uniq_tile_kind::{TileKindError, UniqTileKind},
};

use crate::{
	dimensions,
	image::{ReadError as ImageReadError, WriteError as ImageWriteError, WriteImageFile, read_image_file},
	osd::tile::InvalidDimensionsError,
};

pub type Image = ImageBuffer<Rgba<u8>, Vec<u8>>;
pub type ImageDimensions = dimensions::Dimensions<u32>;

pub const TILE_COUNT: usize = 256;

impl TileKind {
	#[must_use]
	pub const fn avatar_image_dimensions(&self) -> ImageDimensions {
		let tile_dimensions = self.dimensions();
		#[allow(clippy::cast_possible_truncation)]
		let height = TILE_COUNT as u32 * tile_dimensions.height;
		ImageDimensions {
			width: tile_dimensions.width,
			height,
		}
	}

	/// Finds the tile kind matching the provided Avatar image dimensions
	///
	/// # Errors
	/// Returns `InvalidDimensionsError` if no matching tile kind is found
	pub fn for_avatar_image_dimensions(dimensions: ImageDimensions) -> Result<Self, InvalidDimensionsError> {
		for kind in Self::iter() {
			#[allow(clippy::cast_possible_truncation)]
			let expected_height = TILE_COUNT as u32 * kind.dimensions().height;
			if dimensions.width == kind.dimensions().width && dimensions.height == expected_height {
				return Ok(kind);
			}
		}
		Err(InvalidDimensionsError { dimensions })
	}
}

#[derive(Debug, From, Error)]
pub enum LoadError {
	// #[error("failed loading image `{file_path}`: {error}")]
	// FileError {
	//     file_path: PathBuf,
	//     error: IOError
	// },
	#[error(transparent)]
	ImageReadError(ImageReadError),
	#[from(ignore)]
	#[error("file {file_path} has dimensions ({dimensions}) which do not match any known tile kind")]
	InvalidDimensionsError {
		file_path: PathBuf,
		dimensions: ImageDimensions,
	},
}

impl LoadError {
	// pub fn file_error<P: AsRef<Path>>(file_path: P, error: IOError) -> Self {
	//     Self::FileError {
	//         file_path: file_path.as_ref().to_path_buf(),
	//         error
	//     }
	// }

	pub fn invalid_dimensions<P: AsRef<Path>>(file_path: P, dimensions: ImageDimensions) -> Self {
		Self::InvalidDimensionsError {
			file_path: file_path.as_ref().to_path_buf(),
			dimensions,
		}
	}
}

/// Loads an Avatar tile collection from the provided image file
///
/// # Errors
/// Returns `LoadError` if loading fails
pub fn load<P: AsRef<Path>>(path: P) -> Result<Vec<Tile>, LoadError> {
	let image = read_image_file(&path)?;
	let tile_kind = TileKind::for_avatar_image_dimensions(image.dimensions().into()).map_err(|error| {
		let InvalidDimensionsError { dimensions } = error;
		LoadError::invalid_dimensions(&path, dimensions)
	})?;
	let tile_dimensions = tile_kind.dimensions();
	let mut tiles = vec![Tile::new(tile_kind); TILE_COUNT];
	for (tile_index, tile) in tiles.iter_mut().enumerate() {
		#[allow(clippy::cast_possible_truncation)]
		let tile_y = tile_index as u32 * tile_dimensions.height;
		let tile_from_image = image
			.view(0, tile_y, tile_dimensions.width, tile_dimensions.height)
			.to_image();
		tile.copy_from(&tile_from_image, 0, 0).ok();
	}
	Ok(tiles)
}

#[derive(Debug, From, Error)]
pub enum SaveError {
	#[error(transparent)]
	TileKindError(TileKindError),
	#[error(transparent)]
	ImageWriteError(ImageWriteError),
	#[error("not enough tiles, Avatar tile collection must contain 256 tiles")]
	WrongCollectionSize(usize),
}

/// Saves an Avatar tile collection to the provided image file
///
/// # Errors
/// Returns `SaveError` if saving fails
pub fn save<P: AsRef<Path>>(tiles: &[Tile], path: P) -> Result<(), SaveError> {
	if tiles.len() < TILE_COUNT {
		return Err(SaveError::WrongCollectionSize(tiles.len()));
	}
	if tiles.len() > TILE_COUNT {
		log::warn!(
			"Avatar font files can only contain 256 tiles but the source collection contains {}",
			tiles.len()
		);
	}
	let tile_kind = tiles.tile_kind()?;
	let img_dim = tile_kind.avatar_image_dimensions();
	let mut image = Image::new(img_dim.width(), img_dim.height());
	for (tile_index, tile) in tiles[0..TILE_COUNT].iter().enumerate() {
		#[allow(clippy::cast_possible_truncation)]
		let tile_y = tile_index as u32 * tile_kind.dimensions().height;
		image.copy_from(tile.image(), 0, tile_y).ok();
	}
	image.write_image_file(path)?;
	Ok(())
}
