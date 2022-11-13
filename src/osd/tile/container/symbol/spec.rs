
use std::{collections::HashMap, path::PathBuf};
use std::ops::Range;
use std::path::Path;
use derive_more::{From, Deref};
use getset::CopyGetters;
use parse_int::parse;
use regex::Regex;
use lazy_static::lazy_static;
use thiserror::Error;

use crate::file::{self, Error as FileError};


#[derive(Debug, CopyGetters)]
#[getset(get_copy = "pub")]
pub struct Spec {
    start_tile_index: usize,
    span: usize
}

impl Spec {

    pub fn new(start_tile_index: usize, span: usize) -> Self {
        Self { start_tile_index, span }
    }

    pub fn end_tile_index(&self) -> usize {
        self.start_tile_index + self.span
    }

    pub fn tile_index_range(&self) -> Range<usize> {
        Range { start: self.start_tile_index, end: self.end_tile_index() }
    }

}

#[derive(Debug, Deref)]
pub struct Specs(Vec<Spec>);

impl Specs {

    pub fn load_file<P: AsRef<Path>>(path: P) -> Result<Self, LoadSpecsFileError> {
        let file = file::open(&path)?;
        let file_content: HashMap<String, String> = serde_yaml::from_reader(file)
            .map_err(|error| LoadSpecsFileError::file_structure(&path, error))?;
        lazy_static! {
            static ref SPEC_RE: Regex = Regex::new(r"\A(?P<start_tile_index>0x[\da-zA-Z]+|\d+):(?P<span>\d+)\z").unwrap();
        }
        let mut spec_vec = Vec::with_capacity(file_content.len());
        for (symbol_name, spec) in file_content {
            match SPEC_RE.captures(&spec) {
                Some(captures) => {
                    let (start_tile_index, span) = (captures.name("start_tile_index").unwrap(), captures.name("span").unwrap());
                    let spec = Spec::new(parse(start_tile_index.as_str()).unwrap(), parse(span.as_str()).unwrap());
                    spec_vec.push(spec);
                },
                None => return Err(LoadSpecsFileError::invalid_symbol_spec(&path, &symbol_name, &spec)),
            }
        }
        Ok(spec_vec.into())
    }

    pub fn find_start_index(&self, start_tile_index: usize) -> Option<&Spec> {
        self.iter().find(|sym_spec| sym_spec.start_tile_index() == start_tile_index)
    }

}

impl From<Vec<Spec>> for Specs {
    fn from(spec_vec: Vec<Spec>) -> Self {
        Self(spec_vec)
    }
}

#[derive(Debug, From, Error)]
pub enum LoadSpecsFileError {
    #[error("failed to open symbol specs file: {0}")]
    OpenError(FileError),
    #[error("failed to parse symbol specs file {file_path}: {error}")]
    FileStructureError { file_path: PathBuf, error: serde_yaml::Error },
    #[error("invalid spec for symbol {symbol_name} in file {file_path}: {spec}")]
    InvalidSymbolSpec { file_path: PathBuf, symbol_name: String, spec: String },
}

impl LoadSpecsFileError {
    pub fn file_structure<P: AsRef<Path>>(file_path: P, error: serde_yaml::Error) -> Self {
        Self::FileStructureError { file_path: file_path.as_ref().to_path_buf(), error }
    }

    pub fn invalid_symbol_spec<P: AsRef<Path>>(file_path: P, symbol_name: &str, spec: &str) -> Self {
        Self::InvalidSymbolSpec { file_path: file_path.as_ref().to_path_buf(), symbol_name: symbol_name.to_owned(), spec: spec.to_owned() }
    }
}