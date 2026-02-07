#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // Hides console for Windows release

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
compile_error!("Gem Player only supports macOS and Windows.");

use dark_light::Mode;
use eframe::{
    egui::{
        Color32, Context, DroppedFile, Event, FontData, FontDefinitions, FontFamily, Key, Rgba, Shadow, ThemePreference, Vec2,
        ViewportBuilder, Visuals,
    },
    icon_data, run_native, App, CreationContext, Frame, NativeOptions, Storage,
};
use egui_notify::Toasts;
use font_kit::{family_name::FamilyName, handle::Handle, properties::Properties, source::SystemSource};
use fully_pub::fully_pub;
use library_watcher::{setup_library_watcher, LibraryAndPlaylists, LibraryWatcherCommand};
use log::{debug, error, info, warn};
use mimalloc::MiMalloc;
use player::{
    adjust_volume_by_percentage, build_audio_backend_from_device, get_audio_output_devices_and_names, mute_or_unmute, play_next,
    play_or_pause, play_previous, Player, VisualizerState,
};
use playlist::Playlist;
use rfd::FileDialog;
use rodio::cpal::{default_host, traits::HostTrait};
use std::{
    collections::HashMap,
    fs, io,
    path::{Path, PathBuf},
    sync::{
        mpsc::{channel, Receiver, Sender, TryRecvError},
        Arc,
    },
    thread,
    time::Duration,
};
use track::{is_relevant_media_file, SortBy, SortOrder, Track};
use visualizer::{setup_visualizer_pipeline, CENTER_FREQUENCIES};

use crate::ui::{
    library_view::LibraryViewState,
    playlist_view::PlaylistsViewState,
    root::{gem_player_ui, UIState, View},
    settings_view::SettingsViewState,
    widgets::marquee::Marquee,
};

mod custom_window;
mod library_watcher;
mod player;
mod playlist;
mod track;
mod ui;
mod visualizer;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

pub const LIBRARY_DIRECTORY_STORAGE_KEY: &str = "library_directory";
pub const THEME_STORAGE_KEY: &str = "theme";
pub const VOLUME_STORAGE_KEY: &str = "volume";

#[fully_pub]
struct GemPlayer {
    ui: UIState,

    library: Vec<Track>,
    playlists: Vec<Playlist>,

    library_directory: Option<PathBuf>,
    folder_picker_receiver: Option<Receiver<Option<PathBuf>>>, // None -> No folder picker dialog. Some -> Folder picker dialog open.
    library_watcher: LibraryWatcher,

    player: Player,
}

#[fully_pub]
struct LibraryWatcher {
    command_sender: Sender<LibraryWatcherCommand>,
    update_receiver: Receiver<Option<LibraryAndPlaylists>>,
}

fn main() -> eframe::Result {
    env_logger::init(); // Log to stderr (if run with `RUST_LOG=debug`).
    info!("Starting up Gem Player.");

    let icon_data = icon_data::from_png_bytes(include_bytes!("../assets/icon.png")).expect("The icon data must be valid");

    let options = NativeOptions {
        viewport: ViewportBuilder::default()
            .with_min_inner_size(Vec2::new(900.0, 500.0))
            .with_decorations(false)
            .with_transparent(true)
            .with_icon(icon_data),
        ..Default::default()
    };
    run_native("Gem Player", options, Box::new(|cc| Ok(Box::new(init_gem_player(cc)))))
}

pub fn init_gem_player(cc: &CreationContext<'_>) -> GemPlayer {
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

    let mut backend = None;

    let host = default_host();
    if let Some(device) = host.default_output_device() {
        let backend_result = build_audio_backend_from_device(device);
        match backend_result {
            Ok(b) => backend = Some(b),
            Err(e) => error!("Failed to start audio device: {}", e),
        }
    }
    let audio_output_devices_cache = get_audio_output_devices_and_names();

    let (visualizer_command_sender, bands_receiver) = setup_visualizer_pipeline();

    let mut library_directory = None;
    let mut theme_preference = ThemePreference::System;
    let mut initial_volume = 0.6; // If this is the first run, we want a reasonable default.

    if let Some(storage) = cc.storage {
        if let Some(library_directory_string) = storage.get_string(LIBRARY_DIRECTORY_STORAGE_KEY) {
            library_directory = Some(PathBuf::from(library_directory_string));
        }

        if let Some(theme_string) = storage.get_string(THEME_STORAGE_KEY) {
            if let Ok(theme) = serde_json::from_str(&theme_string) {
                theme_preference = theme;
            }
        }

        if let Some(volume_string) = storage.get_string(VOLUME_STORAGE_KEY) {
            if let Ok(volume) = serde_json::from_str::<f32>(&volume_string) {
                initial_volume = volume.clamp(0.0, 1.0);
            }
        }
    }

    let mut library_and_playlists_are_loading = false;
    let (watcher_command_sender, update_receiver) = setup_library_watcher().expect("Failed to initialize library watcher.");
    if let Some(directory) = &library_directory {
        let command = LibraryWatcherCommand::SetPath(directory.clone());
        if let Err(e) = watcher_command_sender.send(command) {
            error!("Failed to start watching library directory: {e}");
            library_directory = None;
        } else {
            library_and_playlists_are_loading = true;
        }
    }

    apply_theme(&cc.egui_ctx, theme_preference);

    if let Some(b) = &backend {
        b.sink.set_volume(initial_volume);
    }

    GemPlayer {
        ui: UIState {
            current_view: View::Library,
            theme_preference,
            search: String::new(),
            cached_track_key: None,
            library: LibraryViewState {
                cached_library: None,
                selected_tracks: Vec::new(),
                sort_by: SortBy::Title,
                sort_order: SortOrder::Ascending,
            },
            playlists: PlaylistsViewState {
                selected_playlist_key: None,
                cached_playlist_tracks: None,
                rename_buffer: None,
                delete_modal_open: false,
                selected_tracks: Vec::new(),
            },
            settings: SettingsViewState {
                audio_output_devices_cache,
            },
            library_and_playlists_are_loading,
            toasts: Toasts::default().with_anchor(egui_notify::Anchor::BottomRight).with_shadow(Shadow {
                offset: [0, 0],
                blur: 1,
                spread: 1,
                color: Color32::BLACK,
            }),
            marquee: Marquee::new(),
            volume_popup_is_open: false,
        },

        library: Vec::new(),
        playlists: Vec::new(),

        library_directory,
        folder_picker_receiver: None,
        library_watcher: LibraryWatcher {
            update_receiver,
            command_sender: watcher_command_sender,
        },
        player: Player {
            history: Vec::new(),
            playing: None,
            queue: Vec::new(),

            repeat: false,
            shuffle: None,
            muted: false,
            volume_before_mute: None,
            paused_before_scrubbing: None,

            backend,
            raw_artwork: None,
            visualizer: VisualizerState {
                command_sender: visualizer_command_sender,
                bands_receiver,
                display_bands: vec![0.0; CENTER_FREQUENCIES.len()],
            },
        },
    }
}

impl App for GemPlayer {
    fn clear_color(&self, _visuals: &Visuals) -> [f32; 4] {
        Rgba::TRANSPARENT.to_array() // Make sure we don't paint anything behind the rounded corners
    }

    // This is set because egui was persisting the state of the library table scroll position across runs.
    fn persist_egui_memory(&self) -> bool {
        false
    }

    fn save(&mut self, storage: &mut dyn Storage) {
        if let Some(library_directory) = &self.library_directory {
            storage.set_string(LIBRARY_DIRECTORY_STORAGE_KEY, library_directory.to_string_lossy().to_string());
        }

        let theme_json_string = serde_json::to_string(&self.ui.theme_preference).unwrap();
        storage.set_string(THEME_STORAGE_KEY, theme_json_string);

        if let Some(backend) = &self.player.backend {
            let volume_json_string = serde_json::to_string(&backend.sink.volume()).unwrap();
            storage.set_string(VOLUME_STORAGE_KEY, volume_json_string);
        }
    }

    fn update(&mut self, ctx: &Context, _frame: &mut Frame) {
        // Input
        handle_key_commands(ctx, self);

        // Update
        check_for_next_track(self);
        poll_library_watcher_messages(self);
        poll_folder_picker(self);

        // Render
        gem_player_ui(self, ctx);
        self.ui.toasts.show(ctx);

        // Set a minimum refresh rate for the app to keep the ui elements updated.
        ctx.request_repaint_after(Duration::from_millis(33)); // ~30 fps
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        if let Some(backend) = &self.player.backend {
            backend.sink.stop();
        }
        let _ = self.player.visualizer.command_sender.send(visualizer::VisualizerCommand::Shutdown);
        let _ = self.library_watcher.command_sender.send(LibraryWatcherCommand::Shutdown);
    }
}

fn poll_library_watcher_messages(gem: &mut GemPlayer) {
    let update = gem.library_watcher.update_receiver.try_recv();
    match update {
        Ok(Some((library, playlists))) => {
            on_library_reloaded(gem, library, playlists);
        }
        Ok(None) => {
            let message = "Failed to load library folder.";
            error!("{}", message);
            gem.ui.toasts.error(message);

            gem.library_directory = None;
            gem.ui.library_and_playlists_are_loading = false;
        }
        Err(TryRecvError::Empty) => {} // no update available this frame
        Err(TryRecvError::Disconnected) => {
            error!("Library watcher has disconnected.");
            gem.ui.library_and_playlists_are_loading = false;
        }
    }
}

// Reset / reconcile the relevant ui state so that we don't become out of sync.
// For example, have selected a playlist that has since been deleted.
pub fn on_library_reloaded(gem: &mut GemPlayer, new_library: Vec<Track>, new_playlists: Vec<Playlist>) {
    gem.library = new_library;
    gem.playlists = new_playlists;

    gem.ui.library.cached_library = None;
    gem.ui.playlists.cached_playlist_tracks = None;

    // Reconcile the selected tracks in the library view.
    gem.ui
        .library
        .selected_tracks
        .retain(|track_id| gem.library.iter().any(|t| &t.path == track_id));

    // Reconcile the playlist selection + playlist-selected tracks in the playlist view.
    if let Some(selected_playlist_key) = &gem.ui.playlists.selected_playlist_key {
        let maybe_playlist = gem.playlists.iter().find(|p| &p.m3u_path == selected_playlist_key);
        if let Some(playlist) = maybe_playlist {
            // Playlist still exists -> reconcile selected tracks
            gem.ui
                .playlists
                .selected_tracks
                .retain(|track_id| playlist.tracks.iter().any(|t| &t.path == track_id));
        } else {
            // Playlist no longer exists -> reset playlist UI state
            gem.ui.playlists.selected_playlist_key = None;
            gem.ui.playlists.selected_tracks.clear();
            gem.ui.playlists.rename_buffer = None;
            gem.ui.playlists.delete_modal_open = false;
        }
    } else {
        gem.ui.playlists.selected_tracks.clear();
    }

    gem.ui.library_and_playlists_are_loading = false;
}

pub fn handle_dropped_file(dropped_file: &DroppedFile, gem: &mut GemPlayer) -> io::Result<()> {
    let Some(path) = dropped_file.path.as_ref() else {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "Dropped file has no path"));
    };

    let Some(library_path) = gem.library_directory.as_ref() else {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "No library directory set"));
    };

    let Some(file_name) = path.file_name() else {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "Dropped file has no file name"));
    };

    if !is_relevant_media_file(path) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Dropped file is not a relevant media file",
        ));
    }

    let destination = library_path.join(file_name);
    fs::copy(path, destination)?;

    Ok(())
}

fn poll_folder_picker(gem: &mut GemPlayer) {
    let Some(receiver) = &gem.folder_picker_receiver else {
        return;
    };

    match receiver.try_recv() {
        Ok(maybe_directory) => {
            gem.folder_picker_receiver = None;

            if let Some(directory) = maybe_directory {
                info!("Selected folder: {:?}", directory);

                let command = LibraryWatcherCommand::SetPath(directory.clone());
                let result = gem.library_watcher.command_sender.send(command);
                if result.is_err() {
                    let message = "Failed to start watching library directory. Reverting back to old directory.";
                    error!("{}", message);
                    gem.ui.toasts.error(message);
                } else {
                    gem.library_directory = Some(directory);
                    gem.ui.library_and_playlists_are_loading = true;
                }
            } else {
                info!("No folder selected");
            }
        }
        Err(std::sync::mpsc::TryRecvError::Empty) => {} // folder picker is still open.
        Err(std::sync::mpsc::TryRecvError::Disconnected) => {
            error!("Folder picker channel disconnected unexpectedly.");
            gem.folder_picker_receiver = None;
        }
    }
}

/// Spawns a folder picker in a background thread and returns the receiver where the selected folder will eventually be sent.
fn spawn_folder_picker(start_dir: &Path) -> Receiver<Option<PathBuf>> {
    let (sender, receiver) = channel();
    let start_dir = start_dir.to_path_buf();

    thread::spawn(move || {
        let maybe_directory = FileDialog::new().set_directory(start_dir).pick_folder().map(|p| p.to_path_buf());
        let _ = sender.send(maybe_directory);
    });

    receiver
}

fn check_for_next_track(gem: &mut GemPlayer) {
    let Some(backend) = &gem.player.backend else {
        return;
    };

    if !backend.sink.empty() {
        return; // If a track is still playing, do nothing
    }

    let result = play_next(&mut gem.player);
    if let Err(e) = result {
        error!("{}", e);
        gem.ui.toasts.error("Error playing the next track");
    }
}

fn maybe_play_next(gem: &mut GemPlayer) {
    let result = play_next(&mut gem.player);
    if let Err(e) = result {
        error!("{}", e);
        gem.ui.toasts.error("Error playing the next track");
    }
}

// If we are near the beginning of the track, we go to the previously played track.
// Otherwise, we seek to the beginning.
// This is what actually gets called by the UI and key command.
pub fn maybe_play_previous(gem: &mut GemPlayer) {
    let rewind_threshold = 5.0;
    let mut under_threshold = false;

    if let Some(backend) = &gem.player.backend {
        let playback_position = backend.sink.get_pos().as_secs_f32();
        under_threshold = playback_position < rewind_threshold;
    }

    let previous_track_exists = !gem.player.history.is_empty();

    let can_go_previous = under_threshold && previous_track_exists;
    if can_go_previous {
        if let Err(e) = play_previous(&mut gem.player) {
            error!("{}", e);
            gem.ui.toasts.error("Error playing the previous track");
        }
    } else if let Some(backend) = &gem.player.backend {
        if let Err(e) = backend.sink.try_seek(Duration::ZERO) {
            error!("Error rewinding track: {:?}", e);
        }
        backend.sink.play();
    }
}

const KEY_COMMANDS: &[(Key, &str)] = &[
    (Key::Space, "Play/Pause"),
    (Key::ArrowLeft, "Previous"),
    (Key::ArrowRight, "Next"),
    (Key::ArrowUp, "Volume Up"),
    (Key::ArrowDown, "Volume Down"),
    (Key::M, "Mute/Unmute"),
];

pub fn handle_key_commands(ctx: &Context, gem: &mut GemPlayer) {
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
                    Key::Space => {
                        if let Some(backend) = &mut gem.player.backend {
                            play_or_pause(&mut backend.sink);
                        }
                    }
                    Key::ArrowLeft => maybe_play_previous(gem),
                    Key::ArrowRight => maybe_play_next(gem),
                    Key::ArrowUp => {
                        if let Some(backend) = &mut gem.player.backend {
                            adjust_volume_by_percentage(&mut backend.sink, 0.1);
                        }
                    }
                    Key::ArrowDown => {
                        if let Some(backend) = &mut gem.player.backend {
                            adjust_volume_by_percentage(&mut backend.sink, -0.1);
                        }
                    }
                    Key::M => mute_or_unmute(&mut gem.player),
                    _ => {}
                }
            }
        }
    });
}

pub fn apply_theme(ctx: &Context, preference: ThemePreference) {
    let visuals = match preference {
        ThemePreference::Dark => Visuals::dark(),
        ThemePreference::Light => Visuals::light(),
        ThemePreference::System => match dark_light::detect() {
            Ok(Mode::Light) => Visuals::light(),
            _ => Visuals::dark(),
        },
    };

    ctx.set_visuals(visuals);
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

/// Loads system fonts as fallbacks for various language regions and adds them to the provided `FontDefinitions`.
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
            info!("Inserting font fallback for region: {region}.");
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
