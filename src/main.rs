use eframe::egui::{Color32, Context, Event, Key, Rgba, ThemePreference, Vec2, ViewportBuilder, Visuals};
use egui_notify::Toasts;
use fully_pub::fully_pub;
use indexmap::IndexMap;
use lazy_static::lazy_static;
use log::{error, info};
use player::{adjust_volume_by_percentage, mute_or_unmute, play_next, play_or_pause, play_previous, clear_the_queue, Player};
use playlist::{read_all_from_a_directory, Playlist, PlaylistRetrieval};
use rodio::{OutputStream, Sink};
use std::{
    path::{Path, PathBuf},
    time::Duration,
};
use track::{read_in_tracks_from_directory, SortBy, SortOrder, Track, TrackRetrieval};
use ui::{render_gem_player, update_theme, LibraryViewState, PlaylistsViewState, UIState};

mod player;
mod playlist;
mod track;
mod ui;

/*
TODO:
* perfomance improvements. cache or don't sort and filter songs every frame?
* could use egui_inbox for library updating with watcher. should expensive operations such as opening a file use an async system? research this!
* Music Visualizer.
* maybe make volume slider hover. Could make a new fat enum like muted, unmuted(volume)?
* UI + aestethics. Scrolling track info could be cool (maybe only applies when the string is too big?)
* Fullscreen?
*/

pub const LIBRARY_DIRECTORY_STORAGE_KEY: &str = "library_directory";
pub const THEME_STORAGE_KEY: &str = "theme";

#[fully_pub]
pub struct GemPlayer {
    pub ui_state: UIState,

    pub library: Vec<Track>,                // All the tracks stored in the user's music directory.
    pub library_directory: Option<PathBuf>, // The directory where music is stored.
    pub playlists: Vec<Playlist>,

    pub player: Player,
}

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

pub fn init_gem_player(cc: &eframe::CreationContext<'_>) -> GemPlayer {
    egui_extras::install_image_loaders(&cc.egui_ctx);

    egui_material_icons::initialize(&cc.egui_ctx);

    let mut library_directory = None;
    let mut theme_preference = ThemePreference::System;
    if let Some(storage) = cc.storage {
        if let Some(library_directory_string) = storage.get_string(LIBRARY_DIRECTORY_STORAGE_KEY) {
            library_directory = Some(PathBuf::from(library_directory_string));
        }

        if let Some(theme_string) = storage.get_string(THEME_STORAGE_KEY) {
            theme_preference = ron::from_str(&theme_string).unwrap_or(ThemePreference::System);
        }
    }

    let sort_by = SortBy::Title;
    let sort_order = SortOrder::Ascending;

    let mut library = Vec::new();
    let mut playlists = Vec::new();
    if let Some(directory) = &library_directory {
        let (found_tracks, found_playlists) = read_tracks_and_playlists_from_directory(directory);

        library = found_tracks;
        playlists = found_playlists;
    }

    let (stream, handle) = OutputStream::try_default().unwrap();
    let sink = Sink::try_new(&handle).unwrap();
    sink.pause();
    let initial_volume = 0.6;
    sink.set_volume(initial_volume);

    GemPlayer {
        ui_state: UIState {
            current_view: ui::View::Library,
            theme_preference,
            library: LibraryViewState {
                cached_library: Vec::new(),
                cache_dirty: true,
                search_string: String::new(),
                selected_track_key: None,
                sort_by,
                sort_order,
                track_menu_is_open: false,
            },
            playlists: PlaylistsViewState {
                selected_playlist_key: None,
                cached_playlist_tracks: Vec::new(),
                cache_dirty: true,
                playlist_rename: None,
                delete_playlist_modal_is_open: false,
                selected_track_key: None,
                track_menu_is_open: false,
                search_string: String::new(),
            },
            toasts: Toasts::default()
                .with_anchor(egui_notify::Anchor::BottomRight)
                .with_shadow(eframe::egui::Shadow {
                    offset: [0, 0],
                    blur: 1,
                    spread: 1,
                    color: Color32::BLACK,
                }),
        },

        library,
        library_directory,
        playlists,

        player: Player {
            history: Vec::new(),
            playing: None,
            queue: Vec::new(),

            repeat: false,
            shuffle: None,
            muted: false,
            volume_before_mute: None,
            paused_before_scrubbing: None,

            stream,
            sink,
        },
    }
}

impl eframe::App for GemPlayer {
    fn clear_color(&self, _visuals: &Visuals) -> [f32; 4] {
        Rgba::TRANSPARENT.to_array() // Make sure we don't paint anything behind the rounded corners
    }

    // This is set because egui was persisting the state of the library table scroll position across runs.
    fn persist_egui_memory(&self) -> bool {
        false
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        if let Some(library_directory) = &self.library_directory {
            storage.set_string(LIBRARY_DIRECTORY_STORAGE_KEY, library_directory.to_string_lossy().to_string());
        }

        let theme_ron_string = ron::to_string(&self.ui_state.theme_preference).unwrap();
        storage.set_string(THEME_STORAGE_KEY, theme_ron_string);
    }

    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        handle_key_commands(ctx, self);

        check_for_next_track(self);

        ctx.request_repaint_after_secs(1.0); // Necessary to keep UI up-to-date with the current state of the sink/player.
        update_theme(self, ctx);
        render_gem_player(self, ctx);
        self.ui_state.toasts.show(ctx);
    }
}

pub fn read_tracks_and_playlists_from_directory(directory: &Path) -> (Vec<Track>, Vec<Playlist>) {
    let mut library = Vec::new();
    let mut playlists = Vec::new();

    match read_in_tracks_from_directory(directory) {
        Ok(found_tracks) => {
            library = found_tracks;
        }
        Err(e) => {
            error!("{}", e);
        }
    }

    match read_all_from_a_directory(directory) {
        Ok(found_playlists) => {
            playlists = found_playlists;
        }
        Err(e) => {
            error!("{}", e);
        }
    }

    info!(
        "Loaded library from {:?}: {} tracks, {} playlists.",
        directory,
        library.len(),
        playlists.len()
    );

    (library, playlists)
}

pub fn check_for_next_track(gem_player: &mut GemPlayer) {
    if !gem_player.player.sink.empty() {
        return; // If a track is still playing, do nothing
    }

    let result = play_next(&mut gem_player.player);
    if let Err(e) = result {
        error!("{}", e);
        gem_player.ui_state.toasts.error("Error playing the next track");
    }
}

pub fn maybe_play_next(gem_player: &mut GemPlayer) { // maybe just remove this.
    let result = play_next(&mut gem_player.player);
    if let Err(e) = result {
        error!("{}", e);
        gem_player.ui_state.toasts.error("Error playing the next track");
    }
}

// If we are near the beginning of the track, we go to the previously played track.
// Otherwise, we seek to the beginning.
// This is what actually gets called by the UI and key command.
pub fn maybe_play_previous(gem_player: &mut GemPlayer) {
    let playback_position = gem_player.player.sink.get_pos().as_secs_f32();
    let rewind_threshold = 5.0;

    let under_threshold = playback_position < rewind_threshold;
    let previous_track_exists = !gem_player.player.history.is_empty();

    let can_go_previous = under_threshold && previous_track_exists;
    if can_go_previous {
        if let Err(e) = play_previous(&mut gem_player.player) {
            error!("{}", e);
            gem_player.ui_state.toasts.error("Error playing the previous track");
        }
    } else {
        if let Err(e) = gem_player.player.sink.try_seek(Duration::ZERO) {
            error!("Error rewinding track: {:?}", e);
        }
        gem_player.player.sink.play();
    }
}

pub fn play_library(gem_player: &mut GemPlayer, starting_track: Option<&Track>) -> Result<(), String> {
    clear_the_queue(&mut gem_player.player);

    let mut start_index = 0;
    if let Some(track) = starting_track {
        start_index = gem_player.library.get_position_by_path(&track.path);
    }

    // Add tracks from the starting index to the end. Then add tracks from the beginning up to the starting index.
    for i in start_index..gem_player.library.len() {
        gem_player.player.queue.push(gem_player.library[i].clone());
    }
    for i in 0..start_index {
        gem_player.player.queue.push(gem_player.library[i].clone());
    }

    play_next(&mut gem_player.player)?;

    Ok(())
}

pub fn play_playlist(gem_player: &mut GemPlayer, playlist_key: &Path, starting_track_key: Option<&Path>) -> Result<(), String> {
    clear_the_queue(&mut gem_player.player);

    let playlist = gem_player.playlists.get_by_path(playlist_key);

    let mut start_index = 0;
    if let Some(key) = starting_track_key {
        start_index = playlist.tracks.get_position_by_path(key);
    }

    // Add tracks from the starting index to the end, then from the beginning up to the starting index.
    for i in start_index..playlist.tracks.len() {
        gem_player.player.queue.push(playlist.tracks[i].clone());
    }
    for i in 0..start_index {
        gem_player.player.queue.push(playlist.tracks[i].clone());
    }

    play_next(&mut gem_player.player)?;

    Ok(())
}

lazy_static! {
    pub static ref KEY_COMMANDS: IndexMap<Key, &'static str> = {
        let mut map = IndexMap::new();

        map.insert(Key::Space, "Play/Pause");
        map.insert(Key::ArrowLeft, "Previous");
        map.insert(Key::ArrowRight, "Next");
        map.insert(Key::ArrowUp, "Volume Up");
        map.insert(Key::ArrowDown, "Volume Down");
        map.insert(Key::M, "Mute/Unmute");

        map
    };
}

pub fn handle_key_commands(ctx: &Context, gem_player: &mut GemPlayer) {
    if ctx.wants_keyboard_input() {
        return;
    }

    ctx.input(|i| {
        for event in &i.events {
            if let Event::Key {
                key,
                pressed: true,
                physical_key: _,
                repeat: _,
                modifiers: _,
            } = event
            {
                let Some(binding) = KEY_COMMANDS.get(key) else {
                    continue;
                };

                info!("Key pressed: {}", binding);

                match key {
                    Key::Space => play_or_pause(&mut gem_player.player),
                    Key::ArrowLeft => maybe_play_previous(gem_player),
                    Key::ArrowRight => maybe_play_next(gem_player),
                    Key::ArrowUp => adjust_volume_by_percentage(&mut gem_player.player, 0.1),
                    Key::ArrowDown => adjust_volume_by_percentage(&mut gem_player.player, -0.1),
                    Key::M => mute_or_unmute(&mut gem_player.player),
                    _ => {}
                }
            }
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
