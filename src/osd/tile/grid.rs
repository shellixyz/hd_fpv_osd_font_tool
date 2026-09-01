use std::{
	ops::Index,
	path::{Path, PathBuf},
};

use derive_more::{Deref, Display, From, IntoIterator};
use getset::Getters;
use image::{GenericImage, GenericImageView, ImageBuffer, Rgba};
use strum::IntoEnumIterator;
use thiserror::Error;

use super::{
	Kind as TileKind, Tile,
	container::{
		tile_set::TileSet,
		uniq_tile_kind::{TileKindError, UniqTileKind},
	},
};
use crate::{
	create_path::{CreatePathError, create_path},
	dimensions,
	image::{ReadError as ImageLoadError, WriteError as ImageWriteError, WriteImageFile, read_image_file},
	osd::tile,
};

#[derive(Debug, Error)]
#[error("image dimensions {0} does not match valid dimensions for any of the recognized tile kinds")]
pub struct InvalidImageDimensionsError(ImageDimensions);

#[derive(Debug, From, Error, Display)]
pub enum LoadError {
	ImageLoadError(ImageLoadError),
	InvalidImageDimensions(InvalidImageDimensionsError),
	TileKindError(TileKindError),
}

#[derive(Debug, From, Error, Display)]
pub enum SaveImageError {
	CreatePathError(CreatePathError),
	ImageWriteError(ImageWriteError),
	TileKindError(TileKindError),
}

pub type ImageDimensions = dimensions::Dimensions<u32>;

const WIDTH: usize = 16;
const SEPARATOR_THICKNESS: u32 = 2;

pub type Image = ImageBuffer<Rgba<u8>, Vec<u8>>;

#[derive(Deref, IntoIterator)]
pub struct Grid(Vec<Tile>);

impl Grid {
	#[must_use]
	pub fn index_to_grid_coordinates(index: usize) -> (usize, usize) {
		(index % WIDTH, index / WIDTH)
	}

	fn grid_coordinates_to_index(x: usize, y: usize) -> usize {
		assert!(x < WIDTH);
		x + y * WIDTH
	}

	fn image_tile_position(tile_kind: tile::Kind, x: u32, y: u32) -> (u32, u32) {
		let tile_dimensions = tile_kind.dimensions();
		(
			x * (SEPARATOR_THICKNESS + tile_dimensions.width()),
			y * (SEPARATOR_THICKNESS + tile_dimensions.height()),
		)
	}

	/// Determines the tile kind and grid height from the provided image dimensions
	///
	/// # Errors
	/// Returns `InvalidImageDimensionsError` if the image dimensions do not match any valid tile
	/// kind grid
	pub fn image_tile_kind_and_grid_height(
		image_dimensions: ImageDimensions,
	) -> Result<(tile::Kind, usize), InvalidImageDimensionsError> {
		for tile_kind in tile::Kind::iter() {
			#[allow(clippy::cast_possible_truncation)]
			let expected_width = (WIDTH as u32 - 1) * SEPARATOR_THICKNESS + WIDTH as u32 * tile_kind.dimensions().width;
			if image_dimensions.width == expected_width {
				if (image_dimensions.height - tile_kind.dimensions().height)
					.is_multiple_of(tile_kind.dimensions().height + SEPARATOR_THICKNESS)
				{
					let grid_height = (image_dimensions.height - tile_kind.dimensions().height)
						/ (tile_kind.dimensions().height + SEPARATOR_THICKNESS)
						+ 1;
					return Ok((tile_kind, grid_height as usize));
				}
				return Err(InvalidImageDimensionsError(image_dimensions));
			}
		}
		Err(InvalidImageDimensionsError(image_dimensions))
	}

	/// Loads a tile grid from the provided grid image
	///
	/// # Errors
	/// Returns `LoadError` if loading fails
	pub fn load_from_image<P: AsRef<Path>>(path: P) -> Result<Self, LoadError> {
		let image = read_image_file(&path)?;
		let (img_dim_width, img_dim_height) = image.dimensions();
		let (tile_kind, grid_height) = Self::image_tile_kind_and_grid_height(ImageDimensions {
			width: img_dim_width,
			height: img_dim_height,
		})?;
		log::info!(
			"detected {tile_kind} kind of tiles in a {WIDTH}x{grid_height} grid in {}",
			path.as_ref().to_string_lossy()
		);
		let tile_dimensions = tile_kind.dimensions();
		let mut tiles_container = Vec::with_capacity(WIDTH * grid_height);

		for y in 0..grid_height {
			for x in 0..WIDTH {
				#[allow(clippy::cast_possible_truncation)]
				let (tile_pos_x, tile_pos_y) = Self::image_tile_position(tile_kind, x as u32, y as u32);
				let tile_view = image
					.view(tile_pos_x, tile_pos_y, tile_dimensions.width, tile_dimensions.height)
					.to_image();
				let Ok(tile) = Tile::try_from(tile_view) else {
					unreachable!();
				};
				tiles_container.push(tile);
			}
		}

		Ok(Self(tiles_container))
	}

	/// Loads a tile grid from the provided grid image with normalized path
	///
	/// # Errors
	/// Returns `LoadError` if loading fails
	pub fn load_from_image_norm<P: AsRef<Path>>(
		dir: P,
		tile_kind: TileKind,
		ident: Option<&str>,
	) -> Result<Self, LoadError> {
		Self::load_from_image(normalized_image_file_path(dir, tile_kind, ident))
	}

	fn image_dimensions(tile_kind: tile::Kind, height: usize) -> ImageDimensions {
		let tile_dimensions = tile_kind.dimensions();

		#[allow(clippy::cast_possible_truncation)]
		let width = WIDTH as u32 * tile_dimensions.width() + (WIDTH as u32 - 1) * SEPARATOR_THICKNESS;
		#[allow(clippy::cast_possible_truncation)]
		let height = height as u32 * tile_dimensions.height() + (height as u32 - 1) * SEPARATOR_THICKNESS;
		ImageDimensions { width, height }
	}

	#[must_use]
	pub fn height(&self) -> usize {
		let h_full_width = self.0.len() / WIDTH;
		if self.0.len().is_multiple_of(WIDTH) {
			h_full_width
		} else {
			h_full_width + 1
		}
	}

	/// Generates the grid image from the tiles
	///
	/// # Errors
	/// Returns `TileKindError` if the tile kind cannot be determined
	pub fn generate_image(&self) -> Result<Image, TileKindError> {
		let tile_kind = self.tile_kind()?;
		let img_dim = Self::image_dimensions(tile_kind, self.height());
		let mut image = Image::from_pixel(img_dim.width(), img_dim.height(), Rgba::from([0, 0, 0, 255]));

		for (index, tile) in self.0.iter().enumerate() {
			let (x, y) = Self::index_to_grid_coordinates(index);
			#[allow(clippy::cast_possible_truncation)]
			let (tile_position_x, tile_position_y) = Self::image_tile_position(tile_kind, x as u32, y as u32);
			image.copy_from(tile.image(), tile_position_x, tile_position_y).ok();
		}

		Ok(image)
	}

	/// Saves the grid image to the provided path
	///
	/// # Errors
	/// Returns `SaveImageError` if saving fails
	pub fn normalized_image_file_name(&self, ident: Option<&str>) -> Result<PathBuf, TileKindError> {
		Ok(normalized_image_file_name(self.tile_kind()?, ident))
	}

	/// Returns the normalized image file path for the grid
	///
	/// # Errors
	/// Returns `TileKindError` if the tile kind cannot be determined
	pub fn normalized_image_file_path<P: AsRef<Path>>(
		&self,
		dir: P,
		ident: Option<&str>,
	) -> Result<PathBuf, TileKindError> {
		Ok(normalized_image_file_path(dir, self.tile_kind()?, ident))
	}

	/// Saves the grid image to the provided path
	///
	/// # Errors
	/// Returns `SaveImageError` if saving fails
	pub fn save_image<P: AsRef<Path>>(&self, path: P) -> Result<(), SaveImageError> {
		self.generate_image()?.write_image_file(path)?;
		Ok(())
	}

	/// Saves the grid image to the provided path with normalized naming
	///
	/// # Errors
	/// Returns `SaveImageError` if saving fails
	pub fn save_image_norm<P: AsRef<Path>>(&self, dir: P, ident: Option<&str>) -> Result<(), SaveImageError> {
		create_path(&dir)?;
		self.save_image(self.normalized_image_file_path(&dir, ident)?)
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

#[must_use]
pub fn normalized_image_file_name(tile_kind: TileKind, ident: Option<&str>) -> PathBuf {
	let tile_kind_str = match tile_kind {
		TileKind::SD => "_sd",
		TileKind::HD => "_hd",
	};
	let ident = match ident {
		Some(ident) => format!("_{ident}"),
		None => String::new(),
	};
	PathBuf::from(format!("grid{ident}{tile_kind_str}.png"))
}

pub fn normalized_image_file_path<P: AsRef<Path>>(dir: P, tile_kind: TileKind, ident: Option<&str>) -> PathBuf {
	[dir.as_ref().to_path_buf(), normalized_image_file_name(tile_kind, ident)]
		.into_iter()
		.collect()
}

#[derive(Getters)]
#[getset(get = "pub")]
pub struct Set {
	pub(crate) sd_grid: Grid,
	pub(crate) hd_grid: Grid,
}

impl Set {
	fn check_grid_kind(grid: &Grid, expected_tile_kind: TileKind) -> Result<(), TileKindError> {
		let tile_kind = grid.tile_kind()?;
		if tile_kind != expected_tile_kind {
			return Err(TileKindError::LoadedDoesNotMatchRequested {
				requested: expected_tile_kind,
				loaded: tile_kind,
			});
		}
		Ok(())
	}

	/// Loads a tile grid set from the provided grid images
	///
	/// # Errors
	/// Returns `LoadError` if loading fails
	pub fn load_from_images<P: AsRef<Path>>(sd_grid_image_path: P, hd_grid_image_path: P) -> Result<Self, LoadError> {
		let sd_grid = Grid::load_from_image(sd_grid_image_path)?;
		Self::check_grid_kind(&sd_grid, TileKind::SD)?;
		let hd_grid = Grid::load_from_image(hd_grid_image_path)?;
		Self::check_grid_kind(&hd_grid, TileKind::HD)?;
		Ok(Self { sd_grid, hd_grid })
	}

	/// Saves a tile grid set to the provided grid images
	///
	/// # Errors
	/// Returns `SaveImageError` if saving fails
	pub fn load_from_images_norm<P: AsRef<Path>>(dir: P, ident: Option<&str>) -> Result<Self, LoadError> {
		let sd_grid = Grid::load_from_image_norm(&dir, TileKind::SD, ident)?;
		Self::check_grid_kind(&sd_grid, TileKind::SD)?;
		let hd_grid = Grid::load_from_image_norm(&dir, TileKind::HD, ident)?;
		Self::check_grid_kind(&hd_grid, TileKind::HD)?;
		Ok(Self { sd_grid, hd_grid })
	}

	/// Saves a tile grid set to the provided grid images
	///
	/// # Errors
	/// Returns `SaveImageError` if saving fails
	pub fn save_images<P: AsRef<Path>>(&self, sd_grid_path: P, hd_grid_path: P) -> Result<(), SaveImageError> {
		self.sd_grid.save_image(sd_grid_path)?;
		self.hd_grid.save_image(hd_grid_path)
	}

	/// Saves a tile grid set to the provided grid images
	///
	/// # Errors
	/// Returns `SaveImageError` if saving fails
	pub fn save_images_norm<P: AsRef<Path>>(&self, dir: P, ident: Option<&str>) -> Result<(), SaveImageError> {
		self.sd_grid.save_image_norm(&dir, ident)?;
		self.hd_grid.save_image_norm(&dir, ident)
	}

	#[must_use]
	pub fn into_tile_set(self) -> TileSet {
		TileSet {
			sd_tiles: self.sd_grid.0,
			hd_tiles: self.hd_grid.0,
		}
	}
}
