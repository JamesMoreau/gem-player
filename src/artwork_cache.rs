use std::{
    fs::{create_dir_all, write},
    io,
    path::PathBuf,
};

use directories::ProjectDirs;
use fully_pub::fully_pub;
use m3u::Url;

use crate::APP_NAME;

#[fully_pub]
struct ArtworkCache {
    dir: PathBuf,
}

const ARTWORK_CACHE_FILENAME: &str = "playing.png";

impl ArtworkCache {
    pub fn set_playing(&mut self, data: &[u8]) -> io::Result<PathBuf> {
        let path = self.dir.join(ARTWORK_CACHE_FILENAME);
        write(&path, data)?;
        Ok(path)
    }

    pub fn new() -> io::Result<Self> {
        let proj_dirs = ProjectDirs::from("", "", APP_NAME).ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "no project dirs"))?;

        let dir = proj_dirs.cache_dir().join("artwork");

        create_dir_all(&dir)?;

        Ok(Self { dir })
    }

    pub fn playing(&self) -> PathBuf {
        self.dir.join(ARTWORK_CACHE_FILENAME)
    }

    pub fn current_uri(&self) -> String {
        Url::from_file_path(self.playing()).unwrap().to_string()
    }
}
