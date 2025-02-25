use std::{
    io::{self, ErrorKind},
    path::{Path, PathBuf},
    time::Duration,
};

use fully_pub::fully_pub;
use glob::glob;
use lofty::{
    file::{AudioFile, TaggedFileExt},
    tag::ItemKey,
};
use log::error;
use strum_macros::EnumIter;
use uuid::Uuid;

use crate::player::SUPPORTED_AUDIO_FILE_TYPES;

#[derive(EnumIter, Debug, PartialEq, Eq, Clone, Copy)]
pub enum SortBy {
    Title,
    Artist,
    Album,
    Time,
}

#[derive(EnumIter, Debug, PartialEq, Eq, Clone, Copy)]
pub enum SortOrder {
    Ascending,
    Descending,
}

#[fully_pub]
#[derive(Debug, Clone)]
pub struct Track {
    id: Uuid,
    title: Option<String>,
    artist: Option<String>,
    album: Option<String>,
    duration: Duration,
    artwork: Option<Vec<u8>>,
    file_path: PathBuf,
}

pub fn find_track(track_id: Uuid, tracks: &[Track]) -> Option<&Track> {
    tracks.iter().find(|p| p.id == track_id)
}

pub fn find_track_mut(track_id: Uuid, tracks: &mut [Track]) -> Option<&mut Track> {
    tracks.iter_mut().find(|p| p.id == track_id)
}

pub fn sort_tracks(tracks: &mut [Track], sort_by: SortBy, sort_order: SortOrder) {
    tracks.sort_by(|a, b| {
        let ordering = match sort_by {
            SortBy::Title => a.title.as_deref().unwrap_or("").cmp(b.title.as_deref().unwrap_or("")),
            SortBy::Artist => a.artist.as_deref().unwrap_or("").cmp(b.artist.as_deref().unwrap_or("")),
            SortBy::Album => a.album.as_deref().unwrap_or("").cmp(b.album.as_deref().unwrap_or("")),
            SortBy::Time => a.duration.cmp(&b.duration),
        };

        match sort_order {
            SortOrder::Ascending => ordering,
            SortOrder::Descending => ordering.reverse(),
        }
    });
}

pub fn get_track_from_file(path: &Path) -> io::Result<Track> {
    if !path.is_file() {
        return Err(io::Error::new(io::ErrorKind::NotFound, "Path is not a file"));
    }

    let result_file = lofty::read_from_path(path);
    let tagged_file = match result_file {
        Ok(file) => file,
        Err(e) => {
            return Err(io::Error::new(ErrorKind::InvalidData, format!("Error reading file: {}", e)));
        }
    };

    let tag = match tagged_file.primary_tag() {
        Some(tag) => tag,
        None => match tagged_file.first_tag() {
            Some(tag) => tag,
            None => return Err(io::Error::new(ErrorKind::InvalidData, format!("No tags found in file: {:?}", path))),
        },
    };

    let id = Uuid::new_v4();

    let title = tag
        .get_string(&ItemKey::TrackTitle)
        .map(|t| t.to_owned())
        .or_else(|| path.file_stem().and_then(|s| s.to_str()).map(|s| s.to_owned()));

    let artist = tag.get_string(&ItemKey::TrackArtist).map(|a| a.to_owned());

    let album = tag.get_string(&ItemKey::AlbumTitle).map(|a| a.to_owned());

    let properties = tagged_file.properties();
    let duration = properties.duration();

    let artwork_result = tag.pictures().first();
    let artwork = artwork_result.map(|artwork| artwork.data().to_vec());

    let file_path = path.to_path_buf();

    Ok(Track {
        id,
        title,
        artist,
        album,
        duration,
        artwork,
        file_path,
    })
}

pub fn read_music_from_a_directory(path: &Path) -> io::Result<Vec<Track>> {
    let patterns = SUPPORTED_AUDIO_FILE_TYPES
        .iter()
        .map(|file_type| format!("{}/*.{}", path.to_string_lossy(), file_type))
        .collect::<Vec<String>>();

    let mut file_paths = Vec::new();
    for pattern in patterns {
        let file_paths_result = glob(&pattern);
        match file_paths_result {
            Ok(paths) => {
                for path in paths.filter_map(Result::ok) {
                    file_paths.push(path);
                }
            }
            Err(e) => {
                return Err(io::Error::new(io::ErrorKind::Other, format!("Invalid pattern: {}", e)));
            }
        }
    }

    let mut tracks = Vec::new();
    for path in file_paths {
        let result = get_track_from_file(&path);
        match result {
            Ok(track) => tracks.push(track),
            Err(e) => error!("{}", e),
        }
    }

    Ok(tracks)
}

pub fn get_duration_of_tracks(tracks: &[Track]) -> Duration {
    tracks.iter().map(|track| track.duration).sum()
}

pub fn open_track_file_location(track: &Track) -> io::Result<()> {
    let maybe_folder = track.file_path.as_path().parent();
    let Some(folder) = maybe_folder else {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "Track has no file path."));
    };

    open::that_detached(folder)?;

    Ok(())
}
