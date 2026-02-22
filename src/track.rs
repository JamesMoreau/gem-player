use fully_pub::fully_pub;
use infer::{get_from_path, MatcherType};
use lofty::{
    file::{AudioFile, FileType, TaggedFileExt},
    read_from, read_from_path,
    tag::ItemKey,
};
use log::error;
use rayon::prelude::*;
use rodio::SampleRate;
use std::{
    fs::{metadata, File},
    io::{self, ErrorKind},
    num::NonZeroU32,
    path::{Path, PathBuf},
    time::{Duration, SystemTime},
};
use strum_macros::EnumIter;
use walkdir::WalkDir;

#[derive(EnumIter, Debug, PartialEq, Eq, Clone, Copy)]
pub enum SortBy {
    Title,
    Artist,
    Album,
    Time,
    DateAdded,
}

pub fn sort_by_label(sort_by: SortBy) -> &'static str {
    match sort_by {
        SortBy::Title => "Title",
        SortBy::Artist => "Artist",
        SortBy::Album => "Album",
        SortBy::Time => "Time",
        SortBy::DateAdded => "Date Added",
    }
}

#[derive(EnumIter, Debug, PartialEq, Eq, Clone, Copy)]
pub enum SortOrder {
    Ascending,
    Descending,
}

#[fully_pub]
#[derive(Debug, Clone)]
struct Track {
    title: Option<String>,
    artist: Option<String>,
    album: Option<String>,
    duration: Duration,
    path: PathBuf,
    sample_rate: Option<SampleRate>,
    codec: FileType,
    date_added: SystemTime,
}

impl PartialEq for Track {
    #[inline]
    fn eq(&self, other: &Track) -> bool {
        self.path == other.path
    }
}

pub trait TrackRetrieval {
    fn get_by_path(&self, path: &Path) -> &Track;
}

impl TrackRetrieval for Vec<Track> {
    fn get_by_path(&self, path: &Path) -> &Track {
        self.iter().find(|t| t.path == path).expect("Track not found")
    }
}

pub fn sort(tracks: &mut [Track], sort_by: SortBy, sort_order: SortOrder) {
    tracks.sort_by(|a, b| {
        let ordering = match sort_by {
            SortBy::Title => a.title.as_deref().unwrap_or("").cmp(b.title.as_deref().unwrap_or("")),
            SortBy::Artist => a.artist.as_deref().unwrap_or("").cmp(b.artist.as_deref().unwrap_or("")),
            SortBy::Album => a.album.as_deref().unwrap_or("").cmp(b.album.as_deref().unwrap_or("")),
            SortBy::Time => a.duration.cmp(&b.duration),
            SortBy::DateAdded => a.date_added.cmp(&b.date_added),
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

    let tagged_file = read_from_path(path).map_err(|e| io::Error::new(ErrorKind::InvalidData, format!("Error reading file: {}", e)))?;

    let tag = tagged_file
        .primary_tag()
        .or_else(|| tagged_file.first_tag())
        .ok_or_else(|| io::Error::new(ErrorKind::InvalidData, format!("No tags found in file: {:?}", path)))?;

    let title = tag
        .get_string(ItemKey::TrackTitle)
        .map(|t| t.to_owned())
        .or_else(|| path.file_stem().and_then(|s| s.to_str()).map(|s| s.to_owned()));
    let artist = tag.get_string(ItemKey::TrackArtist).map(|a| a.to_owned());
    let album = tag.get_string(ItemKey::AlbumTitle).map(|a| a.to_owned());

    let properties = tagged_file.properties();
    let duration = properties.duration();

    let sample_rate = properties
        .sample_rate()
        .map(|rate| {
            NonZeroU32::new(rate)
                .ok_or_else(|| io::Error::new(ErrorKind::InvalidData, format!("Invalid sample rate (0) in file: {:?}", path)))
        })
        .transpose()?;

    let file_path = path.to_path_buf();

    let codec = tagged_file.file_type();

    let file_metadata = metadata(path)?;
    let date_added = file_metadata.created().or_else(|_| file_metadata.modified())?;

    Ok(Track {
        title,
        artist,
        album,
        duration,
        path: file_path,
        sample_rate,
        codec,
        date_added,
    })
}

pub fn is_relevant_media_file(path: &Path) -> bool {
    let file_type = get_from_path(path).ok().flatten().map(|k| k.matcher_type());
    matches!(file_type, Some(MatcherType::Audio) | Some(MatcherType::Video))
}

pub fn load_tracks_from_directory(directory: &Path) -> io::Result<Vec<Track>> {
    let entries: Vec<_> = WalkDir::new(directory)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|entry| {
            let path = entry.path();
            path.is_file() && is_relevant_media_file(path)
        })
        .map(|entry| entry.into_path())
        .collect();

    let tracks: Vec<Track> = entries
        .par_iter()
        .filter_map(|path| match load_from_file(path) {
            Ok(track) => Some(track),
            Err(e) => {
                error!("{}", e);
                None
            }
        })
        .collect();

    Ok(tracks)
}

pub fn calculate_total_duration(tracks: &[Track]) -> Duration {
    tracks.iter().map(|track| track.duration).sum()
}

pub fn open_file_location(track: &Track) -> io::Result<()> {
    let path = track.path.as_path();

    let result = opener::reveal(path);
    if let Err(e) = result {
        return Err(io::Error::other(format!("Failed to open file location: {}", e)));
    }

    Ok(())
}

pub fn extract_artwork_from_file(file: &mut File) -> io::Result<Option<Vec<u8>>> {
    let tagged_file = read_from(file).map_err(|e| io::Error::new(ErrorKind::InvalidData, format!("Error reading tags: {}", e)))?;

    let tag = tagged_file
        .primary_tag()
        .or_else(|| tagged_file.first_tag())
        .ok_or_else(|| io::Error::new(ErrorKind::InvalidData, "No tags found"))?;

    Ok(tag.pictures().first().map(|pic| pic.data().to_vec()))
}

pub fn file_type_name(ft: FileType) -> &'static str {
    match ft {
        FileType::Aac => "AAC",
        FileType::Aiff => "AIFF",
        FileType::Ape => "APE",
        FileType::Flac => "FLAC",
        FileType::Mpeg => "MPEG",
        FileType::Mp4 => "MP4",
        FileType::Mpc => "MPC",
        FileType::Opus => "OPUS",
        FileType::Vorbis => "VORB",
        FileType::Speex => "SPX",
        FileType::Wav => "WAV",
        FileType::WavPack => "WVPK",
        FileType::Custom(_) => "CSTM",
        _ => "UNK",
    }
}
