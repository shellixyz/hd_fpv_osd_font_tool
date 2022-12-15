
use std::path::{Path, PathBuf};
use std::io::{Error as IOError, Read, Seek, Write};

use derive_more::From;
use thiserror::Error;
use getset::Getters;
use strum::{IntoEnumIterator, Display};
use fs_err::File;

use super::tile::{
    self,
    Tile,
    Kind as TileKind,
    grid::Grid as TileGrid,
    container::{
        into_tile_grid::IntoTileGrid,
        tile_set::TileSet,
        uniq_tile_kind::UniqTileKind,
    },
};

use crate::osd::tile::InvalidSizeError;


pub const TILE_COUNT: usize = 256;

impl TileKind {

    pub fn bin_file_size_bytes(&self) -> usize {
        self.raw_rgba_size_bytes() * TILE_COUNT
    }

    pub fn for_bin_file_size_bytes(bytes: u64) -> Result<Self, tile::InvalidSizeError> {
        for kind in Self::iter() {
            if bytes == kind.bin_file_size_bytes() as u64 {
                return Ok(kind);
            }
        }
        Err(tile::InvalidSizeError(bytes))
    }

}

#[derive(Debug, From, Error)]
pub enum OpenError {
    #[error(transparent)]
    FileError(IOError),
    #[from(ignore)]
    #[error("file {file_path} has a size ({size}B) which does not match a valid bin file size")]
    InvalidSizeError {
        file_path: PathBuf,
        size: u64
    }
}

impl OpenError {
    pub fn invalid_size<P: AsRef<Path>>(file_path: P, size: u64) -> Self {
        Self::InvalidSizeError { file_path: file_path.as_ref().to_path_buf(), size }
    }
}

#[derive(Debug, Error, From)]
pub enum SeekError {
    #[error(transparent)]
    FileError(IOError),
    #[error("cannot seek outside of the file ({file_path}) new position would be {new_pos}")]
    OutOfBoundsError {
        file_path: PathBuf,
        new_pos: isize
    }
}

impl SeekError {
    pub fn out_of_bounds<P: AsRef<Path>>(file_path: P, new_pos: isize) -> Self {
        Self::OutOfBoundsError { file_path: file_path.as_ref().to_path_buf(), new_pos }
    }
}

#[derive(Debug, From, Error, Display)]
pub enum SeekReadError {
    SeekError(SeekError),
    FileError(IOError)
}

#[derive(Debug, From, Error)]
pub enum LoadError {
    #[error(transparent)]
    OpenError(OpenError),
    #[error(transparent)]
    ReadError(IOError),
    #[error("tile kind loaded from {file_path} does not match requested: load {loaded}, requested {requested}")]
    LoadedTileKindDoesNotMatchRequested { file_path: PathBuf, loaded: TileKind, requested: TileKind },
    #[error("File size does not match a valid bin file size: file {file_path}, size {size}B")]
    WrongSizeError { file_path: PathBuf, size: u64 },
}

impl LoadError {
    pub fn tile_kind_mismatch<P: AsRef<Path>>(file_path: P, loaded: TileKind, requested: TileKind) -> Self {
        Self::LoadedTileKindDoesNotMatchRequested { file_path: file_path.as_ref().to_path_buf(), loaded, requested }
    }

    pub fn because_file_is_missing(&self) -> bool {
        matches!(self,
            LoadError::OpenError(OpenError::FileError(file_error))
                if matches!(file_error.kind(), std::io::ErrorKind::NotFound)
        )
    }
}

pub enum SeekFrom {
    Start(usize),
    End(isize),
    Current(isize)
}

#[derive(Getters)]
pub struct BinFileReader {
    file_path: PathBuf,
    file: File,

    #[getset(get = "pub")]
    tile_kind: tile::Kind,

    #[getset(get = "pub")]
    pos: usize
}

impl BinFileReader {

    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, OpenError> {
        let file = File::open(&path)?;
        let tile_kind = tile::Kind::for_bin_file_size_bytes(file.metadata().unwrap().len())
            .map_err(|error| {
                let InvalidSizeError(size) = error;
                OpenError::invalid_size(&path, size)
            })?;
        log::info!("detected {} kind of tiles in {}", tile_kind, path.as_ref().to_string_lossy());
        Ok(Self { file, file_path: path.as_ref().to_path_buf(), tile_kind, pos: 0 })
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
        self.read_tile().map_err(SeekReadError::FileError)
    }

    // seek to tile position
    // returns new position if new position is inside the file or SeekError otherwise
    pub fn seek(&mut self, pos: SeekFrom) -> Result<usize, SeekError> {
        let new_pos = match pos {
            SeekFrom::Start(pos_from_start) => pos_from_start as isize,
            SeekFrom::End(pos_from_end) => TILE_COUNT as isize - 1 + pos_from_end,
            SeekFrom::Current(pos_from_current) => self.pos as isize + pos_from_current,
        };
        if new_pos < 0 || new_pos >= TILE_COUNT as isize {
            return Err(SeekError::out_of_bounds(&self.file_path, new_pos));
        }
        let new_pos= new_pos * self.tile_kind.raw_rgba_size_bytes() as isize;
        self.file.seek(std::io::SeekFrom::Start(new_pos as u64))?;
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

    pub fn into_tile_grid(self) -> Result<TileGrid, SeekReadError> {
        Ok(self.read_tiles()?.into_tile_grid())
    }

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

pub fn load<P: AsRef<Path>>(path: P) -> Result<Vec<Tile>, LoadError> {
    Ok(BinFileReader::open(path)?.read_tiles()?)
}

pub fn load_norm<P: AsRef<Path>>(dir: P, tile_kind: TileKind, ident: &Option<&str>, part: FontPart) -> Result<Vec<Tile>, LoadError> {
    let file_path = normalized_file_path(&dir, tile_kind, ident, part);
    let tiles = load(&file_path)?;
    let loaded_tile_kind = tiles.tile_kind().unwrap();
    if loaded_tile_kind != tile_kind {
        return Err(LoadError::tile_kind_mismatch(&file_path, loaded_tile_kind, tile_kind));
    }
    Ok(tiles)
}

pub fn load_extended<P: AsRef<Path>>(base_path: P, ext_path: P) -> Result<Vec<Tile>, LoadError> {
    let base_tiles = load(&base_path)?;
    let base_tile_kind = base_tiles.tile_kind().expect("should not fail for collections from bin files");
    let ext_tiles = load(&ext_path)?;
    let ext_tile_kind = ext_tiles.tile_kind().expect("should not fail for collections from bin files");
    if ext_tile_kind != base_tile_kind {
        return Err(LoadError::tile_kind_mismatch(&ext_path, ext_tile_kind, base_tile_kind))
    }
    let tiles = [base_tiles, ext_tiles].into_iter().flatten().collect();
    Ok(tiles)
}

pub fn load_extended_check_kind<P: AsRef<Path>>(base_path: P, ext_path: P, requested_tile_kind: TileKind) -> Result<Vec<Tile>, LoadError> {

    fn check_tile_kind<P: AsRef<Path>>(file_path: P, tiles: &[Tile], expected_tile_kind: TileKind) -> Result<(), LoadError> {
        let tile_kind = tiles.tile_kind().expect("should not fail for collections from bin files");
        if tile_kind != expected_tile_kind {
            return Err(LoadError::tile_kind_mismatch(file_path, tile_kind, expected_tile_kind))
        }
        Ok(())
    }

    let base_tiles = load(&base_path)?;
    check_tile_kind(&base_path, &base_tiles, requested_tile_kind)?;
    let ext_tiles = load(&ext_path)?;
    check_tile_kind(&ext_path, &ext_tiles, requested_tile_kind)?;
    let tiles = [base_tiles, ext_tiles].into_iter().flatten().collect();
    Ok(tiles)
}

pub enum FontPart {
    Base,
    Ext
}

pub fn normalized_file_name(tile_kind: TileKind, ident: &Option<&str>, part: FontPart) -> PathBuf {
    let font_part_str = match part {
        FontPart::Base => "",
        FontPart::Ext => "_2",
    };
    let tile_kind_str = match tile_kind {
        TileKind::SD => "",
        TileKind::HD => "_hd",
    };
    let ident = match ident {
        Some(ident) => format!("_{ident}"),
        None => "".to_owned(),
    };
    PathBuf::from(format!("font{ident}{tile_kind_str}{font_part_str}.bin"))
}

pub fn normalized_file_path<P: AsRef<Path>>(dir: P, tile_kind: TileKind, ident: &Option<&str>, part: FontPart) -> PathBuf {
    [dir.as_ref().to_path_buf(), normalized_file_name(tile_kind, ident, part)].into_iter().collect()
}

pub fn load_base_norm<P: AsRef<Path>>(dir: P, tile_kind: TileKind, ident: &Option<&str>) -> Result<Vec<Tile>, LoadError> {
    load_norm(dir, tile_kind, ident, FontPart::Base)
}

pub fn load_extended_norm<P: AsRef<Path>>(dir: P, tile_kind: TileKind, ident: &Option<&str>) -> Result<Vec<Tile>, LoadError> {
    let base_tiles = load_norm(&dir, tile_kind, ident, FontPart::Base)?;
    let ext_tiles = load_norm(&dir, tile_kind, ident, FontPart::Ext)?;
    let tiles = [base_tiles, ext_tiles].into_iter().flatten().collect();
    Ok(tiles)
}

impl TileSet {

    pub fn load_bin_files<P: AsRef<Path>>(sd_path: P, sd_2_path: P, hd_path: P, hd_2_path: P) -> Result<Self, LoadError> {
        let sd_tiles = load_extended_check_kind(&sd_path, &sd_2_path, TileKind::SD)?;
        let hd_tiles = load_extended_check_kind(&hd_path, &hd_2_path, TileKind::HD)?;
        Ok(Self { sd_tiles, hd_tiles })
    }

    pub fn load_bin_files_norm<P: AsRef<Path>>(dir: P, ident: &Option<&str>) -> Result<Self, LoadError> {
        let sd_tiles = load_extended_norm(&dir, TileKind::SD, ident)?;
        let hd_tiles = load_extended_norm(&dir, TileKind::HD, ident)?;
        Ok(Self { sd_tiles, hd_tiles })
    }

}

pub fn load_set<P: AsRef<Path>>(sd_path: P, sd_2_path: P, hd_path: P, hd_2_path: P) -> Result<TileSet, LoadError> {
    TileSet::load_bin_files(sd_path, sd_2_path, hd_path, hd_2_path)
}

pub fn load_set_norm<P: AsRef<Path>>(dir: P, ident: &Option<&str>) -> Result<TileSet, LoadError> {
    TileSet::load_bin_files_norm(dir, ident)
}

#[derive(Debug, From, Error)]
pub enum TileWriteError {
    #[error(transparent)]
    FileError(IOError),
    #[from(ignore)]
    #[error("Already written tiles of kind {written_kind} and trying to now write tiles of kind {writing_kind}")]
    TileKindMismatchError {
        written_kind: TileKind,
        writing_kind: TileKind
    },
    #[from(ignore)]
    #[error("Maximum number of tiles reached: a bin file can only contain 256 tiles maximum")]
    MaximumTilesReached,
    #[error("Not enough tiles, a bin file must contain exactly 256 tiles")]
    NotEnoughTiles(BinFileWriter)
}

#[derive(Debug, Error, From)]
pub enum FillRemainingSpaceError {
    #[error(transparent)]
    TileWrite(TileWriteError),
    #[from(ignore)]
    #[error("bin file is empty, cannot determine tile kind to write")]
    Empty
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
        self.file.close()?;
        Ok(())
    }

}