use eframe::egui::{self, Vec2};

mod ui;
mod utils;
mod player;
mod models;

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

* remove egui:: everywhere.
*/

fn main() -> eframe::Result {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
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

