use std::time::Duration;

use eframe::egui::{self, TextureFilter, TextureOptions, ViewportCommand};
use egui_extras::TableBuilder;
use strum::IntoEnumIterator;
use strum_macros::EnumIter;

use crate::{song::Song, sort_songs, utils::{self, format_duration_to_mmss}, GemPlayer, SortBy, SortOrder};

#[derive(Debug, Clone, PartialEq, Eq, EnumIter)]
pub enum View {
    Library,
    Queue,
    Playlists,
    Settings,
}

pub fn custom_window_frame(ctx: &egui::Context, title: &str, add_contents: impl FnOnce(&mut egui::Ui)) {
    let panel_frame = egui::Frame {
        fill: ctx.style().visuals.window_fill(),
        rounding: 10.0.into(),
        stroke: ctx.style().visuals.widgets.noninteractive.fg_stroke,
        outer_margin: 0.5.into(), // so the stroke is within the bounds
        ..Default::default()
    };

    egui::CentralPanel::default().frame(panel_frame).show(ctx, |ui| {
        let app_rect = ui.max_rect();

        let title_bar_height = 24.0;
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
        let mut content_ui = ui.new_child(egui::UiBuilder::new().max_rect(content_rect));
        add_contents(&mut content_ui);
    });
}

pub fn title_bar_ui(ui: &mut egui::Ui, title_bar_rect: eframe::epaint::Rect, title: &str) {
    let painter = ui.painter();

    let title_bar_response = ui.interact(
        title_bar_rect,
        egui::Id::new("title_bar"),
        egui::Sense::click_and_drag(),
    );

    painter.text(
        title_bar_rect.center(),
        egui::Align2::CENTER_CENTER,
        title,
        egui::FontId::proportional(20.0),
        ui.style().visuals.text_color(),
    );

    // Paint the line under the title:
    painter.line_segment(
        [
            title_bar_rect.left_bottom() + egui::vec2(1.0, 0.0),
            title_bar_rect.right_bottom() + egui::vec2(-1.0, 0.0),
        ],
        ui.visuals().widgets.noninteractive.bg_stroke,
    );

    if title_bar_response.double_clicked() {
        let is_maximized = ui.input(|i| i.viewport().maximized.unwrap_or(false));
        ui.ctx().send_viewport_cmd(ViewportCommand::Maximized(!is_maximized));
    }

    if title_bar_response.drag_started_by(egui::PointerButton::Primary) {
        ui.ctx().send_viewport_cmd(ViewportCommand::StartDrag);
    }

    ui.allocate_new_ui(
        egui::UiBuilder::new()
            .max_rect(title_bar_rect)
            .layout(
                if cfg!(target_os = "macos") {
                    egui::Layout::left_to_right(egui::Align::Center)
                } else {
                    egui::Layout::right_to_left(egui::Align::Center)
                }
            ),
        |ui| {
            ui.add_space(8.0);

            ui.visuals_mut().button_frame = false;
            let button_height = 12.0;

            let close_button = |ui: &mut egui::Ui| {
                let close_response = ui
                    .add(egui::Button::new(egui::RichText::new("❌").size(button_height)))
                    .on_hover_text("Close the window");
                if close_response.clicked() {
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                }
            };

            let maximize_button = |ui: &mut egui::Ui| {
                let is_maximized = ui.input(|i| i.viewport().maximized.unwrap_or(false));
                let tooltip = if is_maximized { "Restore window" } else { "Maximize window" };
                let maximize_response = ui
                    .add(egui::Button::new(egui::RichText::new("🗗").size(button_height)))
                    .on_hover_text(tooltip);
                if maximize_response.clicked() {
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::Maximized(!is_maximized));
                }
            };

            let minimize_button = |ui: &mut egui::Ui| {
                let minimize_response = ui
                    .add(egui::Button::new(egui::RichText::new("🗕").size(button_height)))
                    .on_hover_text("Minimize the window");
                if minimize_response.clicked() {
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::Minimized(true));
                }
            };
    
            if cfg!(target_os = "macos") {
                close_button(ui);
                minimize_button(ui);
                maximize_button(ui);
            } else {
                minimize_button(ui);
                maximize_button(ui);
                close_button(ui);
            }
        },
    );
}

pub fn switch_view(gem_player: &mut GemPlayer, view: View) {
    println!("Switching to view: {:?}", view);
    gem_player.current_view = view;
}

pub fn render_control_ui(ui: &mut egui::Ui, gem_player: &mut GemPlayer) {
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

                ui.add_space(8.0);
    
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

            flex.add_simple(egui_flex::item().grow(1.0), |ui| {
                egui_flex::Flex::vertical().show(ui, |flex| {
                    flex.add_simple(egui_flex::item().grow(1.0), |ui| {
                        let repeat_button = egui::Button::new(egui_material_icons::icons::ICON_REPEAT);
                        let shuffle_button = egui::Button::new(egui_material_icons::icons::ICON_SHUFFLE);

                        let clicked = ui.add(repeat_button).clicked();
                        if clicked {
                            println!("Repeat");
                        }
        
                        let clicked = ui.add(shuffle_button).clicked();
                        if clicked {
                            println!("Shuffle");
                        }
                    });
                });

                ui.add_space(8.0);

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
                    flex.add_simple(egui_flex::item().grow(1.0), |ui| {
                        let mut title = "None".to_string();
                        let mut artist = "None".to_string();
                        let mut album = "None".to_string();
                        let mut position_as_secs = 0.0;
                        let mut song_duration_as_secs = 0.1; // We set to 0.1 so that when no song is playing, the slider is at the start.
                        
                        let song_is_some = gem_player.current_song.is_some();
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

                        if response.dragged() && gem_player.paused_before_scrubbing.is_none() && song_is_some {
                            gem_player.paused_before_scrubbing = Some(gem_player.sink.is_paused());
                            gem_player.sink.pause(); // Pause playback during scrubbing
                        }
                        
                        if response.drag_stopped() && song_is_some {
                            let new_position = Duration::from_secs_f32(position_as_secs);
                            println!("Seeking to {} of {}", format_duration_to_mmss(new_position), title);
                            if let Err(e) = gem_player.sink.try_seek(new_position) {
                                println!("Error seeking to new position: {:?}", e);
                            }
                        
                            // Resume playback if the player was not paused before scrubbing
                            if gem_player.paused_before_scrubbing == Some(false) {
                                gem_player.sink.play();
                            }
                        
                            gem_player.paused_before_scrubbing = None;
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
                                let time_label_text = format!("{} / {}", utils::format_duration_to_mmss(position), format_duration_to_mmss(song_duration));
                                
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

pub fn render_songs_ui(ui: &mut egui::Ui, gem_player: &mut GemPlayer) {
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

pub fn render_queue_ui(ui: &mut egui::Ui, gem_player: &mut GemPlayer) {
    let queue_songs: Vec<Song> = gem_player.queue.clone();

    let header_labels = ["Title", "Artist", "Album", "Time", "Actions"];

    let available_width = ui.available_width();
    let time_width = 80.0;
    let actions_width = 60.0;
    let remaining_width = available_width - time_width - actions_width;
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
        .column(egui_extras::Column::exact(actions_width))
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
            body.rows(26.0, queue_songs.len(), |mut row| {
                let song = &queue_songs[row.index()];

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

                let row_index = row.index();
                row.col(|ui| {
                    if ui.button("Remove").clicked() {
                        gem_player.queue.remove(row_index);
                    }
                });

                let response = row.response();
                if response.clicked() {
                    gem_player.selected_song = Some(row.index());
                }

                if response.double_clicked() {
                    gem_player.load_and_play_song(song);
                }

                response.context_menu(|ui| {
                    if ui.button("Play Next").clicked() {
                        ui.close_menu();
                    }

                    if ui.button("Remove from Queue").clicked() {
                        gem_player.queue.remove(row.index());
                        ui.close_menu();
                    }
                });
            });
        });
}

pub fn render_settings_ui(ui: &mut egui::Ui, gem_player: &mut GemPlayer) {
    let available_width = ui.available_width();
    egui::Frame::none()
        .outer_margin(egui::Margin::symmetric(available_width * (1.0 / 4.0), 32.0))
        .show(ui, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.add(egui::Label::new("Music Library Path:").selectable(false));
                ui.horizontal(|ui| {
                    let path = gem_player.music_directory.as_ref().map_or("No directory selected".to_string(), |p| p.to_string_lossy().to_string());
                    ui.label(path);
                    
                    let clicked = ui.button("Browse").clicked();
                    if clicked {
                        // Add folder picker logic here
                        println!("Browse button clicked");
                    }
                });
        
                ui.add(egui::Separator::default().spacing(32.0));
        
                ui.label("Theme:");
                egui::ComboBox::from_label("Select Theme")
                    .selected_text(&gem_player.theme)
                    .show_ui(ui, |ui| {
                        ui.selectable_value(
                            &mut gem_player.theme,
                            "Light".to_string(),
                            "Light",
                        );
                        ui.selectable_value(
                            &mut gem_player.theme,
                            "Dark".to_string(),
                            "Dark",
                        );
                        ui.selectable_value(
                            &mut gem_player.theme,
                            "System".to_string(),
                            "System",
                        );
                    });
        
                ui.add(egui::Separator::default().spacing(32.0));
        
                ui.heading("About Gem Player");
                let version = env!("CARGO_PKG_VERSION");
                ui.add(egui::Label::new(format!("Version: {version}")).selectable(false));
                ui.add(egui::Label::new("Gem Player is a modern, lightweight music player.").selectable(false));
                ui.add(egui::Label::new("For support or inquiries, visit our website.").selectable(false));
            });
        });
}