use std::{
    fs::{create_dir_all, write},
    io,
    path::{Path, PathBuf},
};

use directories::ProjectDirs;
use m3u::Url;

use crate::APP_NAME;

const ARTWORK_CACHE_FILENAME: &str = "playing.png";

pub fn compute_uri(path: &Path) -> String {
    Url::from_file_path(path)
        .expect("cache path should always be an absolute file path")
        .to_string()
}

pub fn cache_playing_artwork(cache_directory: &Path, data: &[u8]) -> io::Result<String> {
    let path = cache_directory.join(ARTWORK_CACHE_FILENAME);
    write(&path, data)?;

    Ok(compute_uri(&path))
}

pub fn artwork_cache_dir() -> io::Result<PathBuf> {
    let proj_dirs = ProjectDirs::from("", "", APP_NAME).ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "no project dirs"))?;

    let directory = proj_dirs.cache_dir().join("artwork");

    create_dir_all(&directory)?;

    Ok(directory)
}
