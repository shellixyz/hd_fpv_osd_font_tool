
use std::fmt::Display;
use std::ops::Index;
use std::path::{Path, PathBuf};

use derive_more::{Error, Deref, Display, From, IntoIterator};
use getset::Getters;
use image::{ImageBuffer, Rgba, GenericImage, GenericImageView};
use strum::IntoEnumIterator;

use super::container::tile_set::TileSet;
use super::{Tile, Kind as TileKind};
use super::container::uniq_tile_kind::{UniqTileKind, TileKindError};
use crate::create_path::{create_path, CreatePathError};
use crate::dimensions;
use crate::osd::tile;
use crate::image::{read_image_file, WriteImageFile, ReadError as ImageLoadError, WriteError as ImageWriteError};


#[derive(Debug, Error)]
pub struct InvalidImageDimensionsError;

impl Display for InvalidImageDimensionsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("image dimensions does not match valid dimensions for any of the recognized tile kinds")
    }
}

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

    pub fn load_from_image<P: AsRef<Path>>(path: P) -> Result<Self, LoadError> {
        let image = read_image_file(&path)?;
        let (img_dim_width, img_dim_height) = image.dimensions();
        let (tile_kind, grid_height) = Self::image_tile_kind_and_grid_height(ImageDimensions { width: img_dim_width, height: img_dim_height })?;
        log::info!("detected {tile_kind} kind of tiles in a {WIDTH}x{grid_height} grid in {}", path.as_ref().to_string_lossy());
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

    pub fn load_from_image_norm<P: AsRef<Path>>(dir: P, tile_kind: TileKind, ident: &Option<&str>) -> Result<Self, LoadError> {
        Self::load_from_image(normalized_image_file_path(dir, tile_kind, ident))
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
        let tile_kind = self.tile_kind()?;
        let img_dim = Self::image_dimensions(&tile_kind, self.height());
        let mut image = Image::from_pixel(img_dim.width(), img_dim.height(), Rgba::from([0, 0, 0, 255]));

        for (index, tile) in self.0.iter().enumerate() {
            let (x, y) = Self::index_to_grid_coordinates(index);
            let (tile_x_position, tile_y_position) = Self::image_tile_position(&tile_kind, x as u32, y as u32);
            image.copy_from(tile.image(), tile_x_position, tile_y_position).unwrap();
        }

        Ok(image)
    }

    pub fn normalized_image_file_name(&self, ident: &Option<&str>) -> Result<PathBuf, TileKindError> {
        Ok(normalized_image_file_name(self.tile_kind()?, ident))
    }

    pub fn normalized_image_file_path<P: AsRef<Path>>(&self, dir: P, ident: &Option<&str>) -> Result<PathBuf, TileKindError> {
        Ok(normalized_image_file_path(dir, self.tile_kind()?, ident))
    }

    pub fn save_image<P: AsRef<Path>>(&self, path: P) -> Result<(), SaveImageError> {
        self.generate_image()?.write_image_file(path)?;
        Ok(())
    }

    pub fn save_image_norm<P: AsRef<Path>>(&self, dir: P, ident: &Option<&str>) -> Result<(), SaveImageError> {
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

pub fn normalized_image_file_name(tile_kind: TileKind, ident: &Option<&str>) -> PathBuf {
    let tile_kind_str = match tile_kind {
        TileKind::SD => "_sd",
        TileKind::HD => "_hd",
    };
    let ident = match ident {
        Some(ident) => format!("_{ident}"),
        None => "".to_owned(),
    };
    PathBuf::from(format!("grid{ident}{tile_kind_str}.png"))
}

pub fn normalized_image_file_path<P: AsRef<Path>>(dir: P, tile_kind: TileKind, ident: &Option<&str>) -> PathBuf {
    [dir.as_ref().to_path_buf(), normalized_image_file_name(tile_kind, ident)].into_iter().collect()
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
            return Err(TileKindError::LoadedDoesNotMatchRequested { requested: expected_tile_kind, loaded: tile_kind })
        }
        Ok(())
    }

    pub fn load_from_images<P: AsRef<Path>>(sd_grid_image_path: P, hd_grid_image_path: P) -> Result<Self, LoadError> {
        let sd_grid = Grid::load_from_image(sd_grid_image_path)?;
        Self::check_grid_kind(&sd_grid, TileKind::SD)?;
        let hd_grid = Grid::load_from_image(hd_grid_image_path)?;
        Self::check_grid_kind(&hd_grid, TileKind::HD)?;
        Ok(Self { sd_grid, hd_grid })
    }

    pub fn load_from_images_norm<P: AsRef<Path>>(dir: P, ident: &Option<&str>) -> Result<Self, LoadError> {
        let sd_grid = Grid::load_from_image_norm(&dir, TileKind::SD, ident)?;
        Self::check_grid_kind(&sd_grid, TileKind::SD)?;
        let hd_grid = Grid::load_from_image_norm(&dir, TileKind::HD, ident)?;
        Self::check_grid_kind(&hd_grid, TileKind::HD)?;
        Ok(Self { sd_grid, hd_grid })
    }

    pub fn save_images<P: AsRef<Path>>(&self, sd_grid_path: P, hd_grid_path: P) -> Result<(), SaveImageError> {
        self.sd_grid.save_image(sd_grid_path)?;
        self.hd_grid.save_image(hd_grid_path)
    }

    pub fn save_images_norm<P: AsRef<Path>>(&self, dir: P, ident: &Option<&str>) -> Result<(), SaveImageError> {
        self.sd_grid.save_image_norm(&dir, ident)?;
        self.hd_grid.save_image_norm(&dir, ident)
    }

    pub fn into_tile_set(self) -> TileSet {
        TileSet { sd_tiles: self.sd_grid.0, hd_tiles: self.hd_grid.0 }
    }

}