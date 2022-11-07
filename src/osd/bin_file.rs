
use std::path::Path;
use std::io::{Read, Seek, Write, Error as IOError};
use std::fmt::Display;
use std::error::Error;
use std::fs::File;

use close_err::Closable;
use strum::IntoEnumIterator;

use super::tile::{self, Tile, grid::TileGrid, containers::StandardSizeTileArray};

const TILE_COUNT: u32 = 256;

impl tile::Kind {

    pub fn bin_file_size_bytes(&self) -> usize {
        self.raw_rgba_size_bytes() * TILE_COUNT as usize
    }

    pub fn for_bin_file_size_bytes(bytes: usize) -> Result<Self, tile::InvalidSizeError> {
        for kind in Self::iter() {
            if bytes == kind.bin_file_size_bytes() {
                return Ok(kind);
            }
        }
        Err(tile::InvalidSizeError(bytes))
    }

}

#[derive(Debug)]
pub enum OpenError {
    IOError(IOError),
    WrongSizeError
}

impl Error for OpenError {}

impl Display for OpenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use OpenError::*;
        match self {
            IOError(io_error) => io_error.fmt(f),
            WrongSizeError => f.write_str("File size does not match a valid bin file size"),
        }
    }
}

impl From<tile::InvalidSizeError> for OpenError {
    fn from(_: tile::InvalidSizeError) -> Self {
        OpenError::WrongSizeError
    }
}

#[derive(Debug)]
pub enum SeekError {
    IOError(IOError),
    OutOfBoundsError
}

impl Display for SeekError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use SeekError::*;
        match self {
            IOError(io_error) => io_error.fmt(f),
            OutOfBoundsError => f.write_str("Cannot seek outside of the file"),
        }
    }
}

impl Error for SeekError {}

#[derive(Debug)]
pub enum SeekReadError {
    SeekError(SeekError),
    IOError(IOError)
}

impl Error for SeekReadError {}

impl Display for SeekReadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use SeekReadError::*;
        match self {
            SeekError(seek_error) => seek_error.fmt(f),
            IOError(io_error) => io_error.fmt(f),
        }
    }
}

impl From<IOError> for SeekReadError {
    fn from(io_error: IOError) -> Self {
        Self::IOError(io_error)
    }
}

#[derive(Debug)]
pub enum LoadError {
    IOError(IOError),
    WrongSizeError,
    SeekError(SeekError),
}

impl Error for LoadError {}

impl Display for LoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use LoadError::*;
        match self {
            IOError(io_error) => io_error.fmt(f),
            WrongSizeError => f.write_str("File size does not match a valid bin file size"),
            SeekError(seek_error) => seek_error.fmt(f),
        }
    }
}

impl From<OpenError> for LoadError {
    fn from(error: OpenError) -> Self {
        match error {
            OpenError::IOError(error) => Self::IOError(error),
            OpenError::WrongSizeError => Self::WrongSizeError,
        }
    }
}

impl From<SeekReadError> for LoadError {
    fn from(error: SeekReadError) -> Self {
        match error {
            SeekReadError::SeekError(error) => Self::SeekError(error),
            SeekReadError::IOError(error) => Self::IOError(error),
        }
    }
}

pub enum SeekFrom {
    Start(u32),
    End(i32),
    Current(i32)
}


pub struct BinFileReader {
    file: File,
    tile_kind: tile::Kind,
    pos: u32
}

impl BinFileReader {

    pub fn open<P: AsRef<Path> + Display>(path: P) -> Result<Self, OpenError> {
        let file = File::open(&path).map_err(OpenError::IOError)?;
        let tile_kind = tile::Kind::for_bin_file_size_bytes(file.metadata().unwrap().len() as usize)?;
        log::info!("detected {} kind of tiles in {}", tile_kind, path);
        Ok(Self { file, tile_kind, pos: 0 })
    }

    pub(crate) fn read_tile_bytes(&mut self) -> Result<tile::Bytes, IOError> {
        let mut tile_bytes = vec![0; self.tile_kind.raw_rgba_size_bytes()];
        self.file.read_exact(&mut tile_bytes)?;
        self.pos += 1;
        Ok(tile_bytes)
    }

    pub fn read_tile(&mut self) -> Result<Tile, IOError> {
        Ok(Tile::try_from(self.read_tile_bytes()?).unwrap())
    }

    pub fn seek_read_tile(&mut self, pos: SeekFrom) -> Result<Tile, SeekReadError> {
        self.seek(pos).map_err(SeekReadError::SeekError)?;
        self.read_tile().map_err(SeekReadError::IOError)
    }

    // pub fn read_symbol(&mut self, span: u32) -> Result<Symbol, IOError> {
    //     let mut tiles = Vec::with_capacity(span as usize);
    //     for _ in 0..span { tiles.push(self.read_tile()?); }
    //     Ok(Symbol::from(tiles))
    // }

    // pub fn seek_read_symbol(&mut self, pos: SeekFrom, span: u32) -> Result<Symbol, SeekReadError> {
    //     self.seek(pos).map_err(SeekReadError::SeekError)?;
    //     self.read_symbol(span).map_err(SeekReadError::IOError)
    // }

    // seek to tile position
    // returns new position if new position is inside the file or SeekError otherwise
    pub fn seek(&mut self, pos: SeekFrom) -> Result<u32, SeekError> {
        let new_pos = match pos {
            SeekFrom::Start(pos_from_start) => pos_from_start as i32,
            SeekFrom::End(pos_from_end) => TILE_COUNT as i32 - 1 + pos_from_end,
            SeekFrom::Current(pos_from_current) => self.pos as i32 + pos_from_current,
        };
        if new_pos < 0 || new_pos >= TILE_COUNT as i32 {
            return Err(SeekError::OutOfBoundsError);
        }
        let new_pos= new_pos as usize * self.tile_kind.raw_rgba_size_bytes();
        self.file.seek(std::io::SeekFrom::Start(new_pos as u64)).map_err(SeekError::IOError)?;
        self.pos = new_pos as u32;
        Ok(self.pos)
    }

    pub fn rewind(&mut self) -> Result<(), SeekError> {
        self.seek(SeekFrom::Start(0))?;
        Ok(())
    }

    pub fn is_eof(&self) -> bool {
        self.pos >= TILE_COUNT
    }

    pub fn tile_grid(&mut self) -> Result<TileGrid, SeekReadError> {
        TileGrid::try_from(self)
    }

    pub fn tile_array(&mut self) -> Result<StandardSizeTileArray, SeekReadError> {
        StandardSizeTileArray::try_from(self)
    }

}

pub fn load<P: AsRef<Path> + Display>(path: P) -> Result<StandardSizeTileArray, LoadError> {
    let mut bin_file_reader = BinFileReader::open(path)?;
    Ok(StandardSizeTileArray::try_from(&mut bin_file_reader)?)
}

#[derive(Debug)]
pub enum TileWriteError {
    IOError(IOError),
    MaximumTilesReached,
    NotEnoughTiles(BinFileWriter)
}

impl Display for TileWriteError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use TileWriteError::*;
        match self {
            IOError(io_error) => io_error.fmt(f),
            MaximumTilesReached => write!(f, "Maximum number of tiles reached: a bin file can only contain {} tiles maximum", tile::containers::STANDARD_TILE_COUNT),
            NotEnoughTiles(_) => write!(f, "Not enough tiles, a bin file must contain exactly {} tiles", tile::containers::STANDARD_TILE_COUNT),
        }
    }
}

#[derive(Debug)]
pub struct BinFileWriter {
    file: File,
    tile_count: u32
}

impl BinFileWriter {

    pub fn create<P: AsRef<Path>>(path: P) -> Result<Self, IOError> {
        Ok(Self {
            file: File::create(path)?,
            tile_count: 0
        })
    }

    pub fn write_tile(&mut self, tile: &Tile) -> Result<(), TileWriteError> {
        if self.tile_count >= tile::containers::STANDARD_TILE_COUNT as u32 {
            return Err(TileWriteError::MaximumTilesReached);
        }
        self.file.write(tile.as_raw()).map_err(TileWriteError::IOError)?;
        self.tile_count += 1;
        Ok(())
    }

    pub fn finish(self) -> Result<(), TileWriteError> {
        if self.tile_count < tile::containers::STANDARD_TILE_COUNT as u32 {
            return Err(TileWriteError::NotEnoughTiles(self));
        }
        self.file.close().map_err(TileWriteError::IOError)?;
        Ok(())
    }

}