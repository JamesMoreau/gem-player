use eframe::egui::{Vec2, ViewportBuilder};
use log::info;

use player::init_gem_player;
use song::Song;

mod player;
mod playlist;
mod song;
mod ui;

/*
TODO:
* play from playlist.
* previous and next not disabling correctly. also left and right key commands not doing proper check!
* edit track metadata view (but not listed in the navigation. only available by right clicking on a track). could be a popup menu.
* could use egui_inbox for library updating with watcher.
* should expensive operations such as opening a file use an async system? research this!
* Music Visualizer.
* maybe make volume slider hover. Could make a new fat enum like muted, unmuted(volume)?
* images with different aspect ratios should be stretched or cropped to match 1:1.
* should library and playlist views have different sort by ui state?
* sort by and order thing. Could just use a combobox?
* maybe just remove right clicking songs and only have more buttons!?
*/

fn main() -> eframe::Result {
    env_logger::init(); // Log to stderr (if run with `RUST_LOG=debug`).
    info!("Starting up Gem Player.");

    let options = eframe::NativeOptions {
        viewport: ViewportBuilder::default()
            .with_min_inner_size(Vec2::new(900.0, 500.0))
            .with_decorations(false)
            .with_transparent(true),
        ..Default::default()
    };
    eframe::run_native("Gem Player", options, Box::new(|cc| Ok(Box::new(init_gem_player(cc)))))
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

