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
    directory: PathBuf,
}

const ARTWORK_CACHE_FILENAME: &str = "playing.png";

impl ArtworkCache {
    pub fn set_playing(&mut self, data: &[u8]) -> io::Result<String> {
        let path = self.directory.join(ARTWORK_CACHE_FILENAME);
        write(&path, data)?;

        let uri = Url::from_file_path(path).unwrap().to_string();
        Ok(uri)
    }

    pub fn new() -> io::Result<Self> {
        let proj_dirs = ProjectDirs::from("", "", APP_NAME).ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "no project dirs"))?;

        let directory = proj_dirs.cache_dir().join("artwork");

        create_dir_all(&directory)?;

        Ok(Self { directory })
    }
}
