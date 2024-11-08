use crate::song::Song;
use eframe::egui::{self, TextureFilter, TextureOptions, Vec2};
use egui_extras::TableBuilder;
use glob::glob;
use rodio::{Decoder, OutputStream, Sink};
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::time::Duration;
use strum::IntoEnumIterator;
use strum_macros::EnumIter;

mod constants;
mod song;

/*
TODO:
- selection needs to be cleared when songs are sorted / filtered.
- play next song after current song ends
- loading icon when songs are being loaded.
- tab bar at the bottom for playlists, queue, settings, etc.
- should read_music_from_directory return a Result<Vec<Song>, Error> instead of Vec<Song>? Fix this once we allow custom music path.
- file watcher / update on change
- remove the toolbar / titlebar on window.

- In the controls ui we want the following:
  - Play button / Pause button, Next song, previous song
  - Volume slider
  - Current song info: song artwork, Playback progress slider, Visualizer, song title, artist, album, duration.
  - Repeat / Shuffle.
  - Queue
  - Sorting, Search bar

Perhaps we could have the top panel contain the searching and sorting controls, and the bottom panel contain the playback controls and the music visualizer.
Or, we could have the control ui (current song, playback progress, artwork, visualizer) on the top panel be stacked vertically.

Could remove object oriented programming and just have a struct with functions that take a mutable reference to self.
*/

fn main() -> eframe::Result {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_min_inner_size(Vec2::new(1000.0, 500.0)),
        // .with_titlebar_shown(false)
        // .with_title_shown(false)
        // .with_fullsize_content_view(true),
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
    songs: Vec<Song>,

    search_text: String,
    sort_by: SortBy,
    sort_order: SortOrder,

    music_directory: Option<PathBuf>,

    selected_song: Option<usize>, // Index of the selected song in the songs vector.
    // queue: Vec<Song>,
    current_song: Option<Song>, // The currently playing song.
    _stream: OutputStream,      // Holds the OutputStream to keep it alive
    sink: Sink,                 // Controls playback (play, pause, stop, etc.)
    muted: bool,
    volume_before_mute: Option<f32>,
    scrubbing: bool,
}

impl GemPlayer {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        egui_extras::install_image_loaders(&cc.egui_ctx);

        let mut fonts = egui::FontDefinitions::default();
        fonts.font_data.insert(
            "my_font".to_owned(),
            egui::FontData::from_static(include_bytes!(
                "../assets/Inconsolata-VariableFont_wdth,wght.ttf"
            )),
        );
        fonts
            .families
            .get_mut(&egui::FontFamily::Proportional)
            .unwrap()
            .insert(0, "my_font".to_owned());
        fonts
            .families
            .get_mut(&egui::FontFamily::Monospace)
            .unwrap()
            .push("my_font".to_owned());
        cc.egui_ctx.set_fonts(fonts);

        let (_stream, handle) = OutputStream::try_default().unwrap();
        let sink = Sink::try_new(&handle).unwrap();
        sink.pause();
        sink.set_volume(0.6);

        let mut default_self = Self {
            songs: Vec::new(),
            selected_song: None,
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
            scrubbing: false,
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
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Necessary to keep ui up to date with the current state of the sink / player.
        ctx.request_repaint_after_secs(1.0);

        // Control UI.
        egui::TopBottomPanel::top("top_panel")
            .resizable(false)
            // .min_height(48.0)
            .show(ctx, |ui| {
                egui::Frame::none().inner_margin(8.0).show(ui, |ui| {
                    ui.horizontal(|ui| {
                        let play_pause_icon = if self.is_playing() || self.scrubbing {
                            egui::include_image!(
                                "../assets/pause_24dp_E8EAED_FILL0_wght400_GRAD0_opsz24.svg"
                            )
                        } else {
                            egui::include_image!(
                                "../assets/play_arrow_24dp_E8EAED_FILL0_wght400_GRAD0_opsz24.svg"
                            )
                        };
                        let clicked = ui.add(egui::Button::image(play_pause_icon)).clicked();
                        if clicked {
                            self.play_or_pause();
                        }

                        ui.separator();

                        let mut volume = self.sink.volume();
                        let volume_icon = match volume {
                            v if v == 0.0 => egui::include_image!(
                                "../assets/volume_mute_24dp_E8EAED_FILL0_wght400_GRAD0_opsz24.svg"
                            ),
                            v if v < 0.5 => egui::include_image!(
                                "../assets/volume_down_24dp_E8EAED_FILL0_wght400_GRAD0_opsz24.svg"
                            ),
                            _ => egui::include_image!(
                                "../assets/volume_up_24dp_E8EAED_FILL0_wght400_GRAD0_opsz24.svg"
                            ),
                        };
                        let clicked = ui.add(egui::Button::image(volume_icon)).clicked();
                        if clicked {
                            self.muted = !self.muted;
                            if self.muted {
                                self.volume_before_mute = Some(volume);
                                volume = 0.0;
                            } else if let Some(v) = self.volume_before_mute {
                                volume = v;
                            }
                        }

                        let volume_slider = egui::Slider::new(&mut volume, 0.0..=1.0)
                            .trailing_fill(true)
                            .show_value(false);
                        let changed = ui.add(volume_slider).changed();
                        if changed {
                            self.muted = false;
                            self.volume_before_mute =
                                if volume == 0.0 { None } else { Some(volume) }
                        }

                        self.sink.set_volume(volume);

                        ui.separator();

                        let artwork_texture_options =
                            TextureOptions::LINEAR.with_mipmap_mode(Some(TextureFilter::Linear));
                        let artwork_size = egui::vec2(64.0, 64.0);
                        let default_artwork = egui::Image::new(egui::include_image!(
                            "../assets/music_note_24dp_E8EAED_FILL0_wght400_GRAD0_opsz24.svg"
                        ))
                        .texture_options(artwork_texture_options)
                        .fit_to_exact_size(artwork_size);

                        let artwork = self
                            .current_song
                            .as_ref()
                            .and_then(|song| song.artwork.as_ref())
                            .map(|artwork_bytes| {
                                let artwork_uri = format!(
                                    "bytes://artwork-{}",
                                    self.current_song
                                        .as_ref()
                                        .unwrap()
                                        .title
                                        .as_deref()
                                        .unwrap_or("default")
                                );

                                egui::Image::from_bytes(artwork_uri, artwork_bytes.clone())
                                    .texture_options(artwork_texture_options)
                                    .fit_to_exact_size(artwork_size)
                            })
                            .unwrap_or(default_artwork);

                        ui.add(artwork);

                        ui.vertical(|ui| {
                            let mut current_title = "None".to_string();
                            let mut current_artist = "None".to_string();
                            let mut current_duration = "0:00".to_string();

                            if let Some(song) = &self.current_song {
                                current_title =
                                    song.title.clone().unwrap_or("Unknown Title".to_string());
                                current_artist =
                                    song.artist.clone().unwrap_or("Unknown Artist".to_string());
                                current_duration = format_duration_to_mmss(song.duration);
                            }
                            ui.label(&current_title);
                            ui.label(&current_artist);
                            ui.label(&current_duration);

                            let mut playback_progress = 0.0;

                            if let Some(song) = &self.current_song {
                                let current_position_secs = self.sink.get_pos().as_secs();
                                let duration_secs = song.duration.as_secs();

                                // Avoid division by zero.
                                playback_progress = if duration_secs == 0 {
                                    0.0
                                } else {
                                    current_position_secs as f32 / duration_secs as f32
                                };
                            }

                            ui.style_mut().spacing.slider_width = 500.0;
                            let playback_progress_slider =
                                egui::Slider::new(&mut playback_progress, 0.0..=1.0)
                                    .trailing_fill(true)
                                    .show_value(false);

                            let response: egui::Response = ui.add(playback_progress_slider);

                            // We pause the audio during seeking to avoid scrubbing sound.
                            if response.dragged() {
                                self.scrubbing = true;
                                self.sink.pause();
                            }

                            if response.drag_stopped() {
                                if let Some(song) = &self.current_song {
                                    let new_position_secs =
                                        playback_progress * song.duration.as_secs_f32();
                                    let new_position = Duration::from_secs_f32(new_position_secs);

                                    if let Err(e) = self.sink.try_seek(new_position) {
                                        println!("Error seeking to new position: {:?}", e);
                                    }
                                }

                                // Resume playback after seeking
                                self.scrubbing = false;
                                self.sink.play();
                            }
                        });

                        ui.separator();

                        let filter_icon = egui::include_image!(
                            "../assets/filter_list_24dp_E8EAED_FILL0_wght400_GRAD0_opsz24.svg"
                        );
                        ui.menu_image_button(filter_icon, |ui| {
                            let mut should_sort_songs = false;

                            for sort_by in SortBy::iter() {
                                let response = ui.radio_value(
                                    &mut self.sort_by,
                                    sort_by,
                                    format!("{:?}", sort_by),
                                );
                                should_sort_songs |= response.clicked();
                            }

                            ui.separator();

                            for sort_order in SortOrder::iter() {
                                let response = ui.radio_value(
                                    &mut self.sort_order,
                                    sort_order,
                                    format!("{:?}", sort_order),
                                );
                                should_sort_songs |= response.clicked();
                            }

                            if should_sort_songs {
                                sort_songs(&mut self.songs, self.sort_by, self.sort_order);
                            }
                        });

                        let search_bar = egui::TextEdit::singleline(&mut self.search_text)
                            .hint_text("Search...")
                            .desired_width(200.0);
                        ui.add(search_bar);
                    });
                });
            });

        // Songs list.
        egui::CentralPanel::default().show(ctx, |ui| {
            let filtered_songs: Vec<Song> = self
                .songs
                .iter()
                .filter(|song| {
                    let search_lower = self.search_text.to_lowercase();
                    let search_fields = [&song.title, &song.artist, &song.album];
                    search_fields.iter().any(|field| {
                        field
                            .as_ref()
                            .map_or(false, |text| text.to_lowercase().contains(&search_lower))
                    })
                })
                .cloned()
                .collect();

            let header_labels = ["Title", "Artist", "Album", "Time"];

            let available_width = ui.available_width();
            let time_width = 80.0;
            let remaining_width = available_width - time_width;
            let title_width = remaining_width * (2.0 / 4.0);
            let artist_width = remaining_width * (1.0 / 4.0);
            let album_width = remaining_width * (1.0 / 4.0);

            TableBuilder::new(ui)
                .striped(true)
                .sense(egui::Sense::click())
                .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                .column(egui_extras::Column::exact(title_width))
                .column(egui_extras::Column::exact(artist_width))
                .column(egui_extras::Column::exact(album_width))
                .column(egui_extras::Column::exact(time_width))
                .header(16.0, |mut header| {
                    for (i, h) in header_labels.iter().enumerate() {
                        header.col(|ui| {
                            if i == 0 {
                                ui.add_space(16.0);
                            }
                            ui.add(
                                egui::Label::new(egui::RichText::new(*h).strong())
                                    .selectable(false),
                            );
                        });
                    }
                })
                .body(|body| {
                    body.rows(26.0, filtered_songs.len(), |mut row| {
                        let song = &filtered_songs[row.index()];

                        row.set_selected(self.selected_song == Some(row.index()));

                        row.col(|ui| {
                            ui.add_space(16.0);
                            ui.add(
                                egui::Label::new(
                                    song.title.as_ref().unwrap_or(&"Unknown Title".to_string()),
                                )
                                .selectable(false),
                            );
                        });

                        row.col(|ui| {
                            ui.add(
                                egui::Label::new(
                                    song.artist
                                        .as_ref()
                                        .unwrap_or(&"Unknown Artist".to_string()),
                                )
                                .selectable(false),
                            );
                        });

                        row.col(|ui| {
                            ui.add(
                                egui::Label::new(
                                    song.album.as_ref().unwrap_or(&"Unknown".to_string()),
                                )
                                .selectable(false),
                            );
                        });

                        row.col(|ui| {
                            let duration_string = format_duration_to_mmss(song.duration);
                            ui.add(egui::Label::new(duration_string).selectable(false));
                        });

                        let response = row.response();
                        if response.clicked() {
                            self.selected_song = Some(row.index());
                        }

                        if response.double_clicked() {
                            self.load_and_play_song(song);
                        }

                        response.context_menu(|ui| {
                            if ui.button("Play").clicked() {
                                ui.close_menu();
                            }

                            if ui.button("Add to queue").clicked() {
                                ui.close_menu();
                            }

                            ui.separator();

                            if ui.button("Open file location").clicked() {
                                ui.close_menu();
                            }

                            if ui.button("Remove from library").clicked() {
                                ui.close_menu();
                            }
                        });
                    });
                });
        });
    }
}

fn format_duration_to_mmss(duration: std::time::Duration) -> String {
    let total_seconds: f64 = duration.as_secs_f64();
    let minutes = total_seconds / constants::SECONDS_PER_MINUTE as f64;
    let seconds = total_seconds % constants::SECONDS_PER_MINUTE as f64;

    format!("{:.0}:{:02.0}", minutes, seconds)
}

fn format_duration_to_hhmmss(duration: std::time::Duration) -> String {
    let total_seconds: f64 = duration.as_secs_f64();
    let hours =
        total_seconds / (constants::MINUTES_PER_HOUR as f64 * constants::SECONDS_PER_MINUTE as f64);
    let minutes =
        (total_seconds / constants::SECONDS_PER_MINUTE as f64) % constants::MINUTES_PER_HOUR as f64;
    let seconds = total_seconds % constants::SECONDS_PER_MINUTE as f64;
    format!("{:.0}:{:02.0}:{:02.0}", hours, minutes, seconds)
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
    let key = |song: &Song| match sort_by {
        SortBy::Title => song.title.as_deref().unwrap_or("").to_string(),
        SortBy::Artist => song.artist.as_deref().unwrap_or("").to_string(),
        SortBy::Album => song.album.as_deref().unwrap_or("").to_string(),
        SortBy::Time => song.duration.as_secs().to_string(),
    };

    songs.sort_by(|a, b| match sort_order {
        SortOrder::Ascending => key(a).cmp(&key(b)),
        SortOrder::Descending => key(a).cmp(&key(b)).reverse(),
    });
}
