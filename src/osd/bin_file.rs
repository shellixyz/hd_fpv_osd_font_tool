
use std::path::Path;
use std::io::{Read, Seek, Write, Error as IOError};
use std::fmt::Display;
use std::fs::File;

use close_err::Closable;
use derive_more::{From, Error};
use getset::Getters;
use strum::{IntoEnumIterator, Display};

use super::tile::{self, Tile, Kind as TileKind};

pub const TILE_COUNT: usize = 256;

impl tile::Kind {

    pub fn bin_file_size_bytes(&self) -> usize {
        self.raw_rgba_size_bytes() * TILE_COUNT
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

#[derive(Debug, From, Error)]
pub enum OpenError {
    IOError(IOError),
    #[from(ignore)]
    InvalidSizeError
}

impl Display for OpenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use OpenError::*;
        match self {
            IOError(io_error) => io_error.fmt(f),
            InvalidSizeError => f.write_str("File size does not match a valid bin file size"),
        }
    }
}

impl From<tile::InvalidSizeError> for OpenError {
    fn from(_: tile::InvalidSizeError) -> Self {
        OpenError::InvalidSizeError
    }
}

#[derive(Debug, Error)]
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

#[derive(Debug, From, Error, Display)]
pub enum SeekReadError {
    SeekError(SeekError),
    IOError(IOError)
}

#[derive(Debug, From, Error)]
pub enum LoadError {
    IOError(IOError),
    OpenError(OpenError),
    #[from(ignore)]
    WrongSizeError,
}

impl Display for LoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use LoadError::*;
        match self {
            IOError(error) => error.fmt(f),
            OpenError(error) => error.fmt(f),
            WrongSizeError => f.write_str("File size does not match a valid bin file size"),
        }
    }
}

pub enum SeekFrom {
    Start(usize),
    End(isize),
    Current(isize)
}

#[derive(Getters)]
pub struct BinFileReader {
    file: File,

    #[getset(get = "pub")]
    tile_kind: tile::Kind,

    #[getset(get = "pub")]
    pos: usize
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
    pub fn seek(&mut self, pos: SeekFrom) -> Result<usize, SeekError> {
        let new_pos = match pos {
            SeekFrom::Start(pos_from_start) => pos_from_start as isize,
            SeekFrom::End(pos_from_end) => TILE_COUNT as isize - 1 + pos_from_end,
            SeekFrom::Current(pos_from_current) => self.pos as isize + pos_from_current,
        };
        if new_pos < 0 || new_pos >= TILE_COUNT as isize {
            return Err(SeekError::OutOfBoundsError);
        }
        let new_pos= new_pos * self.tile_kind.raw_rgba_size_bytes() as isize;
        self.file.seek(std::io::SeekFrom::Start(new_pos as u64)).map_err(SeekError::IOError)?;
        self.pos = new_pos as usize;
        Ok(self.pos)
    }

    pub fn rewind(&mut self) -> Result<(), SeekError> {
        self.seek(SeekFrom::Start(0))?;
        Ok(())
    }

    pub fn is_eof(&self) -> bool {
        self.pos >= TILE_COUNT
    }

    // pub fn tile_grid(&mut self) -> Result<StandardSizeGrid, SeekReadError> {
    //     StandardSizeGrid::try_from(self)
    // }

    // pub fn tile_array(&mut self) -> Result<StandardSizeArray, SeekReadError> {
    //     StandardSizeArray::try_from(self)
    // }

    pub fn read_tiles(self) -> Result<Vec<Tile>, IOError> {
        let mut tiles = vec![];
        for tile in self {
            tiles.push(tile?);
        }
        Ok(tiles)
    }

}

pub struct BinFileReaderIterator(BinFileReader);

impl Iterator for BinFileReaderIterator {
    type Item = Result<Tile, IOError>;

    fn next(&mut self) -> Option<Self::Item> {
        if *self.0.pos() >= TILE_COUNT {
            return None;
        }
        Some(self.0.read_tile())
    }
}

impl IntoIterator for BinFileReader {
    type Item = Result<Tile, IOError>;

    type IntoIter = BinFileReaderIterator;

    fn into_iter(self) -> Self::IntoIter {
        BinFileReaderIterator(self)
    }
}

pub fn load<P: AsRef<Path> + Display>(path: P) -> Result<Vec<Tile>, LoadError> {
    Ok(BinFileReader::open(path)?.read_tiles()?)
}

#[derive(Debug, From)]
pub enum TileWriteError {
    IOError(IOError),
    #[from(ignore)]
    TileKindMismatchError {
        written_kind: TileKind,
        writing_kind: TileKind
    },
    #[from(ignore)]
    MaximumTilesReached,
    NotEnoughTiles(BinFileWriter)
}

impl std::error::Error for TileWriteError {}

impl Display for TileWriteError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use TileWriteError::*;
        match self {
            IOError(io_error) => io_error.fmt(f),
            MaximumTilesReached => write!(f, "Maximum number of tiles reached: a bin file can only contain {} tiles maximum", TILE_COUNT),
            NotEnoughTiles(_) => write!(f, "Not enough tiles, a bin file must contain exactly {} tiles", TILE_COUNT),
            TileKindMismatchError { written_kind, writing_kind } =>
                write!(f, "Already written tiles of kind {} and trying to now write tiles of kind {}", written_kind, writing_kind),
        }
    }
}

#[derive(Debug, Error, From)]
pub enum FillRemainingSpaceError {
    TileWrite(TileWriteError),
    #[from(ignore)]
    Empty
}

impl Display for FillRemainingSpaceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use FillRemainingSpaceError::*;
        match self {
            TileWrite(error) => error.fmt(f),
            Empty => f.write_str("bin file is empty, cannot determine tile kind to write"),
        }
    }
}

#[derive(Debug)]
pub struct BinFileWriter {
    file: File,
    tile_count: usize,
    tile_kind: Option<TileKind>,
}

impl BinFileWriter {

    pub fn create<P: AsRef<Path>>(path: P) -> Result<Self, IOError> {
        Ok(Self {
            file: File::create(path)?,
            tile_count: 0,
            tile_kind: None
        })
    }

    pub fn write_tile(&mut self, tile: &Tile) -> Result<(), TileWriteError> {
        if self.tile_count >= TILE_COUNT {
            return Err(TileWriteError::MaximumTilesReached);
        }
        match self.tile_kind {
            Some(tile_kind) => if tile_kind != tile.kind() {
                return Err(TileWriteError::TileKindMismatchError { written_kind: tile_kind, writing_kind: tile.kind() })
            },
            None => self.tile_kind = Some(tile.kind()),
        }
        self.file.write_all(tile.as_raw())?;
        self.tile_count += 1;
        Ok(())
    }

    pub fn fill_remaining_space(&mut self) -> Result<(), FillRemainingSpaceError> {
        match self.tile_kind {
            Some(tile_kind) => {
                let transparent_tile = Tile::new(tile_kind);
                for _ in self.tile_count..TILE_COUNT {
                    self.write_tile(&transparent_tile)?;
                }
            },
            None => return Err(FillRemainingSpaceError::Empty),
        }
        Ok(())
    }

    pub fn finish(self) -> Result<(), TileWriteError> {
        if self.tile_count < TILE_COUNT {
            return Err(TileWriteError::NotEnoughTiles(self));
        }
        self.file.close().map_err(TileWriteError::IOError)?;
        Ok(())
    }

}