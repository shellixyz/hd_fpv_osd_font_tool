
use std::path::{PathBuf, Path};
use std::io::Error as IOError;

use thiserror::Error;

#[derive(Debug, Error)]
#[error("failed to create path {path}: {error}")]
pub struct CreatePathError {
    path: PathBuf,
    error: IOError,
}

impl CreatePathError {
    pub fn new<P: AsRef<Path>>(path: P, error: IOError) -> Self {
        Self { path: path.as_ref().to_path_buf(), error }
    }
}

pub fn create_path<P: AsRef<Path>>(path: P) -> Result<(), CreatePathError> {
    std::fs::create_dir_all(&path).map_err(|error| CreatePathError::new(&path, error) )
}