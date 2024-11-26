use crate::song::Song;
use eframe::egui::{self, TextureFilter, TextureOptions, Vec2, ViewportCommand};
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
- tab bar at the bottom for playlists, queue, settings, etc.
- should read_music_from_directory return a Result<Vec<Song>, Error> instead of Vec<Song>? Fix this once we allow custom music path. loading icon when songs are being loaded.
- file watcher / update on change
- register play pause commands with apple menu.

- Play button / Pause button, Next song, previous song
- Repeat / Shuffle above the playback progress.
- Music Visualizer ^.
- Queue

Could remove object oriented programming and just have a struct with functions that take a mutable reference to self.
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

fn custom_window_frame(ctx: &egui::Context, title: &str, add_contents: impl FnOnce(&mut egui::Ui)) {
    use egui::{CentralPanel, UiBuilder};

    let panel_frame = egui::Frame {
        fill: ctx.style().visuals.window_fill(),
        rounding: 10.0.into(),
        stroke: ctx.style().visuals.widgets.noninteractive.fg_stroke,
        outer_margin: 0.5.into(), // so the stroke is within the bounds
        ..Default::default()
    };

    CentralPanel::default().frame(panel_frame).show(ctx, |ui| {
        let app_rect = ui.max_rect();

        let title_bar_height = 32.0;
        let title_bar_rect = {
            let mut rect = app_rect;
            rect.max.y = rect.min.y + title_bar_height;
            rect
        };
        title_bar_ui(ui, title_bar_rect, title);

        // Add the contents:
        let content_rect = {
            let mut rect = app_rect;
            rect.min.y = title_bar_rect.max.y;
            rect
        }
        .shrink(4.0);
        let mut content_ui = ui.new_child(UiBuilder::new().max_rect(content_rect));
        add_contents(&mut content_ui);
    });
}

fn title_bar_ui(ui: &mut egui::Ui, title_bar_rect: eframe::epaint::Rect, title: &str) {
    use egui::{vec2, Align2, FontId, Id, PointerButton, Sense, UiBuilder};

    let painter = ui.painter();

    let title_bar_response = ui.interact(
        title_bar_rect,
        Id::new("title_bar"),
        Sense::click_and_drag(),
    );

    painter.text(
        title_bar_rect.center(),
        Align2::CENTER_CENTER,
        title,
        FontId::proportional(20.0),
        ui.style().visuals.text_color(),
    );

    // Paint the line under the title:
    painter.line_segment(
        [
            title_bar_rect.left_bottom() + vec2(1.0, 0.0),
            title_bar_rect.right_bottom() + vec2(-1.0, 0.0),
        ],
        ui.visuals().widgets.noninteractive.bg_stroke,
    );

    if title_bar_response.double_clicked() {
        let is_maximized = ui.input(|i| i.viewport().maximized.unwrap_or(false));
        ui.ctx()
            .send_viewport_cmd(ViewportCommand::Maximized(!is_maximized));
    }

    if title_bar_response.drag_started_by(PointerButton::Primary) {
        ui.ctx().send_viewport_cmd(ViewportCommand::StartDrag);
    }

    ui.allocate_new_ui(
        UiBuilder::new()
            .max_rect(title_bar_rect)
            .layout(egui::Layout::right_to_left(egui::Align::Center)),
        |ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            ui.visuals_mut().button_frame = false;
            ui.add_space(8.0);
            close_maximize_minimize(ui);
        },
    );
}

fn close_maximize_minimize(ui: &mut egui::Ui) {
    use egui::{Button, RichText};

    let button_height = 12.0;
    let button_distance = 6.0;

    let close_response = ui
        .add(Button::new(RichText::new("❌").size(button_height)))
        .on_hover_text("Close the window");
    if close_response.clicked() {
        ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
    }

    ui.add_space(button_distance);

    let is_maximized = ui.input(|i| i.viewport().maximized.unwrap_or(false));
    if is_maximized {
        let maximized_response = ui
            .add(Button::new(RichText::new("🗗").size(button_height)))
            .on_hover_text("Restore window");
        if maximized_response.clicked() {
            ui.ctx()
                .send_viewport_cmd(ViewportCommand::Maximized(false));
        }
    } else {
        let maximized_response = ui
            .add(Button::new(RichText::new("🗗").size(button_height)))
            .on_hover_text("Maximize window");
        if maximized_response.clicked() {
            ui.ctx().send_viewport_cmd(ViewportCommand::Maximized(true));
        }
    }

    ui.add_space(button_distance);

    let minimized_response = ui
        .add(Button::new(RichText::new("🗕").size(button_height)))
        .on_hover_text("Minimize the window");
    if minimized_response.clicked() {
        ui.ctx().send_viewport_cmd(ViewportCommand::Minimized(true));
    }
}

impl eframe::App for GemPlayer {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        egui::Rgba::TRANSPARENT.to_array() // Make sure we don't paint anything behind the rounded corners
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Necessary to keep ui up to date with the current state of the sink / player.
        ctx.request_repaint_after_secs(1.0);

        // println!("{}", ctx.input(|i: &egui::InputState| i.screen_rect())); // Prints the dimension of the window.

        custom_window_frame(ctx, "", |ui| {
            render_control_ui(ui, self);

            ui.separator();

            render_songs_ui(ui, self);
        });
    }
}

fn render_control_ui(ui: &mut egui::Ui, gem_player: &mut GemPlayer) {
    egui::Frame::none().inner_margin(egui::Margin::symmetric(16.0, 0.0)).show(ui, |ui| {
        egui_flex::Flex::horizontal().show(ui, |flex| {
            flex.add_simple(egui_flex::item().align_self_content(egui::Align2::LEFT_CENTER), |ui| {
                let clicked = ui.button(egui_material_icons::icons::ICON_SKIP_PREVIOUS).clicked();
                if clicked {
                    println!("Previous song");
                }

                let play_pause_icon = if gem_player.is_playing() {
                    egui_material_icons::icons::ICON_PAUSE
                } else {
                    egui_material_icons::icons::ICON_PLAY_ARROW
                };
                let clicked = ui.button(play_pause_icon).clicked();
                if clicked {
                    gem_player.play_or_pause();
                }

                let clicked = ui.button(egui_material_icons::icons::ICON_SKIP_NEXT).clicked();
                if clicked {
                    println!("Next song");
                }
    
                let mut volume = gem_player.sink.volume();
    
                let volume_icon = match volume {
                    v if v == 0.0 => egui_material_icons::icons::ICON_VOLUME_OFF,
                    v if v <= 0.5 => egui_material_icons::icons::ICON_VOLUME_DOWN,
                    _ => egui_material_icons::icons::ICON_VOLUME_UP, // v > 0.5 && v <= 1.0
                };
                let clicked = ui.button(volume_icon).clicked();
                if clicked {
                    gem_player.muted = !gem_player.muted;
                    if gem_player.muted {
                        gem_player.volume_before_mute = Some(volume);
                        volume = 0.0;
                    } else if let Some(v) = gem_player.volume_before_mute {
                        volume = v;
                    }
                }
    
                let volume_slider = egui::Slider::new(&mut volume, 0.0..=1.0)
                    .trailing_fill(true)
                    .show_value(false);
                let changed = ui.add(volume_slider).changed();
                if changed {
                    gem_player.muted = false;
                    gem_player.volume_before_mute = if volume == 0.0 { None } else { Some(volume) }
                }
    
                gem_player.sink.set_volume(volume);
            });

            flex.add_simple(egui_flex::item().grow(1.0).align_self_content(egui::Align2::CENTER_CENTER), |ui| {
                let artwork_texture_options = TextureOptions::LINEAR.with_mipmap_mode(Some(TextureFilter::Linear));
                let artwork_size = egui::Vec2::splat(52.0);
                let rounding = 4.0;
                let default_artwork = egui::Image::new(egui::include_image!(
                    "../assets/music_note_24dp_E8EAED_FILL0_wght400_GRAD0_opsz24.svg"
                ))
                .texture_options(artwork_texture_options)
                .fit_to_exact_size(artwork_size)
                .rounding(rounding);

                let artwork = gem_player
                    .current_song
                    .as_ref()
                    .and_then(|song| song.artwork.as_ref())
                    .map(|artwork_bytes| {
                        let artwork_uri = format!(
                            "bytes://artwork-{}",
                            gem_player.current_song
                                .as_ref()
                                .unwrap()
                                .title
                                .as_deref()
                                .unwrap_or("default")
                        );

                        egui::Image::from_bytes(artwork_uri, artwork_bytes.clone())
                            .texture_options(artwork_texture_options)
                            .fit_to_exact_size(artwork_size)
                            .rounding(rounding)
                    })
                    .unwrap_or(default_artwork);

                ui.add(artwork);

                egui_flex::Flex::vertical().show(ui, |flex| {
                    flex.add_simple(egui_flex::item().grow(1.0).align_self_content(egui::Align2::LEFT_CENTER), |ui| {
                        let mut title = "None".to_string();
                        let mut artist = "None".to_string();
                        let mut album = "None".to_string();
                        let mut position_as_secs = 0.0;
                        let mut song_duration_as_secs = 0.1; // We set to 0.1 so that when no song is playing, the slider is at the start.

                        if let Some(song) = &gem_player.current_song {
                            title = song.title.clone().unwrap_or("Unknown Title".to_string());
                            artist = song.artist.clone().unwrap_or("Unknown Artist".to_string());
                            album = song.album.clone().unwrap_or("Unknown Album".to_string());
                            position_as_secs = gem_player.sink.get_pos().as_secs_f32();
                            song_duration_as_secs = song.duration.as_secs_f32();
                        }

                        ui.style_mut().spacing.slider_width = 500.0;
                        let playback_progress_slider =
                            egui::Slider::new(&mut position_as_secs, 0.0..=song_duration_as_secs)
                                .trailing_fill(true)
                                .show_value(false)
                                .step_by(1.0); // Step by 1 second.
                        let response: egui::Response = ui.add(playback_progress_slider);

                        if response.dragged() {
                            // We pause the audio during seeking to avoid scrubbing sound.
                            gem_player.sink.pause();
                        }
                        
                        if response.drag_stopped() {
                            let new_position = Duration::from_secs_f32(position_as_secs);
                            if let Err(e) = gem_player.sink.try_seek(new_position) {
                                println!("Error seeking to new position: {:?}", e);
                            }

                            gem_player.sink.play();
                        }

                        egui_flex::Flex::horizontal().wrap(false).show(ui, |flex| {
                            flex.add_simple(egui_flex::item().grow(1.0).align_self_content(egui::Align2::LEFT_CENTER), |ui| {
                                let default_text_style = egui::TextStyle::Body.resolve(ui.style());
                                let default_color = ui.visuals().text_color();
                                let data_format = egui::TextFormat::simple(default_text_style.clone(),  egui::Color32::WHITE);
                                
                                let mut job = egui::text::LayoutJob::default();
                                job.append(&title, 0.0, data_format.clone());
                                job.append(" by ", 0.0, egui::TextFormat::simple(default_text_style.clone(), default_color));
                                job.append(&artist, 0.0, data_format.clone());
                                job.append(" on ", 0.0, egui::TextFormat::simple(default_text_style.clone(), default_color));
                                job.append(&album, 0.0, data_format.clone());

                                let song_label = egui::Label::new(job).truncate().selectable(false);
                                ui.add(song_label);
                            });

                            flex.add_simple(egui_flex::item().align_self_content(egui::Align2::RIGHT_CENTER), |ui| {
                                let position = Duration::from_secs_f32(position_as_secs);
                                let song_duration = Duration::from_secs_f32(song_duration_as_secs);
                                let time_label_text = format!("{} / {}", format_duration_to_mmss(position), format_duration_to_mmss(song_duration));
                                
                                let time_label = egui::Label::new(time_label_text).selectable(false);
                                ui.add(time_label);
                            });
                        });
                    });
                });
            });

            flex.add_simple(egui_flex::item().align_self_content(egui::Align2::RIGHT_CENTER), |ui| {
                let filter_icon = egui_material_icons::icons::ICON_FILTER_LIST;
                ui.menu_button(filter_icon, |ui| {
                    let mut should_sort_songs = false;

                    for sort_by in SortBy::iter() {
                        let response = ui.radio_value(
                            &mut gem_player.sort_by,
                            sort_by,
                            format!("{:?}", sort_by),
                        );
                        should_sort_songs |= response.clicked();
                    }

                    ui.separator();

                    for sort_order in SortOrder::iter() {
                        let response = ui.radio_value(
                            &mut gem_player.sort_order,
                            sort_order,
                            format!("{:?}", sort_order),
                        );
                        should_sort_songs |= response.clicked();
                    }

                    if should_sort_songs {
                        sort_songs(&mut gem_player.songs, gem_player.sort_by, gem_player.sort_order);
                    }
                });

                let search_bar = egui::TextEdit::singleline(&mut gem_player.search_text)
                    .hint_text("Search...")
                    .desired_width(140.0);
                ui.add(search_bar);

                let clear_button_is_visible = !gem_player.search_text.is_empty();
                let response = ui.add_visible(clear_button_is_visible, egui::Button::new(egui_material_icons::icons::ICON_CLEAR));
                if response.clicked() {
                    gem_player.search_text.clear();
                }
            });
        });
    });
}

fn render_songs_ui(ui: &mut egui::Ui, gem_player: &mut GemPlayer) {
    let filtered_songs: Vec<Song> = gem_player
        .songs
        .iter()
        .filter(|song| {
            let search_lower = gem_player.search_text.to_lowercase();
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

                row.set_selected(gem_player.selected_song == Some(row.index()));

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
                    gem_player.selected_song = Some(row.index());
                }

                if response.double_clicked() {
                    gem_player.load_and_play_song(song);
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
}

fn format_duration_to_mmss(duration: std::time::Duration) -> String {
    let total_seconds = duration.as_secs();
    let seconds_per_minute = 60;
    let minutes = total_seconds / seconds_per_minute;
    let seconds = total_seconds % seconds_per_minute;

    format!("{}:{:02}", minutes, seconds)
}

// fn format_duration_to_hhmmss(duration: std::time::Duration) -> String {
//     let total_seconds: f64 = duration.as_secs_f64();
//     let hours =
//         total_seconds / (constants::MINUTES_PER_HOUR as f64 * constants::SECONDS_PER_MINUTE as f64);
//     let minutes =
//         (total_seconds / constants::SECONDS_PER_MINUTE as f64) % constants::MINUTES_PER_HOUR as f64;
//     let seconds = total_seconds % constants::SECONDS_PER_MINUTE as f64;
//     format!("{:.0}:{:02.0}:{:02.0}", hours, minutes, seconds)
// }

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
