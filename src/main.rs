use chrono::{DateTime, Utc};
use eframe::egui::{Vec2, ViewportBuilder};
use fully_pub::fully_pub;
use uuid::Uuid;
use std::{path::PathBuf, time::Duration};
use strum_macros::EnumIter;
use colored::Colorize;

mod player;
mod ui;

/*
TODO:
* deleted m3u file should be moved to trash (and not permanently deleted).
* figure out why selecting a playlist causes teh ui to shift down.
* playlists / m3u.
* Music Visualizer ^.
* maybe make volume slider hover.
* use a better url for Image::from_bytes(artwork_uri, artwork_bytes.clone()) that guarantees uniqueness.
* edit track metadata view (but not listed in the navigation. only available by right clicking on a track). could be a popup menu.
* system theme not switching automatically.
* could use egui_inbox for library updating with watcher.
* should expensive operations such as opening a file use an async system? research this!
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

#[fully_pub]
#[derive(Debug, Clone)]
pub struct Song {
    id: Uuid,
    title: Option<String>,
    artist: Option<String>,
    album: Option<String>,
    duration: Duration,
    artwork: Option<Vec<u8>>,
    file_path: PathBuf,
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

#[fully_pub]
#[derive(Debug, Clone)]
pub struct Playlist {
    id: Uuid,
    name: String,
    creation_date_time: DateTime<Utc>,
    songs: Vec<Song>,
    path: Option<PathBuf>,
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
