use std::{
    fs::{create_dir_all, remove_file},
    io,
    path::{Path, PathBuf},
};

use directories::ProjectDirs;
use image::{ImageFormat, load_from_memory};

use crate::{
    APP_NAME,
    track::{Track, extract_artwork},
};

const ARTWORK_CACHE_FILENAME: &str = "playing.png";

pub fn cache_track_artwork(track: &Track) -> io::Result<bool> {
    let cache_directory = get_or_init_artwork_cache()?;

    let Some(picture) = extract_artwork(track) else {
        return Ok(false);
    };

    let image = load_from_memory(picture.data()).map_err(io::Error::other)?;

    image
        .save_with_format(artwork_cache_path(&cache_directory), ImageFormat::Png)
        .map_err(io::Error::other)?;

    Ok(true)
}

fn artwork_cache_path(cache_directory: &Path) -> PathBuf {
    cache_directory.join(ARTWORK_CACHE_FILENAME)
}

pub fn clear_artwork_cache() -> io::Result<()> {
    let path = artwork_cache_path(&get_or_init_artwork_cache()?);

    match remove_file(path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e),
    }
}

pub fn get_or_init_artwork_cache() -> io::Result<PathBuf> {
    let proj_dirs = ProjectDirs::from("", "", APP_NAME).ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "no project dirs"))?;

    let directory = proj_dirs.cache_dir().join("artwork");

    create_dir_all(&directory)?;

    Ok(directory)
}
