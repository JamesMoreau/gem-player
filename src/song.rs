use lofty::prelude::*;
use std::{
    path::{Path, PathBuf},
    time::Duration,
};

pub struct Song {
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub duration: Duration,
    pub artwork: Option<Vec<u8>>,
    pub file_path: PathBuf,
}

pub fn get_song_from_file(path: &Path) -> Option<Song> {
    if !path.is_file() {
        println!("Path is not a file: {:?}", path);
        return None;
    }

    let result_file = lofty::read_from_path(path);
    let tagged_file = match result_file {
        Ok(file) => file,
        Err(e) => {
            println!("Error reading file {}: {}", path.display(), e);
            return None;
        }
    };

    let tag = match tagged_file.primary_tag() {
        Some(tag) => tag,
        None => tagged_file.first_tag()?,
    };

    let title = tag.get_string(&ItemKey::TrackTitle).map(|t| t.to_owned());
    let artist = tag.get_string(&ItemKey::TrackArtist).map(|a| a.to_owned());
    let album = tag.get_string(&ItemKey::AlbumTitle).map(|a| a.to_owned());

    let properties = tagged_file.properties();
    let duration = properties.duration();

    let artwork_result = tag.pictures().first();
    let artwork = artwork_result.map(|artwork| artwork.data().to_vec());

    let file_path = path.to_path_buf();

    Some(Song {
        title,
        artist,
        album,
        duration,
        artwork,
        file_path,
    })
}
