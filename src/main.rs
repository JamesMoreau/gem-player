use eframe::egui::{self, Vec2, ViewportBuilder};

mod ui;
mod player;

/*
TODO:
- instead of a sepator between ui sections, could just use a different color.
- could move filter/sort from the top UI to the bottom UI and have the visualizer at the top.
- selection needs to be cleared when songs are sorted / filtered.
- play next song after current song ends
- tab bar at the bottom for playlists, queue, settings, etc.
- should read_music_from_directory return a Result<Vec<Song>, Error> instead of Vec<Song>? Fix this once we allow custom music path. loading icon when songs are being loaded.
- file watcher / update on change
- register play pause commands with apple menu.

- Play button / Pause button, Next song, previous song
- Repeat / Shuffle above the playback progress. Could stack them vertically to the left of the artwork.
- Music Visualizer ^.
- Queue

* Could remove object oriented programming and just have a struct with functions that take a mutable reference to self.

*/

fn main() -> eframe::Result {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).

    let options = eframe::NativeOptions {
        viewport: ViewportBuilder::default()
            .with_min_inner_size(Vec2::new(1200.0, 500.0))
            .with_decorations(false)
            .with_transparent(true),
        ..Default::default()
    };
    eframe::run_native(
        "Gem Player",
        options,
        Box::new(|cc| Ok(Box::new(player::GemPlayer::new(cc)))),
    )
}

use std::{path::{Path, PathBuf}, time::Duration};

use lofty::{file::{AudioFile, TaggedFileExt}, tag::ItemKey};
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

#[allow(unused)]
#[derive(Debug, Clone)]
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

    let title = tag.get_string(&ItemKey::TrackTitle).map(|t| t.to_owned())
        .or_else(|| path.file_stem().and_then(|s| s.to_str()).map(|s| s.to_owned()));

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

pub fn sort_songs(songs: &mut [Song], sort_by: SortBy, sort_order: SortOrder) {
    songs.sort_by(|a, b| {
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

pub fn format_duration_to_mmss(duration: std::time::Duration) -> String {
    let total_seconds = duration.as_secs();
    let seconds_per_minute = 60;
    let minutes = total_seconds / seconds_per_minute;
    let seconds = total_seconds % seconds_per_minute;

    format!("{}:{:02}", minutes, seconds)
}
