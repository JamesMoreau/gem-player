use std::{
    fs::{create_dir_all, remove_file},
    io,
    path::PathBuf,
};

use anyhow::Result;
use directories::ProjectDirs;
use image::{ImageFormat, load_from_memory};
use m3u::Url;

use crate::{
    APP_NAME,
    track::{Track, extract_artwork},
};

const ARTWORK_CACHE_FILENAME: &str = "playing_artwork.png";

// To cache the playing track's artwork, we extract the picture from the
// track, then normalize it to a png format. There is only ever a single
// artwork cached at a time.
pub fn update_artwork_cache(track: &Track) -> Result<()> {
    clear_artwork_cache()?;

    let Some(picture) = extract_artwork(track)? else {
        // Track has no artwork, which is fine.
        return Ok(());
    };

    let image = load_from_memory(picture.data())?;
    let path = initialize_artwork_cache()?;

    image.save_with_format(&path, ImageFormat::Png)?;

    Ok(())
}

pub fn clear_artwork_cache() -> Result<()> {
    let path = initialize_artwork_cache()?;

    match remove_file(path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e.into()),
    }
}

pub fn artwork_uri() -> Option<String> {
    let path = artwork_cache_path()?.join(ARTWORK_CACHE_FILENAME);

    path.is_file().then(|| {
        Url::from_file_path(&path)
            .expect("cache path must always be a valid file URL path")
            .to_string()
    })
}

fn artwork_cache_path() -> Option<PathBuf> {
    let proj_dirs = ProjectDirs::from("", "", APP_NAME)?;

    Some(proj_dirs.cache_dir().to_path_buf())
}

fn initialize_artwork_cache() -> Result<PathBuf> {
    let directory = artwork_cache_path().ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "no project dirs"))?;

    create_dir_all(&directory)?;

    Ok(directory.join(ARTWORK_CACHE_FILENAME))
}
