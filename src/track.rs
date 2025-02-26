use fully_pub::fully_pub;
use lofty::{
    file::{AudioFile, TaggedFileExt},
    tag::ItemKey,
};
use log::error;
use walkdir::WalkDir;
use std::{
    fs, io::{self, ErrorKind}, path::{Path, PathBuf}, time::Duration
};
use strum_macros::EnumIter;

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
    title: Option<String>,
    artist: Option<String>,
    album: Option<String>,
    duration: Duration,
    artwork: Option<Vec<u8>>,
    file_path: PathBuf,
}

impl PartialEq for Track {
    #[inline]
    fn eq(&self, other: &Track) -> bool {
        self.file_path == other.file_path
    }
}

pub fn sort(tracks: &mut [Track], sort_by: SortBy, sort_order: SortOrder) {
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

pub fn load_from_file(path: &Path) -> io::Result<Track> {
    if !path.is_file() {
        return Err(io::Error::new(io::ErrorKind::NotFound, "Path is not a file"));
    }

    let result = lofty::read_from_path(path);
    let tagged_file = match result {
        Ok(file) => file,
        Err(e) => {
            return Err(io::Error::new(ErrorKind::InvalidData, format!("Error reading file: {}", e)));
        }
    };

    let tag = match tagged_file.primary_tag() { // TODO: can this be reduced?
        Some(tag) => tag,
        None => match tagged_file.first_tag() {
            Some(tag) => tag,
            None => return Err(io::Error::new(ErrorKind::InvalidData, format!("No tags found in file: {:?}", path))),
        },
    };

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
        title,
        artist,
        album,
        duration,
        artwork,
        file_path,
    })
}

fn is_audio_file(path: &Path) -> bool {
    if let Ok(data) = fs::read(path) {
        if let Some(kind) = infer::get(&data) {
            return kind.mime_type().starts_with("audio/");
        }
    }
    
    false
}

pub fn read_music(directory: &Path) -> io::Result<Vec<Track>> {
    let mut tracks = Vec::new();
    
    for entry in WalkDir::new(directory).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();

        let what_we_want = path.is_file() && is_audio_file(path);
        if !what_we_want {
            continue;
        }

        let result = load_from_file(path);
        match result {
            Err(e) => error!("{}", e),
            Ok(track) => tracks.push(track),
        }
    }

    Ok(tracks)
}

pub fn calculate_total_duration(tracks: &[Track]) -> Duration {
    tracks.iter().map(|track| track.duration).sum()
}

pub fn open_file_location(track: &Track) -> io::Result<()> {
    let maybe_folder = track.file_path.as_path().parent();
    let Some(folder) = maybe_folder else {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "Track has no file path."));
    };

    open::that_detached(folder)?;

    Ok(())
}
