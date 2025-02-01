use eframe::egui::{Vec2, ViewportBuilder};
use std::{path::PathBuf, time::Duration};
use strum_macros::EnumIter;
use colored::Colorize;

mod player;
mod ui;

/*
TODO:
* forget paris and just have a simple error, warn, info for debug.
* add toast notifications.
* register play pause commands with apple menu.
* Music Visualizer ^.
* add a debug print to only print in debug mode
* use a better url for Image::from_bytes(artwork_uri, artwork_bytes.clone()) that guarantees uniqueness.
* edit track metadata view (but not listed in the navigation. only available by right clicking on a track)
* Rename "Unknown X" to something else like ??? or N/A.
* playlists / m3u.
* system theme not switching automatically.
*/

fn main() -> eframe::Result {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).

    let options = eframe::NativeOptions {
        viewport: ViewportBuilder::default()
            .with_min_inner_size(Vec2::new(900.0, 500.0))
            .with_decorations(false)
            .with_transparent(true),
        ..Default::default()
    };
    eframe::run_native("Gem Player", options, Box::new(|cc| Ok(Box::new(player::GemPlayer::new(cc)))))
}

#[derive(EnumIter, Debug, PartialEq, Eq, Clone, Copy)]
pub enum Theme {
    System,
    Dark,
    Light,
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

#[derive(Debug, Clone, PartialEq, Eq)]
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

pub fn format_duration_to_hhmmss(duration: std::time::Duration) -> String {
    let total_seconds = duration.as_secs();
    let seconds_per_minute = 60;
    let minutes_per_hour = 60;
    let hours = total_seconds / (minutes_per_hour * seconds_per_minute);
    let minutes = (total_seconds / seconds_per_minute) % minutes_per_hour;
    let seconds = total_seconds % seconds_per_minute;

    format!("{}:{:02}:{:02}", hours, minutes, seconds)
}

#[derive(Debug, Clone)]
pub struct Playlist {
    pub name: String,
    pub creation_date: std::time::SystemTime,
    pub songs: Vec<Song>,
    pub path: Option<PathBuf>,
}

pub fn get_duration_of_songs(songs: &[Song]) -> Duration {
    songs.iter().map(|song| song.duration).sum()
}

#[cfg(debug_assertions)]
pub fn print_info<T: std::fmt::Display>(info: T) {
    println!("ℹ {}", info);
}

#[cfg(debug_assertions)]
pub fn print_success<T: std::fmt::Display>(success: T) {
    println!("✔ {}", success.to_string().green());
}

#[cfg(debug_assertions)]
pub fn print_error<T: std::fmt::Display>(error: T) {
    println!("✖ {}", error.to_string().red());
}
