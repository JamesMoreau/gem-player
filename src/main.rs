use crate::song::Song;
use eframe::egui::{self, Vec2};
use egui_extras::TableBuilder;
use glob::glob;
use std::path::{Path, PathBuf};
use strum::IntoEnumIterator;
use strum_macros::EnumIter;
use std::io::BufReader;
use rodio::{OutputStream, Sink, Decoder};

mod song;

/*
TODO:
- tab bar at the bottom for playlists, queue, settings, etc.
- should read_music_from_directory return a Result<Vec<Song>, Error> instead of Vec<Song>? Fix this once we allow custom music path.
- file watcher / update on change
- remove the toolbar / titlebar on window.

- In the controls ui we want the following:
  - Play button / Pause button
  - Next song, previous song
  - Volume slider
  - Current song info: song artwork, Playback progress slider, Visualizer, song title, artist, album, duration.
  - Repeat / Shuffle.
  - Queue
  - Sorting
  - Search bar

Perhaps we could have the top panel contain the searching and sorting controls, and the bottom panel contain the playback controls and the music visualizer.
Or, we could have the control ui (current song, playback progress, artwork, visualizer) on the top panel be stacked vertically.
*/

fn main() -> eframe::Result {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_min_inner_size(Vec2::new(900.0, 400.0)),
        // .with_inner_size([900.0, 600.0])
        // .with_titlebar_shown(false)
        // .with_title_shown(false)
        // .with_fullsize_content_view(true)
        ..Default::default()
    };
    eframe::run_native(
        "Gem Player",
        options,
        Box::new(|cc| Ok(Box::new(GemPlayer::new(cc)))),
    )
}

struct GemPlayer {
    age: u32,
    songs: Vec<Song>,

    search_text: String,
    sort_by: SortBy,
    sort_order: SortOrder,

    music_directory: Option<PathBuf>,

    selected_song: Option<usize>, // Index of the selected song in the songs vector.
    // current_song: usize,   // The currently playing song.
    volume: f32,
    _stream: OutputStream,    // Holds the OutputStream to keep it alive
    sink: Sink,               // Controls playback (play, pause, stop, etc.)
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

pub const SUPPORTED_AUDIO_FILE_TYPES: [&str; 6] = ["mp3", "m4a", "wav", "flac", "ogg", "opus"];

impl GemPlayer {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // This gives us image support:
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

        let mut default_self = Self {
            age: 42,
            songs: Vec::new(),
            selected_song: None,
            search_text: String::new(),
            volume: 0.0,
            music_directory: None,
            sort_by: SortBy::Title,
            sort_order: SortOrder::Ascending,
            // current_song: None,
            _stream,
            sink,
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

    fn load_song(&mut self, song: &Song) {
        let file = std::fs::File::open(&song.file_path).unwrap();
        let source = Decoder::new(BufReader::new(file)).unwrap();

        self.sink.append(source);
    }

    fn play_song(&mut self) {
        self.sink.play();
    }

    fn pause_song(&mut self) {
        self.sink.pause();
    }

    fn play_next_song(&mut self) {
        self.sink.stop();
        // self.current_song = (self.current_song + 1) % self.songs.len();
        // self.load_song(&self.songs[self.current_song]);
        // self.play_song();
    }
}

impl eframe::App for GemPlayer {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Control UI.
        egui::TopBottomPanel::top("top_panel")
            .resizable(false)
            .min_height(48.0)
            .show(ctx, |ui| {
                egui::Frame::none().inner_margin(8.0).show(ui, |ui| {
                    ui.horizontal(|ui| {
                        let play_icon = egui::include_image!(
                            "../assets/play_arrow_24dp_E8EAED_FILL0_wght400_GRAD0_opsz24.svg"
                        );
                        ui.add(egui::Button::image(play_icon));

                        ui.separator();

                        let volume_icon = egui::include_image!(
                            "../assets/volume_up_24dp_E8EAED_FILL0_wght400_GRAD0_opsz24.svg"
                        );
                        ui.add(egui::Button::image(volume_icon));

                        let volume_slider = egui::Slider::new(&mut self.volume, 0.0..=1.0)
                            .trailing_fill(true)
                            .show_value(false);
                        ui.add(volume_slider);

                        ui.separator();

                        ui.style_mut().spacing.slider_width = 500.0;
                        let playback_progress = egui::Slider::new(&mut self.age, 0..=100)
                            .trailing_fill(true)
                            .show_value(false);
                        ui.add(playback_progress);

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
            let search_lower = self.search_text.to_lowercase();
            let filtered_songs: Vec<&Song> = self
                .songs
                .iter()
                .filter(|song| {
                    let search_fields = [&song.title, &song.artist, &song.album];
                    search_fields.iter().any(|field| {
                        field
                            .as_ref()
                            .map_or(false, |text| text.to_lowercase().contains(&search_lower))
                    })
                })
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
                .body(|mut body| {
                    for (i, song) in filtered_songs.iter().enumerate() {
                        body.row(28.0, |mut row| {
                            row.set_selected(self.selected_song == Some(i));

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
                                self.selected_song = Some(i);
                            }

                            if response.double_clicked() {
                                println!("Play song: {:?}", song.title);
                                // self.load_song(song);
                            }

                            response.context_menu(|ui| {
                                if ui.button("Play").clicked() {
                                    println!("Play song");
                                    ui.close_menu();
                                }

                                if ui.button("Add to queue").clicked() {
                                    println!("Add to queue");
                                    ui.close_menu();
                                }

                                ui.separator();

                                if ui.button("Remove from library").clicked() {
                                    println!("Remove from library");
                                    ui.close_menu();
                                }
                            });
                        });
                    }
                });
        });
    }
}

fn format_duration_to_mmss(duration: std::time::Duration) -> String {
    let seconds_in_a_minute = 60.0;
    let total_seconds = duration.as_secs_f64();
    let minutes = total_seconds / seconds_in_a_minute;
    let seconds = total_seconds % seconds_in_a_minute;

    format!("{:.0}:{:02.0}", minutes, seconds)
}

fn read_music_from_directory(path: &Path) -> Vec<Song> {
    let mut songs = Vec::new();
    let mut file_paths: Vec<PathBuf> = Vec::new();

    let patterns = SUPPORTED_AUDIO_FILE_TYPES
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
