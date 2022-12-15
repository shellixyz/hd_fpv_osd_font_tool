
use std::path::{Path, PathBuf};
use std::io::Error as IOError;
use std::ops::Deref;

use derive_more::From;
use thiserror::Error;
use image::{DynamicImage, ImageError, EncodableLayout, ImageBuffer, PixelWithColorType};
use image::io::Reader as ImageReader;


#[derive(Debug, Error, From)]
pub enum ReadError {
    #[error("failed to open image file `{file_path}`: {error}")]
    OpenError {
        file_path: PathBuf,
        error: IOError
    },
    #[error("failed to decode image file `{file_path}`: {error}")]
    DecodeError {
        file_path: PathBuf,
        error: ImageError
    }
}

impl ReadError {
    pub fn open_error<P: AsRef<Path>>(file_path: P, error: IOError) -> Self {
        Self::OpenError {
            file_path: file_path.as_ref().to_path_buf(),
            error,
        }
    }

    pub fn decode_error<P: AsRef<Path>>(path: P, error: ImageError) -> Self {
        Self::DecodeError { file_path: path.as_ref().to_path_buf(), error }
    }
}

pub fn read_image_file<P: AsRef<Path>>(path: P) -> Result<DynamicImage, ReadError> {
    let reader = ImageReader::open(&path).map_err(|error| ReadError::open_error(&path, error))?;
    reader.decode().map_err(|error| ReadError::decode_error(&path, error) )
}

#[derive(Debug, From, Error)]
#[error("failed to write image {file_path}: {error}")]
pub struct WriteError {
    file_path: PathBuf,
    error: ImageError,
}

impl WriteError {
    pub fn new<P: AsRef<Path>>(path: P, error: ImageError) -> Self {
        Self { file_path: path.as_ref().to_path_buf(), error }
    }
}

pub trait WriteImageFile {
    fn write_image_file<Q: AsRef<Path>>(&self, path: Q) -> Result<(), WriteError>;
}

impl<P, Container> WriteImageFile for ImageBuffer<P, Container>
where
    P: PixelWithColorType,
    [P::Subpixel]: EncodableLayout,
    Container: Deref<Target = [P::Subpixel]>,
{
    fn write_image_file<Q: AsRef<Path>>(&self, path: Q) -> Result<(), WriteError> {
        self.save(&path).map_err(|error| WriteError::new(&path, error) )
    }
}
