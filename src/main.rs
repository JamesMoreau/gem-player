use crate::song::Song;
use eframe::egui;
use egui_extras::TableBuilder;
use glob::glob;
use std::path::{Path, PathBuf};

mod song;

fn main() -> eframe::Result {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([900.0, 600.0]),
        ..Default::default()
    };
    eframe::run_native(
        "My egui App",
        options,
        Box::new(|cc| Ok(Box::new(MyApp::new(cc)))),
    )
}

struct MyApp {
    name: String,
    age: u32,
    songs: Vec<Song>,
}

pub const SUPPORTED_AUDIO_FILE_TYPES: [&str; 6] = ["mp3", "m4a", "wav", "flac", "ogg", "opus"];

impl MyApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // This gives us image support:
        egui_extras::install_image_loaders(&cc.egui_ctx);

        let music_directory_result = dirs::audio_dir();
        let music_directory = match music_directory_result {
            Some(dir) => dir,
            None => {
                println!("No music directory found.");
                return Self {
                    name: "Arthur".to_owned(),
                    age: 42,
                    songs: Vec::new(),
                };
            }
        };

        let my_music_directory = music_directory.join("MyMusic");
        let songs = read_music_from_directory(&my_music_directory);
        for song in &songs {
            println!("Found song: {:?} at path: {:?}", song.title, song.file_path);
        }
        println!("Found {} songs", &songs.len());

        Self {
            name: "Arthur".to_owned(),
            age: 42,
            songs,
        }
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            // ui.heading("My egui Music App");

            // ui.horizontal(|ui| {
            //     let name_label = ui.label("Your name: ");
            //     ui.text_edit_singleline(&mut self.name)
            //         .labelled_by(name_label.id);
            // });
            // ui.add(egui::Slider::new(&mut self.age, 0..=120).text("age"));
            // if ui.button("Increment").clicked() {
            //     self.age += 1;
            // }
            // ui.label(format!("Hello '{}', age {}", self.name, self.age));

            // ui.image(egui::include_image!("../assets/ferris.png"));

            TableBuilder::new(ui)
                .striped(true)
                .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                .columns(egui_extras::Column::remainder(), 5)
                .header(18.0, |mut header| {
                    header.col(|ui| {
                        ui.strong("Artwork");
                    });
                    header.col(|ui| {
                        ui.strong("Title");
                    });
                    header.col(|ui| {
                        ui.strong("Artist");
                    });
                    header.col(|ui| {
                        ui.strong("Album");
                    });
                    header.col(|ui| {
                        ui.strong("Time");
                    });
                })
                .body(|mut body| {
                    for song in &self.songs {
                        body.row(32.0, |mut row| {
                            row.col(|ui| {
                                ui.add(egui::Image::new("https://picsum.photos/seed/1.759706314/1024"));
                            });

                            row.col(|ui| {
                                ui.label(song.title.as_ref().unwrap_or(&"Unknown".to_string()));
                            });

                            row.col(|ui| {
                                ui.label(song.artist.as_ref().unwrap_or(&"Unknown".to_string()));
                            });

                            row.col(|ui| {
                                ui.label(song.album.as_ref().unwrap_or(&"Unknown".to_string()));
                            });

                            row.col(|ui| {
                                let duration_string = format_duration(song.duration);
                                ui.label(duration_string);
                            });
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
