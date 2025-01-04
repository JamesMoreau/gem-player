use std::time::Duration;

use eframe::egui::{
    include_image, pos2, text, vec2, Align, Align2, Button, CentralPanel, Color32, ComboBox, Context, FontId, Frame, Id, Image, Label,
    Layout, Margin, PointerButton, Rect, Rgba, RichText, ScrollArea, Sense, Separator, Slider, TextEdit, TextFormat, TextStyle,
    TextureFilter, TextureOptions, Ui, UiBuilder, Vec2, ViewportCommand, Visuals, WidgetText,
};

use egui_extras::TableBuilder;
use strum::IntoEnumIterator;
use strum_macros::EnumIter;

use crate::{
    format_duration_to_mmss,
    player::{self, add_song_to_queue, is_playing, load_and_play_song, play_next_song_in_queue, play_or_pause, GemPlayer},
    sort_songs, Song, SortBy, SortOrder,
};

#[derive(Debug, Clone, PartialEq, Eq, EnumIter)]
pub enum View {
    Library,
    Queue,
    Playlists,
    Settings,
}

impl eframe::App for player::GemPlayer {
    fn clear_color(&self, _visuals: &Visuals) -> [f32; 4] {
        Rgba::TRANSPARENT.to_array() // Make sure we don't paint anything behind the rounded corners
    }

    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        // Necessary to keep UI up-to-date with the current state of the sink/player.
        ctx.request_repaint_after_secs(1.0);

        // Check if the current song has ended and play the next song in the queue.
        if self.sink.empty() {
            play_next_song_in_queue(self);
        }

        custom_window_frame(ctx, "", |ui| {
            let app_rect = ui.max_rect();

            let control_ui_height = 60.0;
            let control_ui_rect = Rect::from_min_max(app_rect.min, pos2(app_rect.max.x, app_rect.min.y + control_ui_height));

            let navigation_ui_height = 40.0;
            let navigation_ui_rect = Rect::from_min_max(pos2(app_rect.min.x, app_rect.max.y - navigation_ui_height), app_rect.max);

            let content_ui_rect = Rect::from_min_max(
                pos2(app_rect.min.x, control_ui_rect.max.y),
                pos2(app_rect.max.x, navigation_ui_rect.min.y),
            );

            let mut control_ui = ui.new_child(UiBuilder::new().max_rect(control_ui_rect));
            render_control_ui(&mut control_ui, self);

            let mut content_ui = ui.new_child(UiBuilder::new().max_rect(content_ui_rect));
            match self.current_view {
                View::Library => render_songs_ui(&mut content_ui, self),
                View::Queue => render_queue_ui(&mut content_ui, self),
                View::Playlists => {
                    content_ui.label("Playlists section coming soon.");
                }
                View::Settings => render_settings_ui(&mut content_ui, self),
            }

            let mut navigation_ui = ui.new_child(UiBuilder::new().max_rect(navigation_ui_rect));
            render_navigation_ui(&mut navigation_ui, self);
        });
    }
}

pub fn custom_window_frame(ctx: &Context, title: &str, add_contents: impl FnOnce(&mut Ui)) {
    let panel_frame = Frame {
        fill: ctx.style().visuals.window_fill(),
        rounding: 10.0.into(),
        stroke: ctx.style().visuals.widgets.noninteractive.fg_stroke,
        outer_margin: 0.5.into(), // so the stroke is within the bounds
        ..Default::default()
    };

    CentralPanel::default().frame(panel_frame).show(ctx, |ui| {
        let app_rect = ui.max_rect();

        let title_bar_height = 24.0;
        let title_bar_rect = {
            let mut rect = app_rect;
            rect.max.y = rect.min.y + title_bar_height;
            rect
        };
        title_bar_ui(ui, title_bar_rect, title);

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

pub fn title_bar_ui(ui: &mut Ui, title_bar_rect: eframe::epaint::Rect, title: &str) {
    let painter = ui.painter();

    let title_bar_response = ui.interact(title_bar_rect, Id::new("title_bar"), Sense::click_and_drag());

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
        ui.ctx().send_viewport_cmd(ViewportCommand::Maximized(!is_maximized));
    }

    if title_bar_response.drag_started_by(PointerButton::Primary) {
        ui.ctx().send_viewport_cmd(ViewportCommand::StartDrag);
    }

    ui.allocate_new_ui(
        UiBuilder::new().max_rect(title_bar_rect).layout(if cfg!(target_os = "macos") {
            Layout::left_to_right(Align::Center)
        } else {
            Layout::right_to_left(Align::Center)
        }),
        |ui| {
            ui.add_space(8.0);

            ui.visuals_mut().button_frame = false;
            let button_height = 12.0;

            let close_button = |ui: &mut Ui| {
                let close_response = ui
                    .add(Button::new(RichText::new("❌").size(button_height)))
                    .on_hover_text("Close the window");
                if close_response.clicked() {
                    ui.ctx().send_viewport_cmd(ViewportCommand::Close);
                }
            };

            let maximize_button = |ui: &mut Ui| {
                let is_maximized = ui.input(|i| i.viewport().maximized.unwrap_or(false));
                let tooltip = if is_maximized { "Restore window" } else { "Maximize window" };
                let maximize_response = ui.add(Button::new(RichText::new("🗗").size(button_height))).on_hover_text(tooltip);
                if maximize_response.clicked() {
                    ui.ctx().send_viewport_cmd(ViewportCommand::Maximized(!is_maximized));
                }
            };

            let minimize_button = |ui: &mut Ui| {
                let minimize_response = ui
                    .add(Button::new(RichText::new("🗕").size(button_height)))
                    .on_hover_text("Minimize the window");
                if minimize_response.clicked() {
                    ui.ctx().send_viewport_cmd(ViewportCommand::Minimized(true));
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

pub fn render_control_ui(ui: &mut Ui, gem_player: &mut GemPlayer) {
    Frame::none().inner_margin(Margin::symmetric(16.0, 0.0)).show(ui, |ui| {
        egui_flex::Flex::horizontal().show(ui, |flex| {
            flex.add_simple(egui_flex::item().align_self_content(Align2::LEFT_CENTER), |ui| {
                let clicked = ui.button(egui_material_icons::icons::ICON_SKIP_PREVIOUS).clicked();
                if clicked {
                    println!("Previous song");
                }

                let play_pause_icon = if is_playing(gem_player) {
                    egui_material_icons::icons::ICON_PAUSE
                } else {
                    egui_material_icons::icons::ICON_PLAY_ARROW
                };
                let clicked = ui.button(play_pause_icon).clicked();
                if clicked {
                    play_or_pause(gem_player);
                }

                let clicked = ui.button(egui_material_icons::icons::ICON_SKIP_NEXT).clicked();
                if clicked {
                    println!("Next song");
                }
            });

            flex.add_simple(egui_flex::item().grow(1.0), |ui| {
                egui_flex::Flex::vertical().show(ui, |flex| {
                    flex.add_simple(egui_flex::item().grow(1.0), |ui| {
                        let get_button_color = |is_active: bool| {
                            if is_active { ui.visuals().selection.bg_fill } else { Color32::GRAY }
                        };

                        let repeat_button = Button::new(
                            RichText::new(egui_material_icons::icons::ICON_REPEAT)
                                .color(get_button_color(gem_player.repeat)),
                        );
                        let shuffle_button = Button::new(
                            RichText::new(egui_material_icons::icons::ICON_SHUFFLE)
                                .color(get_button_color(gem_player.shuffle)),
                        );

                        let clicked = ui.add(repeat_button).clicked();
                        if clicked {
                            gem_player.repeat = !gem_player.repeat;
                            println!("Repeat: {}", if gem_player.repeat { "On" } else { "Off" });
                        }

                        let clicked = ui.add(shuffle_button).clicked();
                        if clicked {
                            gem_player.shuffle = !gem_player.shuffle;
                            println!("Shuffle: {}", if gem_player.shuffle { "On" } else { "Off" });
                        }
                    });
                });

                ui.add_space(8.0);

                let artwork_texture_options = TextureOptions::LINEAR.with_mipmap_mode(Some(TextureFilter::Linear));
                let artwork_size = Vec2::splat(52.0);
                let rounding = 4.0;
                let default_artwork = Image::new(include_image!("../assets/music_note_24dp_E8EAED_FILL0_wght400_GRAD0_opsz24.svg"))
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
                            gem_player.current_song.as_ref().unwrap().title.as_deref().unwrap_or("default")
                        );

                        Image::from_bytes(artwork_uri, artwork_bytes.clone())
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
                        let playback_progress_slider = Slider::new(&mut position_as_secs, 0.0..=song_duration_as_secs)
                            .trailing_fill(true)
                            .show_value(false)
                            .step_by(1.0); // Step by 1 second.
                        let response = ui.add(playback_progress_slider);

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
                            flex.add_simple(egui_flex::item().grow(1.0).align_self_content(Align2::LEFT_CENTER), |ui| {
                                let default_text_style = TextStyle::Body.resolve(ui.style());
                                let default_color = ui.visuals().text_color();
                                let data_format = TextFormat::simple(default_text_style.clone(), Color32::WHITE);

                                let mut job = text::LayoutJob::default();
                                job.append(&title, 0.0, data_format.clone());
                                job.append(" by ", 0.0, TextFormat::simple(default_text_style.clone(), default_color));
                                job.append(&artist, 0.0, data_format.clone());
                                job.append(" on ", 0.0, TextFormat::simple(default_text_style.clone(), default_color));
                                job.append(&album, 0.0, data_format.clone());

                                let song_label = unselectable_label(job).truncate();
                                ui.add(song_label);
                            });

                            flex.add_simple(egui_flex::item().align_self_content(Align2::RIGHT_CENTER), |ui| {
                                let position = Duration::from_secs_f32(position_as_secs);
                                let song_duration = Duration::from_secs_f32(song_duration_as_secs);
                                let time_label_text =
                                    format!("{} / {}", format_duration_to_mmss(position), format_duration_to_mmss(song_duration));

                                let time_label = unselectable_label(time_label_text);
                                ui.add(time_label);
                            });
                        });
                    });
                });
            });

            flex.add_simple(egui_flex::item().align_self_content(Align2::RIGHT_CENTER), |ui| {
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

                let volume_slider = Slider::new(&mut volume, 0.0..=1.0).trailing_fill(true).show_value(false);
                let changed = ui.add(volume_slider).changed();
                if changed {
                    gem_player.muted = false;
                    gem_player.volume_before_mute = if volume == 0.0 { None } else { Some(volume) }
                }

                gem_player.sink.set_volume(volume);
            });
        });
    });
}

pub fn render_songs_ui(ui: &mut Ui, gem_player: &mut GemPlayer) {
    let filtered_songs: Vec<Song> = gem_player
        .songs
        .iter()
        .filter(|song| {
            let search_lower = gem_player.search_text.to_lowercase();
            let search_fields = [&song.title, &song.artist, &song.album];
            search_fields
                .iter()
                .any(|field| field.as_ref().map_or(false, |text| text.to_lowercase().contains(&search_lower)))
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
        .sense(Sense::click())
        .cell_layout(Layout::left_to_right(Align::Center))
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
                    ui.add(unselectable_label(RichText::new(*h).strong()));
                });
            }
        })
        .body(|body| {
            body.rows(26.0, filtered_songs.len(), |mut row| {
                let song = &filtered_songs[row.index()];

                row.set_selected(gem_player.selected_song == Some(row.index()));

                row.col(|ui| {
                    ui.add_space(16.0);
                    ui.add(unselectable_label(song.title.as_ref().unwrap_or(&"Unknown Title".to_string())));
                });

                row.col(|ui| {
                    ui.add(unselectable_label(song.artist.as_ref().unwrap_or(&"Unknown Artist".to_string())));
                });

                row.col(|ui| {
                    ui.add(unselectable_label(song.album.as_ref().unwrap_or(&"Unknown".to_string())));
                });

                row.col(|ui| {
                    let duration_string = format_duration_to_mmss(song.duration);
                    ui.add(unselectable_label(duration_string));
                });

                let response = row.response();
                if response.clicked() {
                    gem_player.selected_song = Some(row.index());
                }

                if response.double_clicked() {
                    load_and_play_song(gem_player, song);
                }

                response.context_menu(|ui| {
                    if ui.button("Play").clicked() {
                        ui.close_menu();
                    }

                    if ui.button("Add to queue").clicked() {
                        add_song_to_queue(gem_player, song.clone());
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

pub fn render_queue_ui(ui: &mut Ui, gem_player: &mut GemPlayer) {
    if gem_player.queue.is_empty() {
        Frame::none()
            .outer_margin(Margin::symmetric(ui.available_width() * (1.0 / 4.0), 32.0))
            .show(ui, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add(unselectable_label("Queue is empty"));
                });
            });

        return;
    }

    let header_labels = ["Title", "Artist", "Album", "Time"];

    let available_width = ui.available_width();
    let time_width = 80.0;
    let remaining_width = available_width - time_width;
    let title_width = remaining_width * (2.0 / 4.0);
    let artist_width = remaining_width * (1.0 / 4.0);
    let album_width = remaining_width * (1.0 / 4.0);

    TableBuilder::new(ui)
        .striped(true)
        .resizable(true)
        .sense(Sense::click())
        .cell_layout(Layout::left_to_right(Align::Center))
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
                    ui.add(unselectable_label(RichText::new(*h).strong()));
                });
            }
        })
        .body(|body| {
            body.rows(26.0, gem_player.queue.len(), |mut row| {
                let song = gem_player.queue[row.index()].clone();

                row.set_selected(gem_player.selected_song == Some(row.index()));

                row.col(|ui| {
                    ui.add_space(16.0);
                    ui.add(unselectable_label(song.title.as_ref().unwrap_or(&"Unknown Title".to_string())));
                });

                row.col(|ui| {
                    ui.add(unselectable_label(song.artist.as_ref().unwrap_or(&"Unknown Artist".to_string())));
                });

                row.col(|ui| {
                    ui.add(unselectable_label(song.album.as_ref().unwrap_or(&"Unknown".to_string())));
                });

                row.col(|ui| {
                    let duration_string = format_duration_to_mmss(song.duration);
                    ui.add(unselectable_label(duration_string));
                });

                let response = row.response();
                if response.clicked() {
                    gem_player.selected_song = Some(row.index());
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

pub fn render_settings_ui(ui: &mut Ui, gem_player: &mut GemPlayer) {
    let available_width = ui.available_width();
    Frame::none()
        .outer_margin(Margin::symmetric(available_width * (1.0 / 4.0), 32.0))
        .show(ui, |ui| {
            ScrollArea::vertical().show(ui, |ui| {
                ui.add(unselectable_label(RichText::new("Music Library Path").heading()));
                
                ui.horizontal(|ui| {
                    let path = gem_player
                        .music_directory
                        .as_ref()
                        .map_or("No directory selected".to_string(), |p| p.to_string_lossy().to_string());
                    ui.label(path);

                    let clicked = ui.button("Browse").clicked();
                    if clicked {
                        // Add folder picker logic here
                        println!("Browse button clicked");
                    }
                });

                ui.add(Separator::default().spacing(32.0));

                ui.add(unselectable_label(RichText::new("Theme").heading()));
                ComboBox::from_label("Select Theme")
                    .selected_text(&gem_player.theme)
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut gem_player.theme, "Light".to_string(), "Light");
                        ui.selectable_value(&mut gem_player.theme, "Dark".to_string(), "Dark");
                        ui.selectable_value(&mut gem_player.theme, "System".to_string(), "System");
                    });

                ui.add(Separator::default().spacing(32.0));

                ui.add(unselectable_label(RichText::new("About Gem Player").heading()));
                let version = env!("CARGO_PKG_VERSION");
                ui.add(unselectable_label(format!("Version: {version}")));
                ui.add(unselectable_label("Gem Player is a lightweight music player."));

                ui.add(Separator::default().spacing(32.0));

                ui.add(unselectable_label(RichText::new("Author").heading()));
                ui.label("James Moreau");
                ui.hyperlink("https://jamesmoreau.github.io");
            });
        });
}

fn render_navigation_ui(ui: &mut Ui, gem_player: &mut GemPlayer) {
    Frame::none().inner_margin(Margin::symmetric(16.0, 16.0)).show(ui, |ui| {
        egui_flex::Flex::horizontal().show(ui, |flex| {
            flex.add_simple(egui_flex::item().grow(1.0).align_self_content(Align2::LEFT_CENTER), |ui| {
                for view in View::iter() {
                    let response = ui.selectable_label(gem_player.current_view == view, format!("{:?}", view));
                    if response.clicked() {
                        switch_view(gem_player, view);
                    }
                }
            });

            flex.add_simple(egui_flex::item().align_self_content(Align2::RIGHT_CENTER), |ui| {
                let filter_icon = egui_material_icons::icons::ICON_FILTER_LIST;
                ui.menu_button(filter_icon, |ui| {
                    let mut should_sort_songs = false;

                    for sort_by in SortBy::iter() {
                        let response = ui.radio_value(&mut gem_player.sort_by, sort_by, format!("{:?}", sort_by));
                        should_sort_songs |= response.clicked();
                    }

                    ui.separator();

                    for sort_order in SortOrder::iter() {
                        let response = ui.radio_value(&mut gem_player.sort_order, sort_order, format!("{:?}", sort_order));
                        should_sort_songs |= response.clicked();
                    }

                    if should_sort_songs {
                        sort_songs(&mut gem_player.songs, gem_player.sort_by, gem_player.sort_order);
                    }
                });

                let search_bar = TextEdit::singleline(&mut gem_player.search_text)
                    .hint_text("Search...")
                    .desired_width(140.0);
                ui.add(search_bar);

                let clear_button_is_visible = !gem_player.search_text.is_empty();
                let response = ui.add_visible(clear_button_is_visible, Button::new(egui_material_icons::icons::ICON_CLEAR));
                if response.clicked() {
                    gem_player.search_text.clear();
                }
            });
        });
    });
}

fn unselectable_label(text: impl Into<WidgetText>) -> Label {
    Label::new(text).selectable(false)
}
