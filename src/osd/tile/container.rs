
use std::fmt::Display;
use std::path::{Path, PathBuf};
use std::io::Error as IOError;

use derive_more::{Error, Display, From};
use getset::Getters;
use image::ImageError;

use crate::osd::bin_file::{BinFileWriter, self, TileWriteError, FillRemainingSpaceError};

use super::{Tile, Kind as TileKind};
use super::LoadError as TileLoadError;
use super::grid::Grid as TileGrid;

#[derive(Debug, Error, Display, From)]
pub enum SaveTilesToDirError {
    IOError(IOError),
    ImageError(ImageError),
}

pub trait SaveTilesToDir {
    fn save_tiles_to_dir<P: AsRef<Path>>(&self, path: P) -> Result<(), SaveTilesToDirError>;
}

impl<T> SaveTilesToDir for T
where
    for<'any> &'any T: IntoIterator<Item = &'any Tile>,
{
    fn save_tiles_to_dir<P: AsRef<Path>>(&self, path: P) -> Result<(), SaveTilesToDirError> {
        std::fs::create_dir_all(&path)?;

        for (index, tile) in self.into_iter().enumerate() {
            let path: PathBuf = [path.as_ref().to_str().unwrap(), &format!("{:03}.png", index)].iter().collect();
            tile.save(path)?;
        }

        Ok(())
    }
}

#[derive(Debug, Error)]
pub enum TileKindError {
    EmptyContainer,
    MultipleTileKinds,
    LoadedDoesNotMatchRequested {
        requested: TileKind,
        loaded: TileKind,
    }
}

impl Display for TileKindError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use TileKindError::*;
        match self {
            EmptyContainer => f.write_str("cannot determine tile kind from empty container"),
            MultipleTileKinds => f.write_str("container includes multiple tile kinds"),
            LoadedDoesNotMatchRequested { requested, loaded } => write!(f, "loaded kind does not match requested: loaded {loaded}, requested {requested}"),
        }
    }
}

pub trait UniqTileKind {
    fn tile_kind(&self) -> Result<super::Kind, TileKindError>;
}

impl UniqTileKind for &[Tile] {
    fn tile_kind(&self) -> Result<super::Kind, TileKindError> {
        let first_tile_kind = self.first().ok_or(TileKindError::EmptyContainer)?.kind();
        if ! self.iter().all(|tile| tile.kind() == first_tile_kind) {
            return Err(TileKindError::MultipleTileKinds)
        }
        Ok(first_tile_kind)
    }
}

impl UniqTileKind for Vec<Tile> {
    fn tile_kind(&self) -> Result<super::Kind, TileKindError> {
        self.as_slice().tile_kind()
    }
}

#[derive(Debug, Error, Display, From)]
pub enum SaveTilesToBinFileError {
    CreateError(IOError),
    TileKindError(TileKindError),
    TileWriteError(TileWriteError),
    FillRemainingSpaceError(FillRemainingSpaceError)
}

pub trait SaveTilesToBinFile {
    fn save_tiles_to_bin_file<P: AsRef<Path>>(&self, path: P) -> Result<(), SaveTilesToBinFileError>;
}

impl SaveTilesToBinFile for &[Tile] {
    fn save_tiles_to_bin_file<P: AsRef<Path>>(&self, path: P) -> Result<(), SaveTilesToBinFileError> {
        self.tile_kind()?;
        let mut writer = BinFileWriter::create(path)?;

        for tile in self.iter() {
            writer.write_tile(tile)?;
        }

        writer.fill_remaining_space()?;
        writer.finish()?;
        Ok(())
    }
}

impl SaveTilesToBinFile for TileGrid {
    fn save_tiles_to_bin_file<P: AsRef<Path>>(&self, path: P) -> Result<(), SaveTilesToBinFileError> {
        self.as_slice().save_tiles_to_bin_file(path)
    }
}

impl SaveTilesToBinFile for Vec<Tile> {
    fn save_tiles_to_bin_file<P: AsRef<Path>>(&self, path: P) -> Result<(), SaveTilesToBinFileError> {
        self.as_slice().save_tiles_to_bin_file(path)
    }
}

pub trait SaveToBinFiles {
    fn save_to_bin_files<P: AsRef<Path>>(&self, path1: P, path2: P) -> Result<(), SaveTilesToBinFileError>;
}

impl SaveToBinFiles for &[Tile] {
    fn save_to_bin_files<P: AsRef<Path>>(&self, path1: P, path2: P) -> Result<(), SaveTilesToBinFileError> {
        (&self[0..bin_file::TILE_COUNT]).save_tiles_to_bin_file(path1)?;
        (&self[bin_file::TILE_COUNT..2 * bin_file::TILE_COUNT]).save_tiles_to_bin_file(path2)
    }
}

pub trait IntoTileGrid {
    fn into_tile_grid(self) -> TileGrid;
}

impl IntoTileGrid for &[Tile] {
    fn into_tile_grid(self) -> TileGrid {
        TileGrid::from(self)
    }
}

#[derive(Debug, Error, From)]
pub enum LoadTilesFromDirError {
    LoadError(TileLoadError),
    NoTileFound,
    KindMismatchError
}

impl Display for LoadTilesFromDirError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LoadTilesFromDirError::LoadError(load_error) => load_error.fmt(f),
            LoadTilesFromDirError::KindMismatchError => f.write_str("directory contains different kinds of tiles"),
            LoadTilesFromDirError::NoTileFound => f.write_str("no tile found"),
        }
    }
}

pub fn load_tiles_from_dir<P: AsRef<Path>>(path: P, max_tiles: usize) -> Result<Vec<Tile>, LoadTilesFromDirError> {
    let mut tiles = vec![];
    let mut tile_kind = None;

    for index in 0..max_tiles {
        let tile_path: PathBuf = [path.as_ref(), Path::new(&format!("{:03}.png", index))].iter().collect();
        let tile = match Tile::load_image_file(tile_path) {
            Ok(loaded_tile) => Some(loaded_tile),
            Err(error) => match &error {
                TileLoadError::IOError(io_error) =>
                    match io_error.kind() {
                        std::io::ErrorKind::NotFound => None,
                        _ => return Err(error.into()),
                    },
                _ => return Err(error.into())
            },
        };

        match (&tile, &tile_kind) {

            // first loaded tile: record the kind of tile
            (Some(tile), None) => {
                log::info!("detected {} kind of tiles in {}", tile.kind(), path.as_ref().to_string_lossy());
                tile_kind = Some(tile.kind());
            },

            // we have already loaded a tile before, check that the new tile kind is matching what had recorded
            (Some(tile), Some(tile_kind)) => if tile.kind() != *tile_kind {
                return Err(LoadTilesFromDirError::KindMismatchError)
            },

            _ => {}

        }

        tiles.push(tile);
    }

    let tiles = match tile_kind {
        Some(tile_kind) => {
            let last_some_index = tiles.iter().rposition(Option::is_some).unwrap();
            tiles[0..=last_some_index].iter().map(|tile| tile.clone().unwrap_or_else(|| Tile::new(tile_kind))).collect()
        }
        None => return Err(LoadTilesFromDirError::NoTileFound),
    };

    Ok(tiles)
}

#[derive(Debug)]
pub enum TileSetFromError {
    TileKindMismatch(TileKind),
    WrongTileKind(TileKind),
}

impl std::error::Error for TileSetFromError {}

impl Display for TileSetFromError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use TileSetFromError::*;
        match self {
            TileKindMismatch(collection_kind) => write!(f, "mismatched tile kinds in {collection_kind} collection"),
            WrongTileKind(collection_kind) => write!(f, "wrong tile kind in {collection_kind} collection"),
        }
    }
}

#[derive(Debug, Display, Error, From)]
pub enum LoadTileSetTilesFromDirError {
    LoadTilesFromDirError(LoadTilesFromDirError),
    TileSetFromError(TileSetFromError),
}

#[derive(Debug, Display, Error, From)]
pub enum LoadFromTileGridsError {
    GridImageLoadError(super::grid::LoadError),
    TileSetFromError(TileSetFromError),
}

#[derive(Getters)]
#[getset(get = "pub")]
pub struct TileSet {
    pub(crate) sd_tiles: Vec<Tile>,
    pub(crate) hd_tiles: Vec<Tile>,
}

impl TileSet {

    pub fn try_from(sd_tiles: Vec<Tile>, hd_tiles: Vec<Tile>) -> Result<Self, TileSetFromError> {
        let sd_collection_kind = sd_tiles.tile_kind().map_err(|_| TileSetFromError::TileKindMismatch(TileKind::SD))?;
        if sd_collection_kind != TileKind::SD {
            return Err(TileSetFromError::WrongTileKind(TileKind::SD))
        }
        let hd_collection_kind = hd_tiles.tile_kind().map_err(|_| TileSetFromError::TileKindMismatch(TileKind::HD))?;
        if hd_collection_kind != TileKind::HD {
            return Err(TileSetFromError::WrongTileKind(TileKind::HD))
        }
        Ok(Self { sd_tiles, hd_tiles })
    }

    pub fn load_tiles_from_dir<P: AsRef<Path>>(path: P, max_tiles: usize) -> Result<Self, LoadTileSetTilesFromDirError> {
        let sd_tiles = self::load_tiles_from_dir(&path, max_tiles)?;
        let hd_tiles = self::load_tiles_from_dir(&path, max_tiles)?;
        Ok(Self::try_from(sd_tiles, hd_tiles)?)
    }

    pub fn load_from_tile_grids<P: AsRef<Path>>(sd_grid_path: P, hd_grid_path: P) -> Result<Self, LoadFromTileGridsError> {
        let sd_tiles = TileGrid::load_from_image(sd_grid_path)?.to_vec();
        let hd_tiles = TileGrid::load_from_image(hd_grid_path)?.to_vec();
        Ok(Self::try_from(sd_tiles, hd_tiles)?)
    }

}

impl SaveTilesToDir for TileSet {

    fn save_tiles_to_dir<P: AsRef<Path>>(&self, dir: P) -> Result<(), SaveTilesToDirError> {
        let sd_dir: PathBuf = [dir.as_ref(), Path::new("SD")].iter().collect();
        self.sd_tiles.save_tiles_to_dir(sd_dir)?;
        let hd_dir: PathBuf = [dir.as_ref(), Path::new("HD")].iter().collect();
        self.hd_tiles.save_tiles_to_dir(hd_dir)
    }

}