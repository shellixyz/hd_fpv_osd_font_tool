use std::path::{Path, PathBuf};

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

/// Number of tiles in a font page, which is one column of an Avatar image
pub const TILE_COUNT: usize = 256;

/// Maximum number of font pages (columns) in an Avatar image
pub const MAX_PAGE_COUNT: usize = 4;

/// Maximum number of tiles an Avatar image can hold
pub const MAX_TILE_COUNT: usize = MAX_PAGE_COUNT * TILE_COUNT;

impl TileKind {
	/// Dimensions of a single-page Avatar image for this tile kind
	#[must_use]
	pub const fn avatar_image_dimensions(&self) -> ImageDimensions {
		self.avatar_image_dimensions_for_pages(1)
	}

	/// Dimensions of an Avatar image with `page_count` font pages for this tile kind
	#[must_use]
	pub const fn avatar_image_dimensions_for_pages(&self, page_count: u32) -> ImageDimensions {
		let tile_dimensions = self.dimensions();
		#[allow(clippy::cast_possible_truncation)]
		let height = TILE_COUNT as u32 * tile_dimensions.height;
		ImageDimensions {
			width: page_count * tile_dimensions.width,
			height,
		}
	}

	/// Finds the tile kind matching the provided Avatar image dimensions
	///
	/// The image must be 256 tiles tall and 1 to 4 tiles wide.
	///
	/// # Errors
	/// Returns `InvalidDimensionsError` if no matching tile kind is found
	pub fn for_avatar_image_dimensions(dimensions: ImageDimensions) -> Result<Self, InvalidDimensionsError> {
		avatar_image_layout(dimensions).map(|(kind, _)| kind)
	}
}

/// Finds the tile kind and number of font pages of an Avatar image from its dimensions
fn avatar_image_layout(dimensions: ImageDimensions) -> Result<(TileKind, u32), InvalidDimensionsError> {
	for kind in TileKind::iter() {
		let single_page = kind.avatar_image_dimensions();
		if dimensions.height != single_page.height || !dimensions.width.is_multiple_of(single_page.width) {
			continue;
		}
		let page_count = dimensions.width / single_page.width;
		#[allow(clippy::cast_possible_truncation)]
		if (1..=MAX_PAGE_COUNT as u32).contains(&page_count) {
			return Ok((kind, page_count));
		}
	}
	Err(InvalidDimensionsError { dimensions })
}

#[derive(Debug, From, Error)]
pub enum LoadError {
	#[error(transparent)]
	ImageReadError(ImageReadError),
	#[from(ignore)]
	#[error(
		"file {file_path} has dimensions ({dimensions}) which do not match an Avatar font: expected {TILE_COUNT} \
		 tiles tall and 1 to {MAX_PAGE_COUNT} tiles wide"
	)]
	InvalidDimensionsError {
		file_path: PathBuf,
		dimensions: ImageDimensions,
	},
}

impl LoadError {
	pub fn invalid_dimensions<P: AsRef<Path>>(file_path: P, dimensions: ImageDimensions) -> Self {
		Self::InvalidDimensionsError {
			file_path: file_path.as_ref().to_path_buf(),
			dimensions,
		}
	}
}

/// Loads an Avatar tile collection from the provided image file
///
/// The image holds 1 to 4 font pages side by side, each a column of 256 tiles. Tiles are returned
/// in glyph index order: page × 256 + row.
///
/// # Errors
/// Returns `LoadError` if loading fails
pub fn load<P: AsRef<Path>>(path: P) -> Result<Vec<Tile>, LoadError> {
	let image = read_image_file(&path)?;
	let (tile_kind, page_count) = avatar_image_layout(image.dimensions().into()).map_err(|error| {
		let InvalidDimensionsError { dimensions } = error;
		LoadError::invalid_dimensions(&path, dimensions)
	})?;
	let tile_dimensions = tile_kind.dimensions();
	let mut tiles = vec![Tile::new(tile_kind); page_count as usize * TILE_COUNT];
	for (tile_index, tile) in tiles.iter_mut().enumerate() {
		let (tile_x, tile_y) = tile_position(tile_kind, tile_index);
		let tile_from_image = image
			.view(tile_x, tile_y, tile_dimensions.width, tile_dimensions.height)
			.to_image();
		tile.copy_from(&tile_from_image, 0, 0).ok();
	}
	Ok(tiles)
}

/// Pixel position of the top left corner of a tile in an Avatar image
#[allow(clippy::cast_possible_truncation)]
const fn tile_position(tile_kind: TileKind, tile_index: usize) -> (u32, u32) {
	let tile_dimensions = tile_kind.dimensions();
	let page = (tile_index / TILE_COUNT) as u32;
	let row = (tile_index % TILE_COUNT) as u32;
	(page * tile_dimensions.width, row * tile_dimensions.height)
}

#[derive(Debug, From, Error)]
pub enum SaveError {
	#[error(transparent)]
	TileKindError(TileKindError),
	#[error(transparent)]
	ImageWriteError(ImageWriteError),
}

/// Saves an Avatar tile collection to the provided image file
///
/// Writes one font page (column of 256 tiles) per started group of 256 tiles, up to 4 pages. A
/// partial last page is padded with blank tiles and tiles beyond 1024 are dropped with a warning.
/// An empty collection is an error because its tile kind is unknown.
///
/// # Errors
/// Returns `SaveError` if saving fails
pub fn save<P: AsRef<Path>>(tiles: &[Tile], path: P) -> Result<(), SaveError> {
	if tiles.len() > MAX_TILE_COUNT {
		log::warn!(
			"Avatar font files can only contain {MAX_TILE_COUNT} tiles but the source collection contains {}, \
			 dropping the last {}",
			tiles.len(),
			tiles.len() - MAX_TILE_COUNT
		);
	}
	let tiles = &tiles[..tiles.len().min(MAX_TILE_COUNT)];
	let tile_kind = tiles.tile_kind()?;
	#[allow(clippy::cast_possible_truncation)]
	let page_count = tiles.len().div_ceil(TILE_COUNT) as u32;
	let img_dim = tile_kind.avatar_image_dimensions_for_pages(page_count);
	let mut image = Image::new(img_dim.width(), img_dim.height());
	for (tile_index, tile) in tiles.iter().enumerate() {
		let (tile_x, tile_y) = tile_position(tile_kind, tile_index);
		image.copy_from(tile.image(), tile_x, tile_y).ok();
	}
	image.write_image_file(path)?;
	Ok(())
}

#[cfg(test)]
mod tests {
	use std::{fmt::Write, fs, path::Path};

	use image::{GenericImageView, Rgba};
	use sha2::{Digest, Sha256};
	use strum::IntoEnumIterator;
	use temp_dir::TempDir;

	use super::{Image, ImageDimensions, LoadError, TILE_COUNT, load, save};
	use crate::osd::{
		bin_file::{self, FontPart},
		tile::{Kind as TileKind, Tile},
	};

	/// Marks a tile with its glyph index so its position can be checked after a round trip
	fn marker(index: usize) -> Rgba<u8> {
		let [low, high, ..] = index.to_le_bytes();
		Rgba([low, high, 0x5A, 0xFF])
	}

	fn numbered_tiles(kind: TileKind, count: usize) -> Vec<Tile> {
		(0..count)
			.map(|index| {
				let mut tile = Tile::new(kind);
				tile.put_pixel(0, 0, marker(index));
				tile
			})
			.collect()
	}

	fn tile_index(tile: &Tile) -> Option<usize> {
		let Rgba([low, high, check, alpha]) = *tile.get_pixel(0, 0);
		(check == 0x5A && alpha == 0xFF).then(|| usize::from(u16::from_le_bytes([low, high])))
	}

	fn is_blank(tile: &Tile) -> bool {
		tile.pixels().all(|pixel| *pixel == Rgba([0, 0, 0, 0]))
	}

	fn image_with_tile_counts(kind: TileKind, columns: u32, rows: u32) -> Image {
		let tile = kind.dimensions();
		Image::new(columns * tile.width, rows * tile.height)
	}

	#[test]
	fn round_trip_1_2_and_4_pages() {
		let temp_dir = TempDir::new().unwrap();
		for kind in TileKind::iter() {
			for pages in [1, 2, 4] {
				let path = temp_dir.child(format!("{kind}_{pages}.png"));
				let tiles = numbered_tiles(kind, pages * TILE_COUNT);
				save(&tiles, &path).unwrap();

				let saved = image::open(&path).unwrap();
				let tile = kind.dimensions();
				#[allow(clippy::cast_possible_truncation)]
				let expected_width = pages as u32 * tile.width;
				assert_eq!(saved.dimensions(), (expected_width, 256 * tile.height));

				let loaded = load(&path).unwrap();
				assert_eq!(loaded.len(), tiles.len(), "{kind} {pages} pages");
				for (index, (loaded, original)) in loaded.iter().zip(&tiles).enumerate() {
					assert_eq!(loaded.kind(), kind);
					assert_eq!(loaded.image(), original.image(), "{kind} {pages} pages, glyph {index}");
				}
			}
		}
	}

	#[test]
	fn load_orders_glyphs_by_page_then_row() {
		let temp_dir = TempDir::new().unwrap();
		for kind in TileKind::iter() {
			let tile = kind.dimensions();
			let mut image = image_with_tile_counts(kind, 3, 256);
			for page in 0..3 {
				for row in 0..256 {
					image.put_pixel(
						page * tile.width,
						row * tile.height,
						marker((page * 256 + row) as usize),
					);
				}
			}
			let path = temp_dir.child(format!("{kind}_ordering.png"));
			image.save(&path).unwrap();

			let loaded = load(&path).unwrap();
			assert_eq!(loaded.len(), 3 * TILE_COUNT);
			for (index, tile) in loaded.iter().enumerate() {
				assert_eq!(tile_index(tile), Some(index), "{kind} glyph {index}");
			}
		}
	}

	#[test]
	fn load_rejects_invalid_dimensions() {
		let temp_dir = TempDir::new().unwrap();
		for kind in TileKind::iter() {
			let tile = kind.dimensions();
			let cases = [
				("five_pages", 5 * tile.width, 256 * tile.height),
				("partial_column", tile.width + 1, 256 * tile.height),
				("half_column", tile.width + tile.width / 2, 256 * tile.height),
				("too_short", tile.width, 255 * tile.height),
				("too_tall", tile.width, 257 * tile.height),
				("stacked_pages", tile.width, 512 * tile.height),
			];
			for (name, width, height) in cases {
				let path = temp_dir.child(format!("{kind}_{name}.png"));
				Image::new(width, height).save(&path).unwrap();
				let result = load(&path);
				let Err(LoadError::InvalidDimensionsError { file_path, dimensions }) = &result else {
					panic!("{kind} {name}: {width}x{height} did not give a dimensions error: {result:?}");
				};
				assert_eq!(file_path, &path);
				assert_eq!(*dimensions, ImageDimensions { width, height });
				let message = result.unwrap_err().to_string();
				assert!(message.contains(path.to_str().unwrap()), "{message}");
				assert!(message.contains(&format!("{width}x{height}")), "{message}");
			}
		}
	}

	#[test]
	fn save_pads_partial_last_page_with_blank_tiles() {
		let temp_dir = TempDir::new().unwrap();
		for kind in TileKind::iter() {
			let path = temp_dir.child(format!("{kind}_partial.png"));
			save(&numbered_tiles(kind, 300), &path).unwrap();

			let loaded = load(&path).unwrap();
			assert_eq!(loaded.len(), 2 * TILE_COUNT);
			for (index, tile) in loaded.iter().enumerate() {
				if index < 300 {
					assert_eq!(tile_index(tile), Some(index));
				} else {
					assert!(is_blank(tile), "{kind} glyph {index} is not blank");
				}
			}
		}
	}

	#[test]
	fn save_pads_short_collection_to_one_page() {
		let temp_dir = TempDir::new().unwrap();
		for kind in TileKind::iter() {
			let path = temp_dir.child(format!("{kind}_short.png"));
			save(&numbered_tiles(kind, 100), &path).unwrap();

			let loaded = load(&path).unwrap();
			assert_eq!(loaded.len(), TILE_COUNT);
			for (index, tile) in loaded.iter().enumerate() {
				if index < 100 {
					assert_eq!(tile_index(tile), Some(index));
				} else {
					assert!(is_blank(tile), "{kind} glyph {index} is not blank");
				}
			}
		}
	}

	#[test]
	fn save_rejects_empty_collection() {
		let temp_dir = TempDir::new().unwrap();
		assert!(save(&[], temp_dir.child("empty.png")).is_err());
	}

	#[test]
	fn save_keeps_the_first_4_pages() {
		let temp_dir = TempDir::new().unwrap();
		for kind in TileKind::iter() {
			let path = temp_dir.child(format!("{kind}_overflow.png"));
			save(&numbered_tiles(kind, 4 * TILE_COUNT + 10), &path).unwrap();

			let loaded = load(&path).unwrap();
			assert_eq!(loaded.len(), 4 * TILE_COUNT);
			for (index, tile) in loaded.iter().enumerate() {
				assert_eq!(tile_index(tile), Some(index));
			}
		}
	}

	#[test]
	fn single_page_output_matches_1_1_1() {
		// SHA-256 of the files 1.1.1 writes for the bundled base fonts. The PNG encoder comes from
		// the `image` crate, so an upgrade of that crate may change these.
		let expected = [
			(
				TileKind::SD,
				"5bed1b923230861f4fa3f2a67811c5cd56fa26c3e75329e5beacadb017c6adb8",
			),
			(
				TileKind::HD,
				"98199e34934184473e1f8f2f512b4e27ba914088eabb56ef05a48d5474369b7c",
			),
		];
		let temp_dir = TempDir::new().unwrap();
		for (kind, expected_hash) in expected {
			let bin_path = bin_file::normalized_file_path("test_files/djibinsetnorm", kind, None, FontPart::Base);
			let tiles = bin_file::load(bin_path).unwrap();
			let path = temp_dir.child(format!("{kind}.png"));
			save(&tiles, &path).unwrap();
			let hash = Sha256::digest(fs::read(&path).unwrap());
			let hash = hash.iter().fold(String::new(), |mut hex, byte| {
				write!(hex, "{byte:02x}").unwrap();
				hex
			});
			assert_eq!(hash, expected_hash, "{kind}");
		}
	}

	#[test]
	fn djibin_base_and_extended_pair_saves_as_two_pages() {
		let temp_dir = TempDir::new().unwrap();
		let dir = Path::new("test_files/djibinsetnorm");
		for kind in TileKind::iter() {
			let tiles = bin_file::load_extended(
				bin_file::normalized_file_path(dir, kind, None, FontPart::Base),
				bin_file::normalized_file_path(dir, kind, None, FontPart::Ext),
			)
			.unwrap();
			assert_eq!(tiles.len(), 2 * TILE_COUNT);

			let path = temp_dir.child(format!("{kind}_pair.png"));
			save(&tiles, &path).unwrap();
			let tile = kind.dimensions();
			assert_eq!(
				image::open(&path).unwrap().dimensions(),
				(2 * tile.width, 256 * tile.height)
			);

			let loaded = load(&path).unwrap();
			let page_1 = loaded[TILE_COUNT..].iter().zip(&tiles[TILE_COUNT..]);
			assert!(
				page_1
					.into_iter()
					.all(|(loaded, original)| loaded.image() == original.image())
			);
		}
	}
}
