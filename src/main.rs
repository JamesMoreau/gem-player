use eframe::egui::{Color32, Context, Event, Key, Rgba, ThemePreference, Vec2, ViewportBuilder, Visuals};
use egui_notify::Toasts;
use fully_pub::fully_pub;
use indexmap::IndexMap;
use lazy_static::lazy_static;
use log::{error, info};
use player::{
    adjust_volume_by_percentage, check_for_next_track, load_and_play, mute_or_unmute, play_or_pause, process_actions, Player,
    PlayerAction,
};
use playlist::{read_all_from_a_directory, Playlist};
use rodio::{OutputStream, Sink};
use std::path::{Path, PathBuf};
use track::{read_music, SortBy, SortOrder, Track};
use ui::{render_gem_player, update_theme, LibraryViewState, PlaylistsViewState, UIState};

mod player;
mod playlist;
mod track;
mod ui;

/*
TODO:
* library track menu: when a song is added to playlist then the menu closes, then another track menu is opened, the add to playlist dropdown is still open.
* could use egui_inbox for library updating with watcher.
* should expensive operations such as opening a file use an async system? research this!
* Music Visualizer.
* maybe make volume slider hover. Could make a new fat enum like muted, unmuted(volume)?
* profile app.
* Fullscreen?
* UI + aestethics
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

    let mut library = Vec::new();
    let mut playlists = Vec::new();
    if let Some(directory) = &library_directory {
        let (found_music, found_playlists) = read_music_and_playlists_from_directory(directory);

        library = found_music;
        playlists = found_playlists;
    }

    let (_stream, handle) = OutputStream::try_default().unwrap();
    let sink = Sink::try_new(&handle).unwrap();
    sink.pause();
    let initial_volume = 0.6;
    sink.set_volume(initial_volume);

    GemPlayer {
        ui_state: UIState {
            current_view: ui::View::Library,
            theme_preference,
            library_view_state: LibraryViewState {
                search_text: String::new(),
                selected_track: None,
                sort_by: SortBy::Title,
                sort_order: SortOrder::Ascending,
                track_menu_is_open: false,
            },
            playlists_view_state: PlaylistsViewState {
                selected_playlist: None,
                playlist_rename: None,
                delete_playlist_modal_is_open: None,
                selected_track: None,
                track_menu_is_open: false,
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
            actions: Vec::new(),
            playing_track: None,

            queue: Vec::new(),
            history: Vec::new(),

            repeat: false,
            muted: false,
            volume_before_mute: None,
            paused_before_scrubbing: None,

            _stream,
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
        handle_key_commands(ctx, &mut self.player);

        check_for_next_track(self);
        process_actions(self);

        ctx.request_repaint_after_secs(1.0); // Necessary to keep UI up-to-date with the current state of the sink/player.
        update_theme(self, ctx);
        render_gem_player(self, ctx);
        self.ui_state.toasts.show(ctx);
    }
}

pub fn read_music_and_playlists_from_directory(directory: &Path) -> (Vec<Track>, Vec<Playlist>) {
    let mut library = Vec::new();
    let mut playlists = Vec::new();

    match read_music(directory) {
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

pub fn play_library_from_track(gem_player: &mut GemPlayer, track: &Track) {
    gem_player.player.history.clear();
    gem_player.player.queue.clear();

    // Add all the other tracks to the queue.
    for t in &gem_player.library {
        if track == t {
            continue;
        }

        gem_player.player.queue.push(t.clone());
    }

    let result = load_and_play(&mut gem_player.player, track.clone());
    if let Err(e) = result {
        error!("{}", e);
        gem_player
            .ui_state
            .toasts
            .error(format!("Error playing {}", track.title.as_deref().unwrap_or("Unknown")));
    }
}

pub fn play_playlist_from_track(gem_player: &mut GemPlayer, playlist: &Playlist, track: &Track) {
    gem_player.player.history.clear();
    gem_player.player.queue.clear();

    // Add all the other tracks to the queue.
    for t in &playlist.tracks {
        if track == t {
            continue;
        }

        gem_player.player.queue.push(t.clone());
    }

    let result = load_and_play(&mut gem_player.player, track.clone());
    if let Err(e) = result {
        error!("{}", e);
        gem_player
            .ui_state
            .toasts
            .error(format!("Error playing {}", track.title.as_deref().unwrap_or("Unknown")));
    }
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

pub fn handle_key_commands(ctx: &Context, player: &mut Player) {
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
                    Key::Space => play_or_pause(player),
                    Key::ArrowLeft => player.actions.push(PlayerAction::PlayPrevious),
                    Key::ArrowRight => player.actions.push(PlayerAction::PlayNext),
                    Key::ArrowUp => adjust_volume_by_percentage(player, 0.1),
                    Key::ArrowDown => adjust_volume_by_percentage(player, -0.1),
                    Key::M => mute_or_unmute(player),
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
