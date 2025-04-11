use eframe::egui::{
    Color32, Context, Event, FontData, FontDefinitions, FontFamily, Key, Rgba, ThemePreference, Vec2, ViewportBuilder, Visuals,
};
use egui_notify::Toasts;
use font_kit::{family_name::FamilyName, handle::Handle, properties::Properties, source::SystemSource};
use fully_pub::fully_pub;
use log::{debug, error, info, warn};
use player::{adjust_volume_by_percentage, clear_the_queue, mute_or_unmute, play_next, play_or_pause, play_previous, Player};
use playlist::{read_all_from_a_directory, Playlist, PlaylistRetrieval};
use rodio::{OutputStream, Sink};
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    sync::Arc,
    time::{Duration, Instant},
};
use track::{read_in_tracks_from_directory, SortBy, SortOrder, Track, TrackRetrieval};
use ui::{maybe_update_theme, render_gem_player, LibraryViewState, MarqueeState, PlaylistsViewState, UIState, View};

mod player;
mod playlist;
mod track;
mod ui;

/*
TODO:
* Music Visualizer. https://github.com/RustAudio/rodio/issues/722#issuecomment-2761176884
* could use egui_inbox for library updating with watcher. should expensive operations such as opening a file use an async system? research this!
*/

pub const LIBRARY_DIRECTORY_STORAGE_KEY: &str = "library_directory";
pub const THEME_STORAGE_KEY: &str = "theme";
pub const VOLUME_STORAGE_KEY: &str = "volume";

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

    let icon_data = eframe::icon_data::from_png_bytes(include_bytes!("../assets/icon.png")).expect("The icon data must be valid");

    let options = eframe::NativeOptions {
        viewport: ViewportBuilder::default()
            .with_min_inner_size(Vec2::new(900.0, 500.0))
            .with_decorations(false)
            .with_transparent(true)
            .with_icon(icon_data),
        ..Default::default()
    };
    eframe::run_native("Gem Player", options, Box::new(|cc| Ok(Box::new(init_gem_player(cc)))))
}

pub fn init_gem_player(cc: &eframe::CreationContext<'_>) -> GemPlayer {
    egui_extras::install_image_loaders(&cc.egui_ctx);
    egui_material_icons::initialize(&cc.egui_ctx);

    let mut fonts = FontDefinitions::default();
    let font_key = "inconsolata";
    fonts.font_data.insert(
        font_key.to_owned(),
        Arc::new(FontData::from_static(include_bytes!(
            "../assets/Inconsolata-VariableFont_wdth,wght.ttf"
        ))),
    );
    fonts
        .families
        .entry(FontFamily::Proportional)
        .or_default()
        .insert(0, font_key.to_owned());

    load_system_fonts(&mut fonts);
    cc.egui_ctx.set_fonts(fonts);

    let (stream, handle) = OutputStream::try_default().expect("Failed to initialize audio output");
    let sink = Sink::try_new(&handle).expect("Failed to create sink");
    sink.pause();

    let mut library_directory = None;
    let mut theme_preference = ThemePreference::System;
    let mut initial_volume = 0.6; // If this is the first run, we want a reasonable default.

    if let Some(storage) = cc.storage {
        if let Some(library_directory_string) = storage.get_string(LIBRARY_DIRECTORY_STORAGE_KEY) {
            library_directory = Some(PathBuf::from(library_directory_string));
        }

        if let Some(theme_string) = storage.get_string(THEME_STORAGE_KEY) {
            if let Ok(theme) = ron::from_str(&theme_string) {
                theme_preference = theme;
            }
        }

        if let Some(volume_string) = storage.get_string(VOLUME_STORAGE_KEY) {
            if let Ok(volume) = ron::from_str::<f32>(&volume_string) {
                initial_volume = volume.clamp(0.0, 1.0);
            }
        }
    }

    sink.set_volume(initial_volume);

    let mut library = Vec::new();
    let mut playlists = Vec::new();
    if let Some(directory) = &library_directory {
        let (found_tracks, found_playlists) = read_tracks_and_playlists_from_directory(directory);
        library = found_tracks;
        playlists = found_playlists;
    }

    GemPlayer {
        ui_state: UIState {
            current_view: View::Library,
            theme_preference,
            theme_dirty: true,
            search: String::new(),
            library: LibraryViewState {
                cached_library: Vec::new(),
                cache_dirty: true,
                selected_track_key: None,
                sort_by: SortBy::Title,
                sort_order: SortOrder::Ascending,
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
            },
            toasts: Toasts::default()
                .with_anchor(egui_notify::Anchor::BottomRight)
                .with_shadow(eframe::egui::Shadow {
                    offset: [0, 0],
                    blur: 1,
                    spread: 1,
                    color: Color32::BLACK,
                }),
            marquee: MarqueeState {
                offset: 0,
                track_identifier: None,
                last_update: Instant::now(),
                next_update: Instant::now(),
                pause_until: None,
            },
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

        let volume_ron_string = ron::to_string(&self.player.sink.volume()).unwrap();
        storage.set_string(VOLUME_STORAGE_KEY, volume_ron_string);
    }

    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        handle_key_commands(ctx, self);

        check_for_next_track(self);

        // ctx.set_debug_on_hover(true); // For debugging.
        maybe_update_theme(self, ctx);
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

pub fn maybe_play_next(gem_player: &mut GemPlayer) {
    // maybe just remove this.
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

const KEY_COMMANDS: &[(Key, &str)] = &[
    (Key::Space, "Play/Pause"),
    (Key::ArrowLeft, "Previous"),
    (Key::ArrowRight, "Next"),
    (Key::ArrowUp, "Volume Up"),
    (Key::ArrowDown, "Volume Down"),
    (Key::M, "Mute/Unmute"),
];

pub fn handle_key_commands(ctx: &Context, gem_player: &mut GemPlayer) {
    if ctx.wants_keyboard_input() {
        return;
    }

    ctx.input(|i| {
        for event in &i.events {
            if let Event::Key { key, pressed: true, .. } = event {
                let Some(description) = KEY_COMMANDS.iter().find_map(|(k, desc)| (k == key).then_some(*desc)) else {
                    continue;
                };

                info!("Key pressed: {}", description);

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

fn load_font_family(family_names: &[&str]) -> Option<Vec<u8>> {
    let system_source = SystemSource::new();

    for &name in family_names {
        let result = system_source.select_best_match(&[FamilyName::Title(name.to_string())], &Properties::new());
        match result {
            Err(e) => {
                warn!("Could not load {}: {:?}", name, e);
                continue;
            }
            Ok(handle) => match handle {
                Handle::Memory { ref bytes, .. } => {
                    debug!("Loaded {name} from memory.");
                    return Some(bytes.to_vec());
                }
                Handle::Path { ref path, .. } => {
                    info!("Loaded {name} from path: {:?}", path);
                    if let Ok(data) = fs::read(path) {
                        return Some(data);
                    } else {
                        error!("Failed to read font data from path: {:?}", path);
                    }
                }
            },
        }
    }

    None
}

// Loads system fonts as fallbacks for various language regions and adds them to the provided `FontDefinitions`.
pub fn load_system_fonts(fonts: &mut FontDefinitions) {
    let mut fontdb: HashMap<&str, Vec<&str>> = HashMap::new(); // Map of region identifiers to a list of candidate system font names.

    fontdb.insert(
        "simplified_chinese",
        vec![
            "Heiti SC",
            "Songti SC",
            "Noto Sans CJK SC", // Good coverage for Simplified Chinese
            "Noto Sans SC",
            "WenQuanYi Zen Hei", // Includes both Simplified and Traditional Chinese.
            "SimSun",
            "PingFang SC",
            "Source Han Sans CN",
        ],
    );
    fontdb.insert("korean", vec!["Source Han Sans KR"]);
    fontdb.insert("arabic_fonts", vec!["Noto Sans Arabic", "Amiri", "Lateef", "Al Tarikh", "Segoe UI"]);
    // Add more regions and their candidate font names as needed...

    // Iterate over each region and try to load a matching system font.
    for (region, font_names) in fontdb.iter() {
        if let Some(font_data) = load_font_family(font_names) {
            info!("Inserting font fallback for region: {region}");
            fonts.font_data.insert(region.to_string(), FontData::from_owned(font_data).into());

            // Add the region key as a fallback font in the proportional family.
            // This means that if the primary font is missing a glyph, egui will try this fallback.
            if let Some(proportional) = fonts.families.get_mut(&FontFamily::Proportional) {
                proportional.push(region.to_string());
            } else {
                fonts.families.insert(FontFamily::Proportional, vec![region.to_string()]);
            }
        }
    }
}
