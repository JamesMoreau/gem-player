use std::{
    fs::create_dir_all,
    io,
    path::{Path, PathBuf},
};

use directories::ProjectDirs;
use image::{ImageFormat, load_from_memory};
use m3u::Url;

use crate::{
    APP_NAME,
    track::{Track, extract_artwork},
};

const ARTWORK_CACHE_FILENAME: &str = "playing.png";

// To cache the playing track's artwork, we extract the picture from the
// track, then normalize it to a png format. There is only ever a single
// artwork cached at one time.
pub fn cache_track_artwork(track: &Track) -> io::Result<bool> {
    let Some(picture) = extract_artwork(track) else {
        return Ok(false);
    };

    let image = load_from_memory(picture.data()).map_err(io::Error::other)?;

    image
        .save_with_format(artwork_cache_path()?, ImageFormat::Png)
        .map_err(io::Error::other)?;

    Ok(true)
}

pub fn artwork_uri() -> Option<String> {
    let path = artwork_cache_path().ok()?;

    path.is_file().then(|| compute_uri(&path))
}

fn artwork_cache_path() -> io::Result<PathBuf> {
    Ok(get_or_init_artwork_cache()?.join(ARTWORK_CACHE_FILENAME))
}

fn compute_uri(path: &Path) -> String {
    Url::from_file_path(path)
        .expect("cache path must always be a valid file URL path")
        .to_string()
}

fn get_or_init_artwork_cache() -> io::Result<PathBuf> {
    let proj_dirs = ProjectDirs::from("", "", APP_NAME).ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "no project dirs"))?;

    let directory = proj_dirs.cache_dir().join("artwork");

    create_dir_all(&directory)?;

    Ok(directory)
}
