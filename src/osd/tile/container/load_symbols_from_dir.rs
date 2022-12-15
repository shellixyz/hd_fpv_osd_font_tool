
use std::collections::BTreeMap;
use std::fs::ReadDir;
use std::path::{Path, PathBuf};
use std::io::Error as IOError;

use lazy_static::lazy_static;
use regex::Regex;
use thiserror::Error;

use crate::osd::tile::container::symbol::{LoadError as SymbolLoadError, Symbol};


struct DirFilesIterator(ReadDir);

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

fn dir_files_iter<P: AsRef<Path>>(path: P) -> Result<DirFilesIterator, IOError> {
    Ok(DirFilesIterator(std::fs::read_dir(path)?))
}

#[derive(Debug, Error)]
pub enum LoadSymbolsFromDirError {
    #[error("failed to list files from directory {dir_path}: {error}")]
    DirListFiles { dir_path: PathBuf, error: IOError },
    #[error(transparent)]
    LoadError(#[from] SymbolLoadError),
    #[error("overlapping symbol files: {0} and {1}")]
    OverlappingSymbolFiles(PathBuf, PathBuf),
    #[error("symbol span {real_span} does not match span from file name {file_name}")]
    SymbolSpanDoesNotMatchName {
        file_name: PathBuf,
        real_span: usize,
    },
    #[error("no symbol found in directory: {0}")]
    NoSymbolFound(PathBuf),
    #[error("directory should contain a single kind of tile: {0}")]
    KindMismatch(PathBuf)
}

impl LoadSymbolsFromDirError {
    pub fn dir_list_files<P: AsRef<Path>>(dir_path: P, error: IOError) -> Self {
        Self::DirListFiles { dir_path: dir_path.as_ref().to_path_buf(), error }
    }

    pub fn kind_mismatch<P: AsRef<Path>>(dir_path: P) -> Self {
        Self::KindMismatch(dir_path.as_ref().to_path_buf())
    }

    pub fn no_symbol_found<P: AsRef<Path>>(dir_path: P) -> Self {
        Self::NoSymbolFound(dir_path.as_ref().to_path_buf())
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

fn identify_file_name<P: AsRef<Path>>(path: P) -> Option<SymbolDirFileType> {
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

pub fn load_symbols_from_dir<P: AsRef<Path>>(dir_path: P, max_symbols: usize) -> Result<Vec<Symbol>, LoadSymbolsFromDirError> {

    let mut symbol_files = BTreeMap::new();
    let dir_files_iter = dir_files_iter(&dir_path).map_err(|error| LoadSymbolsFromDirError::dir_list_files(&dir_path, error))?;
    for file_path in dir_files_iter {
        let file_path = file_path.map_err(|error| LoadSymbolsFromDirError::dir_list_files(&dir_path, error))?;

        if let Some(file_type) = identify_file_name(&file_path) {
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
                        SymbolLoadError::ImageReadError(image_error) => {
                            use crate::image::ReadError::*;
                            match image_error {
                                OpenError { file_path: _, error: image_error } => match image_error.kind() {
                                    std::io::ErrorKind::NotFound => None,
                                    _ => return Err(error.into()),
                                },
                                DecodeError {..} => return Err(error.into()),
                            }
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
                return Err(LoadSymbolsFromDirError::kind_mismatch(&dir_path))
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
        None => return Err(LoadSymbolsFromDirError::no_symbol_found(&dir_path)),
    };

    Ok(symbols)
}