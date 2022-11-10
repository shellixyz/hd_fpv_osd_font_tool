
use std::collections::BTreeMap;
use std::fmt::Display;
use std::ops::Index;
use std::path::{Path, PathBuf};
use std::io::Error as IOError;

use derive_more::{Error, Display, From};
use getset::Getters;
use image::ImageError;
use regex::Regex;
use lazy_static::lazy_static;
use strum::IntoEnumIterator;
use tap::Tap;

use crate::osd::bin_file::{BinFileWriter, self, TileWriteError, FillRemainingSpaceError};
use crate::osd::symbol::{Symbol, spec::{Spec as SymbolSpec, Specs as SymbolSpecs}};

use super::{Tile, Kind as TileKind};
use super::LoadError as TileLoadError;
use crate::osd::symbol::LoadError as SymbolLoadError;
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
            let path: PathBuf = [path.as_ref(), Path::new(&format!("{:03}.png", index))].iter().collect();
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

pub trait IterUniqTileKind {
    fn tile_kind(&mut self) -> Result<super::Kind, TileKindError>;
}

impl<'a, T> IterUniqTileKind for T
where
    T: Iterator<Item = &'a Tile>
{
    fn tile_kind(&mut self) -> Result<super::Kind, TileKindError> {
        let first_tile_kind = self.next().ok_or(TileKindError::EmptyContainer)?.kind();
        if ! self.all(|tile| tile.kind() == first_tile_kind) {
            return Err(TileKindError::MultipleTileKinds)
        }
        Ok(first_tile_kind)
    }
}

pub trait UniqTileKind {
    fn tile_kind(&self) -> Result<super::Kind, TileKindError>;
}

impl UniqTileKind for &[Tile] {
    fn tile_kind(&self) -> Result<super::Kind, TileKindError> {
        self.iter().tile_kind()
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

pub trait SaveToBinFile {
    fn save_to_bin_file<P: AsRef<Path>>(&self, path: P) -> Result<(), SaveTilesToBinFileError>;
}

impl SaveToBinFile for &[Tile] {
    fn save_to_bin_file<P: AsRef<Path>>(&self, path: P) -> Result<(), SaveTilesToBinFileError> {
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

impl SaveToBinFile for Vec<Tile> {
    fn save_to_bin_file<P: AsRef<Path>>(&self, path: P) -> Result<(), SaveTilesToBinFileError> {
        self.as_slice().save_to_bin_file(path)
    }
}

pub trait SaveTilesToBinFile {
    fn save_tiles_to_bin_file<P: AsRef<Path>>(&self, path: P) -> Result<(), SaveTilesToBinFileError>;
}

impl SaveTilesToBinFile for TileGrid {
    fn save_tiles_to_bin_file<P: AsRef<Path>>(&self, path: P) -> Result<(), SaveTilesToBinFileError> {
        self.as_slice().save_to_bin_file(path)
    }
}

pub trait SaveToBinFiles {
    fn save_to_bin_files<P: AsRef<Path>>(&self, path1: P, path2: P) -> Result<(), SaveTilesToBinFileError>;
}

impl SaveToBinFiles for &[Tile] {
    fn save_to_bin_files<P: AsRef<Path>>(&self, path1: P, path2: P) -> Result<(), SaveTilesToBinFileError> {
        (&self[0..bin_file::TILE_COUNT]).save_to_bin_file(path1)?;
        (&self[bin_file::TILE_COUNT..2 * bin_file::TILE_COUNT]).save_to_bin_file(path2)
    }
}

impl SaveToBinFiles for Vec<Tile> {
    fn save_to_bin_files<P: AsRef<Path>>(&self, path1: P, path2: P) -> Result<(), SaveTilesToBinFileError> {
        self.as_slice().save_to_bin_files(path1, path2)
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
        use LoadTilesFromDirError::*;
        match self {
            LoadError(load_error) => load_error.fmt(f),
            KindMismatchError => f.write_str("directory contains different kinds of tiles"),
            NoTileFound => f.write_str("no tile found"),
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
        use TileSetFromError::*;
        let sd_collection_kind = sd_tiles.tile_kind().map_err(|_| TileKindMismatch(TileKind::SD))?;
        if sd_collection_kind != TileKind::SD {
            return Err(WrongTileKind(TileKind::SD))
        }
        let hd_collection_kind = hd_tiles.tile_kind().map_err(|_| TileKindMismatch(TileKind::HD))?;
        if hd_collection_kind != TileKind::HD {
            return Err(WrongTileKind(TileKind::HD))
        }
        Ok(Self { sd_tiles, hd_tiles })
    }

    pub fn load_tiles_from_dir<P: AsRef<Path>>(path: P, max_tiles: usize) -> Result<Self, LoadTileSetTilesFromDirError> {
        let sd_tiles = self::load_tiles_from_dir(TileKind::SD.set_dir_path(&path), max_tiles)?;
        let hd_tiles = self::load_tiles_from_dir(TileKind::HD.set_dir_path(&path), max_tiles)?;
        Ok(Self::try_from(sd_tiles, hd_tiles)?)
    }

    pub fn load_from_tile_grids<P: AsRef<Path>>(sd_grid_path: P, hd_grid_path: P) -> Result<Self, LoadFromTileGridsError> {
        let sd_tiles = TileGrid::load_from_image(sd_grid_path)?.to_vec();
        let hd_tiles = TileGrid::load_from_image(hd_grid_path)?.to_vec();
        Ok(Self::try_from(sd_tiles, hd_tiles)?)
    }

}

impl Index<TileKind> for TileSet {
    type Output = Vec<Tile>;

    fn index(&self, tile_kind: TileKind) -> &Self::Output {
        match tile_kind {
            TileKind::SD => &self.sd_tiles,
            TileKind::HD => &self.hd_tiles,
        }
    }
}

impl SaveTilesToDir for TileSet {

    fn save_tiles_to_dir<P: AsRef<Path>>(&self, path: P) -> Result<(), SaveTilesToDirError> {
        for tile_kind in TileKind::iter() {
            self[tile_kind].save_tiles_to_dir(tile_kind.set_dir_path(&path))?;
        }
        Ok(())
    }

}

pub trait IntoTilesVec {
    fn into_tiles_vec(self) -> Vec<Tile>;
}

impl IntoTilesVec for Vec<Symbol> {
    fn into_tiles_vec(self) -> Vec<Tile> {
        self.into_iter().flat_map(Symbol::into_tiles).collect()
    }
}

pub trait AsTilesVec<'a> {
    fn as_tiles_vec(&'a self) -> Vec<&'a Tile>;
}

impl<'a> AsTilesVec<'a> for &[Symbol] {
    fn as_tiles_vec(&'a self) -> Vec<&'a Tile> {
        self.tiles_iter().collect()
    }
}

#[derive(Debug, Error, Display, From)]
pub enum SaveSymbolsToDirError {
    IOError(IOError),
    ImageError(ImageError),
}

pub trait SaveSymbolsToDir {
    fn save_to_dir<P: AsRef<Path>>(&self, path: P) -> Result<(), SaveSymbolsToDirError>;
}

impl<T> SaveSymbolsToDir for T
where
    for<'any> &'any T: IntoIterator<Item = &'any Symbol>,
{
    fn save_to_dir<P: AsRef<Path>>(&self, path: P) -> Result<(), SaveSymbolsToDirError> {
        std::fs::create_dir_all(&path)?;
        let mut tile_index = 0;
        for symbol in self {
            let file_name = match symbol.span() {
                1 => format!("{tile_index:03}.png"),
                span => format!("{tile_index:03}-{:03}.png", tile_index + span - 1)
            };
            let file_path: PathBuf = [path.as_ref(), Path::new(&file_name)].iter().collect();
            symbol.generate_image().save(file_path)?;
            tile_index += symbol.span();
        }
        Ok(())
    }
}

pub struct SymbolTilesIter<'a> {
    symbols: &'a [Symbol],
    symbol_index: usize,
    symbol_tile_index: usize
}

impl<'a> SymbolTilesIter<'a> {
    pub fn new(symbols: &'a [Symbol]) -> Self {
        Self { symbols, symbol_index: 0, symbol_tile_index: 0 }
    }
}

impl<'a> Iterator for SymbolTilesIter<'a> {
    type Item = &'a Tile;

    fn next(&mut self) -> Option<Self::Item> {
        if self.symbol_index == self.symbols.len() {
            return None;
        }
        let symbol_tiles = self.symbols[self.symbol_index].tiles();
        let tile = &symbol_tiles[self.symbol_tile_index];
        if self.symbol_tile_index == symbol_tiles.len() - 1 {
            self.symbol_tile_index = 0;
            self.symbol_index += 1;
        } else {
            self.symbol_tile_index += 1;
        }
        Some(tile)
    }
}

pub trait IntoSymbolsTilesIter {
    fn tiles_iter(&self) -> SymbolTilesIter;
}

impl IntoSymbolsTilesIter for &[Symbol] {
    fn tiles_iter(&self) -> SymbolTilesIter {
        SymbolTilesIter::new(self)
    }
}

impl UniqTileKind for &[Symbol] {
    fn tile_kind(&self) -> Result<super::Kind, TileKindError> {
        self.tiles_iter().tile_kind()
    }
}

pub trait SymbolSpecsExt {
    fn find_start_index(&self, start_tile_index: usize) -> Option<&SymbolSpec>;
}

impl SymbolSpecsExt for SymbolSpecs {
    fn find_start_index(&self, start_tile_index: usize) -> Option<&SymbolSpec> {
        self.iter().find(|sym_spec| sym_spec.start_tile_index() == start_tile_index)
    }
}

pub trait ToSymbols {
    fn to_symbols(&self, specs: SymbolSpecs) -> Result<Vec<Symbol>, TileKindError>;
}

impl ToSymbols for &[Tile] {
    fn to_symbols(&self, specs: SymbolSpecs) -> Result<Vec<Symbol>, TileKindError> {
        let mut tile_index = 0;
        let mut symbols = vec![];
        while tile_index < self.len() {
            let symbol = match specs.find_start_index(tile_index) {
                Some(sym_spec) =>
                    Symbol::try_from(Vec::from(&self[sym_spec.tile_index_range()]))?
                        .tap(|_| tile_index += sym_spec.span()),
                None =>
                    Symbol::from(self[tile_index].clone())
                        .tap(|_| tile_index += 1),
            };
            symbols.push(symbol);
        }
        Ok(symbols)
    }
}

impl ToSymbols for Vec<Tile> {
    fn to_symbols(&self, specs: SymbolSpecs) -> Result<Vec<Symbol>, TileKindError> {
        self.as_slice().to_symbols(specs)
    }
}

mod inner {
    use std::fs::ReadDir;
    use std::path::{Path, PathBuf};
    use std::io::Error as IOError;

    pub(super) struct DirFilesIterator(ReadDir);

    impl Iterator for DirFilesIterator {
        type Item = Result<PathBuf, IOError>;

        fn next(&mut self) -> Option<Self::Item> {
            match self.0.next()? {
                Ok(entry) => {
                    let path = entry.path();
                    if path.is_file() {
                        Some(Ok(path))
                    } else {
                        None
                    }
                },
                Err(error) => Some(Err(error))
            }
        }
    }

    pub(super) fn dir_files_iterator<P: AsRef<Path>>(path: P) -> Result<DirFilesIterator, IOError> {
        Ok(DirFilesIterator(std::fs::read_dir(path)?))
    }

}

#[derive(Debug, Error, From)]
pub enum LoadSymbolsFromDirError {
    IOError(IOError),
    LoadError(SymbolLoadError),
    OverlappingSymbolFiles(PathBuf, PathBuf),
    SymbolSpanDoesNotMatchName {
        file_name: PathBuf,
        real_span: usize,
    },
    NoSymbolFound,
    KindMismatchError
}

impl Display for LoadSymbolsFromDirError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use LoadSymbolsFromDirError::*;
        match self {
            LoadError(load_error) => load_error.fmt(f),
            KindMismatchError => f.write_str("directory contains different kinds of tiles"),
            IOError(error) => error.fmt(f),
            OverlappingSymbolFiles(file1, file2) => write!(f, "overlapping symbol files: `{}` and `{}`", file1.to_string_lossy(), file2.to_string_lossy()),
            SymbolSpanDoesNotMatchName { real_span, file_name } => write!(f, "symbol span {real_span} does not match span from file name {}", file_name.to_string_lossy()),
            NoSymbolFound => f.write_str("no symbol found"),
        }
    }
}

enum SymbolDirFileType {
    Tile {
        index: usize
    },
    Symbol {
        start_index: usize,
        end_index: usize
    }
}

impl SymbolDirFileType {
    fn start_index(&self) -> usize {
        match self {
            SymbolDirFileType::Tile { index } => *index,
            SymbolDirFileType::Symbol { start_index, .. } => *start_index,
        }
    }

    fn span(&self) -> usize {
        match self {
            SymbolDirFileType::Tile { .. } => 1,
            SymbolDirFileType::Symbol { start_index, end_index } => end_index - start_index + 1,
        }
    }
}

pub fn load_symbols_from_dir<P: AsRef<Path>>(dir_path: P, max_symbols: usize) -> Result<Vec<Symbol>, LoadSymbolsFromDirError> {

    fn identify_file<P: AsRef<Path>>(path: P) -> Option<SymbolDirFileType> {
        lazy_static! {
            static ref FILE_NAME_RE: Regex = Regex::new(r"\A(?P<start_index>\d{3})(?:-(?P<end_index>\d{3}))?\.").unwrap();
        }

        if let Some(captures) = FILE_NAME_RE.captures(path.as_ref().file_name().unwrap().to_string_lossy().to_string().as_str()) {
            let start_index = captures.name("start_index").unwrap().as_str().parse().expect("failed to parse start index");
            match captures.name("end_index") {
                Some(end_index) => {
                    let end_index = end_index.as_str().parse().expect("failed to parse end index");
                    Some(SymbolDirFileType::Symbol { start_index, end_index })
                },
                None => Some(SymbolDirFileType::Tile { index: start_index }),
            }
        } else {
            None
        }
    }

    let mut symbol_files = BTreeMap::new();
    for file_path in inner::dir_files_iterator(&dir_path)? {
        let file_path = file_path?;

        if let Some(file_type) = identify_file(&file_path) {
            use std::collections::btree_map;
            match symbol_files.entry(file_type.start_index()) {
                btree_map::Entry::Vacant(entry) => { entry.insert((file_path, file_type)); },
                btree_map::Entry::Occupied(entry) => {
                    let (existing_path, _) = entry.get();
                    return Err(LoadSymbolsFromDirError::OverlappingSymbolFiles(file_path, existing_path.clone()));
                },
            }
        }
    }

    let mut symbols = Vec::with_capacity(symbol_files.len());
    let mut tile_kind = None;
    let mut tile_index = 0;
    let mut previous_symbol_file_path: Option<&PathBuf> = None;
    for _symbol_index in 0..max_symbols {

        let symbol = match symbol_files.get(&tile_index) {
            Some((file_path, file_type)) => {

                if file_type.start_index() < tile_index {
                    return Err(LoadSymbolsFromDirError::OverlappingSymbolFiles(previous_symbol_file_path.unwrap().clone(), file_path.clone()))
                }

                previous_symbol_file_path = Some(file_path);

                match Symbol::load_image_file(file_path) {
                    Ok(loaded_symbol) => {

                        if loaded_symbol.span() != file_type.span() {
                            return Err(LoadSymbolsFromDirError::SymbolSpanDoesNotMatchName { file_name: file_path.clone(), real_span: loaded_symbol.span() })
                        }

                        Some(loaded_symbol)
                    }
                    Err(error) => match &error {
                        SymbolLoadError::IOError(io_error) =>
                            match io_error.kind() {
                                std::io::ErrorKind::NotFound => None,
                                _ => return Err(error.into()),
                            },
                        _ => return Err(error.into())
                    },
                }

            },
            None => None,
        };

        match (&symbol, &tile_kind) {

            // first loaded tile: record the kind of tile
            (Some(symbol), None) => {
                log::info!("detected {} kind of tiles in {}", symbol.tile_kind(), dir_path.as_ref().to_string_lossy());
                tile_kind = Some(symbol.tile_kind());
            },

            // we have already loaded a tile before, check that the new tile kind is matching what had recorded
            (Some(symbol), Some(tile_kind)) => if symbol.tile_kind() != *tile_kind {
                return Err(LoadSymbolsFromDirError::KindMismatchError)
            },

            _ => {}

        }

        if let Some(symbol) = &symbol {
            tile_index += symbol.span();
        } else {
            tile_index += 1;
        }

        symbols.push(symbol);
    }

    let symbols = match tile_kind {
        Some(tile_kind) => {
            let last_some_index = symbols.iter().rposition(Option::is_some).unwrap();
            symbols[0..=last_some_index].iter().map(|symbol| symbol.clone().unwrap_or_else(|| Symbol::new(tile_kind))).collect()
        }
        None => return Err(LoadSymbolsFromDirError::NoSymbolFound),
    };

    Ok(symbols)
}