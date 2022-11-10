
use std::collections::HashMap;
use std::error::Error;
use std::fmt::Display;
use std::ops::Range;
use std::io::Error as IOError;
use std::path::Path;
use derive_more::{From, Deref};
use getset::CopyGetters;
use parse_int::parse;
use regex::Regex;
use lazy_static::lazy_static;

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

impl From<Vec<Spec>> for Specs {
    fn from(spec_vec: Vec<Spec>) -> Self {
        Self(spec_vec)
    }
}

#[derive(Debug, From)]
pub enum LoadSpecsFileError {
    IOError(IOError),
    FileStructureError(serde_yaml::Error),
    InvalidSymbolSpec(String),
}

impl Error for LoadSpecsFileError {}

impl Display for LoadSpecsFileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use LoadSpecsFileError::*;
        match self {
            FileStructureError(error) => error.fmt(f),
            InvalidSymbolSpec(spec) => write!(f, "invalid symbol spec: `{spec}`"),
            IOError(error) => error.fmt(f),
        }
    }
}

pub fn load_specs_file<P: AsRef<Path>>(path: P) -> Result<Specs, LoadSpecsFileError> {
    let file = std::fs::File::open(path)?;
    let file_content: HashMap<String, String> = serde_yaml::from_reader(file)?;
    lazy_static! {
        static ref SPEC_RE: Regex = Regex::new(r"\A(?P<start_tile_index>0x[\da-zA-Z]+|\d+):(?P<span>\d+)\z").unwrap();
    }
    let mut spec_vec = Vec::with_capacity(file_content.len());
    for (_, spec) in file_content {
        match SPEC_RE.captures(&spec) {
            Some(captures) => {
                let (start_tile_index, span) = (captures.name("start_tile_index").unwrap(), captures.name("span").unwrap());
                let spec = Spec::new(parse(start_tile_index.as_str()).unwrap(), parse(span.as_str()).unwrap());
                spec_vec.push(spec);
            },
            None => panic!("no match"),
        }
    }
    Ok(spec_vec.into())
}