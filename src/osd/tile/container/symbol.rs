pub mod set;
pub mod spec;

use derive_more::{Error, From, Index};
use getset::CopyGetters;
use image::{GenericImage, GenericImageView, ImageBuffer, Rgba};
use std::fmt::Display;
use std::path::Path;

use crate::dimensions;
use crate::image::{ReadError as ImageReadError, read_image_file};
use crate::osd::tile::{
	InvalidHeightError, Kind as TileKind, Tile,
	container::{TileKindError, UniqTileKind},
};

#[derive(Debug, From, Error)]
pub enum LoadError {
	ImageReadError(ImageReadError),
	InvalidImageHeightError(InvalidHeightError),
	InvalidImageWidthError { tile_kind: TileKind, image_width: u32 },
}

impl Display for LoadError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		use LoadError as LE;
		match self {
			LE::ImageReadError(image_error) => image_error.fmt(f),
			LE::InvalidImageWidthError { tile_kind, image_width } => {
				write!(f, "invalid tile image width for {tile_kind} tile kind: {image_width}")
			},
			LE::InvalidImageHeightError(error) => error.fmt(f),
		}
	}
}

pub type ImageDimensions = dimensions::Dimensions<u32>;

pub type Image = ImageBuffer<Rgba<u8>, Vec<u8>>;

#[derive(Clone, Index, CopyGetters)]
pub struct Symbol {
	#[getset(get_copy = "pub")]
	tile_kind: TileKind,
	#[index]
	tiles: Vec<Tile>,
}

impl Symbol {
	#[must_use]
	pub fn new(tile_kind: TileKind) -> Self {
		Self {
			tile_kind,
			tiles: vec![Tile::new(tile_kind)],
		}
	}

	/// Loads a symbol from an image file
	///
	/// # Errors
	/// Returns `LoadError` if loading fails or the image dimensions are invalid
	pub fn load_image_file<P: AsRef<Path>>(path: P) -> Result<Self, LoadError> {
		let image = read_image_file(&path)?;
		let (image_width, image_height) = image.dimensions();
		let tile_kind = TileKind::for_height(image_height)?;
		let tile_dimensions = tile_kind.dimensions();
		if image_width % tile_dimensions.width != 0 {
			return Err(LoadError::InvalidImageWidthError { tile_kind, image_width });
		}
		let span = image_width / tile_dimensions.width;
		let mut tiles = Vec::with_capacity(span as usize);
		for tile_index in 0..span {
			let tile_x = tile_index * tile_dimensions.width;
			let Ok(tile) = Tile::try_from(
				image
					.view(tile_x, 0, tile_dimensions.width, tile_dimensions.height)
					.to_image(),
			) else {
				unreachable!();
			};
			tiles.push(tile);
		}
		Ok(Self { tile_kind, tiles })
	}

	#[must_use]
	pub fn span(&self) -> usize {
		self.tiles.len()
	}

	#[must_use]
	pub fn tiles(&self) -> &Vec<Tile> {
		&self.tiles
	}

	#[must_use]
	pub fn into_tiles(self) -> Vec<Tile> {
		self.tiles
	}

	#[must_use]
	pub fn image_dimensions(&self) -> ImageDimensions {
		#[allow(clippy::cast_possible_truncation)]
		ImageDimensions {
			width: self.span() as u32 * self.tile_kind.dimensions().width,
			height: self.tile_kind.dimensions().height,
		}
	}

	#[must_use]
	pub fn generate_image(&self) -> Image {
		let mut image = Image::new(self.image_dimensions().width, self.image_dimensions().height);

		for (index, tile) in self.tiles.iter().enumerate() {
			#[allow(clippy::cast_possible_truncation)]
			let x = index as u32 * self.tile_kind.dimensions().width;
			image.copy_from(tile.image(), x, 0).ok();
		}

		image
	}
}

impl TryFrom<Vec<Tile>> for Symbol {
	type Error = TileKindError;

	fn try_from(tiles: Vec<Tile>) -> Result<Self, Self::Error> {
		let tile_kind = tiles.tile_kind()?;
		Ok(Self { tile_kind, tiles })
	}
}

impl From<Tile> for Symbol {
	fn from(tile: Tile) -> Self {
		Self {
			tile_kind: tile.kind(),
			tiles: vec![tile],
		}
	}
}
