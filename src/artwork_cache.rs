use std::{
    fs::{create_dir_all, read_dir, remove_file, write},
    io,
    path::{Path, PathBuf},
};

use directories::ProjectDirs;
use lofty::picture::{MimeType, Picture};
use m3u::Url;

use crate::APP_NAME;

const ARTWORK_CACHE_FILEBASE: &str = "playing";

pub fn cache_artwork(picture: &Picture) -> io::Result<String> {
    let cache_directory = get_or_init_artwork_cache()?;
    clear_artwork_cache(&cache_directory)?;

    let path = artwork_cache_path(&cache_directory, picture.mime_type());
    write(&path, picture.data())?;

    Ok(compute_uri(&path))
}

fn compute_uri(path: &Path) -> String {
    Url::from_file_path(path)
        .expect("cache path should always be an absolute file path")
        .to_string()
}

fn artwork_cache_path(cache_directory: &Path, mime: Option<&MimeType>) -> PathBuf {
    let ext = mime.and_then(MimeType::ext).unwrap_or("png");
    let filename = format!("{}.{}", ARTWORK_CACHE_FILEBASE, ext);

    cache_directory.join(filename)
}

fn clear_artwork_cache(cache_directory: &Path) -> io::Result<()> {
    for entry in read_dir(cache_directory)? {
        remove_file(entry?.path())?;
    }

    Ok(())
}

pub fn get_or_init_artwork_cache() -> io::Result<PathBuf> {
    let proj_dirs = ProjectDirs::from("", "", APP_NAME).ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "no project dirs"))?;

    let directory = proj_dirs.cache_dir().join("artwork");

    create_dir_all(&directory)?;

    Ok(directory)
}
