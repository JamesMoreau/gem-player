use crate::song::Song;
use eframe::egui::{self, Vec2};
use glob::glob;
use rodio::{Decoder, OutputStream, Sink};
use std::io::BufReader;
use std::path::{Path, PathBuf};
use strum::IntoEnumIterator;
use strum_macros::EnumIter;

mod constants;
mod song;
mod ui;
mod utils;

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
        Box::new(|cc| Ok(Box::new(GemPlayer::new(cc)))),
    )
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

struct GemPlayer {
    current_view: ui::View,
    songs: Vec<Song>,

    music_directory: Option<PathBuf>,

    selected_song: Option<usize>, // Index of the selected song in the songs vector.
    queue: Vec<Song>,
    current_song: Option<Song>, // The currently playing song.
    _stream: OutputStream,      // Holds the OutputStream to keep it alive
    sink: Sink,                 // Controls playback (play, pause, stop, etc.)
    
    muted: bool,
    volume_before_mute: Option<f32>,

    paused_before_scrubbing: Option<bool>, // None if not scrubbing, Some(true) if paused, Some(false) if playing.

    search_text: String,
    sort_by: SortBy,
    sort_order: SortOrder,

    theme: String,
}

impl GemPlayer {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        egui_extras::install_image_loaders(&cc.egui_ctx);

        egui_material_icons::initialize(&cc.egui_ctx);

        let (_stream, handle) = OutputStream::try_default().unwrap();
        let sink = Sink::try_new(&handle).unwrap();
        sink.pause();
        sink.set_volume(0.6);

        let mut default_self = Self {
            current_view: ui::View::Library,
            songs: Vec::new(),
            selected_song: None,
            queue: Vec::new(),
            search_text: String::new(),
            music_directory: None,
            sort_by: SortBy::Title,
            sort_order: SortOrder::Ascending,
            current_song: None,
            // queue: Vec::new(),
            _stream,
            sink,
            muted: false,
            volume_before_mute: None,
            paused_before_scrubbing: None,
            theme: "Default".to_owned(),
        };

        // Find the music directory.
        let audio_directory = match dirs::audio_dir() {
            Some(dir) => dir,
            None => {
                println!("No music directory found.");
                return default_self;
            }
        };
        let my_music_directory = audio_directory.join("MyMusic");
        default_self.music_directory = Some(my_music_directory);

        let songs = match &default_self.music_directory {
            Some(path) => read_music_from_directory(path),
            None => Vec::new(),
        };
        println!("Found {} songs", &songs.len());
        sort_songs(
            &mut default_self.songs,
            default_self.sort_by,
            default_self.sort_order,
        );

        Self {
            songs,
            ..default_self
        }
    }

    fn is_playing(&self) -> bool {
        !self.sink.is_paused()
    }

    // TODO: Is this ok to call this function from the UI thread since we are doing heavy events like loading a file?
    fn load_and_play_song(&mut self, song: &Song) {
        self.sink.stop(); // Stop the current song if any.

        let file_result = std::fs::File::open(&song.file_path);
        let file = match file_result {
            Ok(file) => file,
            Err(e) => {
                println!("Error opening file: {:?}", e);
                return;
            }
        };

        let source_result = Decoder::new(BufReader::new(file));
        let source = match source_result {
            Ok(source) => source,
            Err(e) => {
                println!(
                    "Error decoding file: {}, Error: {:?}",
                    song.file_path.to_string_lossy(),
                    e
                );
                return;
            }
        };

        self.current_song = Some(song.clone());

        self.sink.append(source);
        self.sink.play();
    }

    fn play_or_pause(&mut self) {
        if self.sink.is_paused() {
            self.sink.play()
        } else {
            self.sink.pause()
        }
    }
}

impl eframe::App for GemPlayer {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        egui::Rgba::TRANSPARENT.to_array() // Make sure we don't paint anything behind the rounded corners
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Necessary to keep UI up-to-date with the current state of the sink/player.
        ctx.request_repaint_after_secs(1.0);
    
        ui::custom_window_frame(ctx, "", |ui| {
            let app_rect = ui.max_rect();
            
            let control_ui_height = 60.0;
            let control_rect = egui::Rect::from_min_max(
                app_rect.min,
                egui::pos2(app_rect.max.x, app_rect.min.y + control_ui_height),
            );
            
            let navigation_ui_height = 32.0;
            let navigation_rect = egui::Rect::from_min_max(
                egui::pos2(app_rect.min.x, app_rect.max.y - navigation_ui_height),
                app_rect.max,
            );
    
            let content_ui_rect = egui::Rect::from_min_max(
                egui::pos2(app_rect.min.x, control_rect.max.y),
                egui::pos2(app_rect.max.x, navigation_rect.min.y),
            );
    
            let mut control_ui = ui.new_child(egui::UiBuilder::new().max_rect(control_rect));
            ui::render_control_ui(&mut control_ui, self);
    
            let mut content_ui = ui.new_child(egui::UiBuilder::new().max_rect(content_ui_rect));
            match self.current_view {
                ui::View::Library => ui::render_songs_ui(&mut content_ui, self),
                ui::View::Queue => ui::render_queue_ui(&mut content_ui, self),
                ui::View::Playlists => {
                    content_ui.label("Playlists section coming soon.");
                }
                ui::View::Settings => ui::render_settings_ui(&mut content_ui, self),
            }
    
            let mut navigation_ui = ui.new_child(egui::UiBuilder::new().max_rect(navigation_rect));
            navigation_ui.horizontal_centered(|ui| {
                ui.add_space(16.0);
                for view in ui::View::iter() {
                    let response = ui.selectable_label(self.current_view == view, format!("{:?}", view));
                    if response.clicked() {
                        ui::switch_view(self, view);
                    }
                }
            });
        });
    }     
}

fn read_music_from_directory(path: &Path) -> Vec<Song> {
    let mut songs = Vec::new();
    let mut file_paths: Vec<PathBuf> = Vec::new();

    let patterns = constants::SUPPORTED_AUDIO_FILE_TYPES
        .iter()
        .map(|file_type| format!("{}/*.{}", path.to_string_lossy(), file_type))
        .collect::<Vec<String>>();

    for pattern in patterns {
        let file_paths_result = glob(&pattern);
        match file_paths_result {
            Ok(paths) => {
                for path in paths.filter_map(Result::ok) {
                    file_paths.push(path);
                }
            }
            Err(e) => {
                println!("Error reading pattern {}: {}", pattern, e);
            }
        }
    }

    if file_paths.is_empty() {
        println!("No music files found in directory: {:?}", path);
        return songs;
    }

    for entry in file_paths {
        let song_option = song::get_song_from_file(&entry);
        let song = match song_option {
            Some(song) => song,
            None => {
                println!("Error reading song from file: {:?}", entry);
                continue;
            }
        };
        songs.push(song);
    }

    songs
}

fn sort_songs(songs: &mut [Song], sort_by: SortBy, sort_order: SortOrder) {
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
