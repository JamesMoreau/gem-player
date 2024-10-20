use crate::song::Song;
use eframe::egui::{self};
use egui_extras::TableBuilder;
use glob::glob;
use std::path::{Path, PathBuf};

mod song;

/*
TODO:
figure out why shrinking the window horizontally causes buttons to shrink and then crash.

- In the controls ui we want the following:
  - Play button
  - Pause button
  - Volume slider
  - Playback progress slider
  - Current song info
  - song artwork
  - Search bar
  - Music Visualizer
  - Sorting

Perhaps we could have the top panel contain the searching and sorting controls, and the bottom panel contain the playback controls and the music visualizer.
Or, we could have the control ui (current song, playback progress, artwork, visualizer) on the top panel be stacked vertically.

- file watcher / update on change
*/

fn main() -> eframe::Result {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([900.0, 600.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Gem Player",
        options,
        Box::new(|cc| Ok(Box::new(MyApp::new(cc)))),
    )
}

struct MyApp {
    age: u32,
    songs: Vec<Song>,
    
    search_text: String,
    sort_by: SortBy,
    sort_order: SortOrder,
    
    music_directory: Option<PathBuf>,
    
    selected_song: Option<usize>,
    volume: f32,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum SortBy {
    Title,
    Artist,
    Album,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum SortOrder {
    Ascending,
    Descending,
}

pub const SUPPORTED_AUDIO_FILE_TYPES: [&str; 6] = ["mp3", "m4a", "wav", "flac", "ogg", "opus"];

impl MyApp {
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

        let mut default_self = Self {
            age: 42,
            songs: Vec::new(),
            selected_song: None,
            search_text: String::new(),
            volume: 0.0,
            music_directory: None,
            sort_by: SortBy::Title,
            sort_order: SortOrder::Ascending,
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

        Self {
            songs,
            ..default_self
        }
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("top_panel")
            .resizable(false)
            .min_height(48.0)
            .show(ctx, |ui| {
                egui::Frame::none().inner_margin(8.0).show(ui, |ui| {
                    ui.horizontal_centered(|ui| {
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
                            ui.radio_value(&mut self.sort_by, SortBy::Title, "Title");
                            ui.radio_value(&mut self.sort_by, SortBy::Artist, "Artist");
                            ui.radio_value(&mut self.sort_by, SortBy::Album, "Album");
                            ui.separator();
                            ui.radio_value(&mut self.sort_order, SortOrder::Ascending, "Ascending"); 
                            ui.radio_value(&mut self.sort_order, SortOrder::Descending, "Descending");                           
                        });

                        let search_bar = egui::TextEdit::singleline(&mut self.search_text)
                            .hint_text("Search...")
                            .desired_width(200.0);
                        ui.add(search_bar);
                    });
                });
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            let header_labels = ["Title", "Artist", "Album", "Time"];

            TableBuilder::new(ui)
                .striped(true)
                .sense(egui::Sense::click())
                .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                .columns(egui_extras::Column::remainder(), header_labels.len())
                .header(16.0, |mut header| {
                    for h in &header_labels {
                        header.col(|ui| {
                            ui.strong(h.to_string());
                        });
                    }
                })
                .body(|mut body| {
                    for (i, song) in self.songs.iter().enumerate() {
                        body.row(28.0, |mut row| {
                            row.set_selected(self.selected_song == Some(i));

                            row.col(|ui| {
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
                                let duration_string = format_duration(song.duration);
                                ui.add(egui::Label::new(duration_string).selectable(false));
                            });

                            if row.response().clicked() {
                                self.selected_song = Some(i);
                            }
                        });
                    }
                });
        });
    }
}

fn format_duration(duration: std::time::Duration) -> String {
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
