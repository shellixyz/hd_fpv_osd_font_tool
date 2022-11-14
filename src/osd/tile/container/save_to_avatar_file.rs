
use std::path::Path;

use super::Tile;

use crate::osd::{
    tile::grid::Grid as TileGrid,
    avatar_file::{
        self,
        SaveError as AvatarFileSaveError,
    }
};

pub trait SaveToAvatarFile {
    fn save_to_avatar_file<P: AsRef<Path>>(&self, path: P) -> Result<(), AvatarFileSaveError>;
}

impl SaveToAvatarFile for &[Tile] {
    fn save_to_avatar_file<P: AsRef<Path>>(&self, path: P) -> Result<(), AvatarFileSaveError> {
        avatar_file::save(self, path)
    }
}

impl SaveToAvatarFile for Vec<Tile> {
    fn save_to_avatar_file<P: AsRef<Path>>(&self, path: P) -> Result<(), AvatarFileSaveError> {
        self.as_slice().save_to_avatar_file(path)
    }
}

pub trait SaveTilesToAvatarFile {
    fn save_tiles_to_avatar_file<P: AsRef<Path>>(&self, path: P) -> Result<(), AvatarFileSaveError>;
}

impl SaveTilesToAvatarFile for TileGrid {
    fn save_tiles_to_avatar_file<P: AsRef<Path>>(&self, path: P) -> Result<(), AvatarFileSaveError> {
        self.as_slice().save_to_avatar_file(path)
    }
}
