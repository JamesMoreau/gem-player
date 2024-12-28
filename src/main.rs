use eframe::egui::{Vec2, ViewportBuilder};
use std::{path::PathBuf, time::Duration};
use strum_macros::EnumIter;

mod player;
mod ui;

/*
TODO:
- instead of a sepator between ui sections, could just use a different color.
- could move filter/sort from the top UI to the bottom UI and have the visualizer at the top.
- selection needs to be cleared when songs are sorted / filtered.
- play next song after current song ends
- should read_music_from_directory return a Result<Vec<Song>, Error> instead of Vec<Song>? Fix this once we allow custom music path. loading icon when songs are being loaded.
- file watcher / update on change
- register play pause commands with apple menu.
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
    eframe::run_native("Gem Player", options, Box::new(|cc| Ok(Box::new(player::GemPlayer::new(cc)))))
}

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
