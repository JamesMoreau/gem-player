use std::time::Duration;

use chrono::Utc;
use eframe::egui::{
    containers, include_image, text, vec2, Align, Align2, Button, CentralPanel, Color32, ComboBox, Context, FontId, Frame, Id, Image,
    Label, Layout, Margin, PointerButton, Rgba, RichText, ScrollArea, Sense, Separator, Slider, TextEdit, TextFormat, TextStyle,
    TextureFilter, TextureOptions, Ui, UiBuilder, Vec2, ViewportCommand, Visuals,
};

use egui_extras::{Size, StripBuilder, TableBuilder};
use egui_flex::{item, Flex, FlexJustify};
use egui_material_icons::icons;
use rfd::FileDialog;
use strum::IntoEnumIterator;
use strum_macros::EnumIter;
use uuid::Uuid;

use crate::{
    format_duration_to_hhmmss, format_duration_to_mmss, get_duration_of_songs,
    player::{
        self, add_next_to_queue, add_to_queue, handle_input, is_playing, move_song_to_front, play_library_from_song, play_next,
        play_or_pause, play_previous, read_music_from_a_directory, remove_from_queue, shuffle_queue, GemPlayer, PlaylistsUIState,
    },
    print_error, print_info, sort_songs, Playlist, Song, SortBy, SortOrder, Theme,
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
        // let _window_rect = ctx.input(|i: &eframe::egui::InputState| i.screen_rect()); // For debugging.
        // info!("Window rect: {:?}", window_rect);

        // Necessary to keep UI up-to-date with the current state of the sink/player.
        ctx.request_repaint_after_secs(1.0);

        match self.ui_state.theme {
            Theme::System => {} // We don't need to do anything here since egui will automatically switch when the system theme changes.
            Theme::Dark => ctx.set_visuals(Visuals::dark()),
            Theme::Light => ctx.set_visuals(Visuals::light()),
        }

        // Check if the current song has ended and play the next song in the queue.
        if self.player.sink.empty() {
            play_next(self);
        }

        handle_input(ctx, self);

        custom_window_frame(ctx, "", |ui| {
            let control_ui_height = 64.0;
            let navigation_ui_height = 32.0;

            StripBuilder::new(ui)
                .size(Size::exact(control_ui_height))
                .size(Size::remainder())
                .size(Size::exact(navigation_ui_height))
                .vertical(|mut strip| {
                    strip.cell(|ui| {
                        render_control_ui(ui, self);
                        ui.add(Separator::default().spacing(0.0).shrink(1.0));
                    });
                    strip.cell(|ui| match self.ui_state.current_view {
                        View::Library => render_library_ui(ui, self),
                        View::Queue => render_queue_ui(ui, &mut self.player.queue),
                        View::Playlists => render_playlists_ui(ui, &mut self.playlists, &mut self.ui_state.playlists_ui_state),
                        View::Settings => render_settings_ui(ui, self),
                    });
                    strip.cell(|ui| {
                        ui.add(Separator::default().spacing(0.0).shrink(1.0));
                        render_navigation_ui(ui, self);
                    });
                });
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
        };
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

    ui.scope_builder(
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
                    .add(Button::new(RichText::new(icons::ICON_CLOSE).size(button_height)))
                    .on_hover_text("Close the window");
                if close_response.clicked() {
                    ui.ctx().send_viewport_cmd(ViewportCommand::Close);
                }
            };

            let maximize_button = |ui: &mut Ui| {
                let is_maximized = ui.input(|i| i.viewport().maximized.unwrap_or(false));
                let tooltip = if is_maximized { "Restore window" } else { "Maximize window" };
                let maximize_response = ui
                    .add(Button::new(RichText::new(icons::ICON_SQUARE).size(button_height)))
                    .on_hover_text(tooltip);
                if maximize_response.clicked() {
                    ui.ctx().send_viewport_cmd(ViewportCommand::Maximized(!is_maximized));
                }
            };

            let minimize_button = |ui: &mut Ui| {
                let minimize_response = ui
                    .add(Button::new(RichText::new(icons::ICON_MINIMIZE).size(button_height)))
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
    print_info(format!("Switching to view: {:?}", view));
    gem_player.ui_state.current_view = view;
}

pub fn render_control_ui(ui: &mut Ui, gem_player: &mut GemPlayer) {
    ui.spacing_mut().item_spacing.y = 0.0;

    Frame::none().inner_margin(Margin::symmetric(16.0, 0.0)).show(ui, |ui| {
        Flex::horizontal().w_full().justify(FlexJustify::SpaceBetween).show(ui, |flex| {
            flex.add_ui(item(), |ui| {
                let previous_button = Button::new(RichText::new(icons::ICON_SKIP_PREVIOUS));
                let is_previous_enabled = gem_player.player.current_song.is_some() || !gem_player.player.history.is_empty();

                let response = ui
                    .add_enabled(is_previous_enabled, previous_button)
                    .on_hover_text("Previous")
                    .on_disabled_hover_text("No previous song");
                if response.clicked() {
                    // If we are near the beginning of the song, we go to the previously played song.
                    // Otherwise, we seek to the beginning.
                    let playback_position = gem_player.player.sink.get_pos().as_secs_f32();
                    let rewind_threshold = 5.0; // If playback is within first 5 seconds, go to previous song.

                    if playback_position < rewind_threshold && !gem_player.player.history.is_empty() {
                        play_previous(gem_player);
                    } else {
                        let result = gem_player.player.sink.try_seek(Duration::ZERO);
                        if let Err(e) = result {
                            print_error(format!("Error rewinding song: {:?}", e));
                        }
                    }
                }

                let play_pause_icon = if is_playing(&mut gem_player.player) {
                    icons::ICON_PAUSE
                } else {
                    icons::ICON_PLAY_ARROW
                };
                let tooltip = if is_playing(&mut gem_player.player) { "Pause" } else { "Play" };
                let play_pause_button = Button::new(RichText::new(play_pause_icon));
                let song_is_playing = gem_player.player.current_song.is_some();
                let response = ui
                    .add_enabled(song_is_playing, play_pause_button)
                    .on_hover_text(tooltip)
                    .on_disabled_hover_text("No current song");
                if response.clicked() {
                    play_or_pause(&mut gem_player.player);
                }

                let next_button = Button::new(RichText::new(icons::ICON_SKIP_NEXT));
                let next_song_exists = !gem_player.player.queue.is_empty();
                let response = ui
                    .add_enabled(next_song_exists, next_button)
                    .on_hover_text("Next")
                    .on_disabled_hover_text("No next song");
                if response.clicked() {
                    play_next(gem_player);
                }
            });

            flex.add_ui(item(), |ui| {
                let artwork_texture_options = TextureOptions::LINEAR.with_mipmap_mode(Some(TextureFilter::Linear));
                let artwork_size = Vec2::splat(ui.available_height());
                let default_artwork = Image::new(include_image!("../assets/music_note_24dp_E8EAED_FILL0_wght400_GRAD0_opsz24.svg"))
                    .texture_options(artwork_texture_options)
                    .fit_to_exact_size(artwork_size);

                let artwork = gem_player
                    .player
                    .current_song
                    .as_ref()
                    .and_then(|song| {
                        song.artwork.as_ref().map(|artwork_bytes| {
                            let artwork_uri = format!("bytes://artwork-{}", song.title.as_deref().unwrap_or("default"));

                            Image::from_bytes(artwork_uri, artwork_bytes.clone())
                                .texture_options(artwork_texture_options)
                                .fit_to_exact_size(artwork_size)
                        })
                    })
                    .unwrap_or(default_artwork);

                ui.add(artwork);

                Flex::vertical().h_full().justify(FlexJustify::Center).show(ui, |flex| {
                    flex.add_ui(item(), |ui| {
                        let mut title = "None".to_string();
                        let mut artist = "None".to_string();
                        let mut album = "None".to_string();
                        let mut position_as_secs = 0.0;
                        let mut song_duration_as_secs = 0.1; // We set to 0.1 so that when no song is playing, the slider is at the start.

                        if let Some(song) = &gem_player.player.current_song {
                            title = song.title.clone().unwrap_or("Unknown Title".to_string());
                            artist = song.artist.clone().unwrap_or("Unknown Artist".to_string());
                            album = song.album.clone().unwrap_or("Unknown Album".to_string());
                            position_as_secs = gem_player.player.sink.get_pos().as_secs_f32();
                            song_duration_as_secs = song.duration.as_secs_f32();
                        }

                        ui.style_mut().spacing.slider_width = 500.0;
                        let playback_progress_slider = Slider::new(&mut position_as_secs, 0.0..=song_duration_as_secs)
                            .trailing_fill(true)
                            .show_value(false)
                            .step_by(1.0); // Step by 1 second.
                        let song_is_playing = gem_player.player.current_song.is_some();
                        let response = ui.add_enabled(song_is_playing, playback_progress_slider);

                        if response.dragged() && gem_player.player.paused_before_scrubbing.is_none() {
                            gem_player.player.paused_before_scrubbing = Some(gem_player.player.sink.is_paused());
                            gem_player.player.sink.pause(); // Pause playback during scrubbing
                        }

                        if response.drag_stopped() {
                            let new_position = Duration::from_secs_f32(position_as_secs);
                            print_info(format!("Seeking to {} of {}", format_duration_to_mmss(new_position), title));
                            if let Err(e) = gem_player.player.sink.try_seek(new_position) {
                                print_error(format!("Error seeking to new position: {:?}", e));
                            }

                            // Resume playback if the player was not paused before scrubbing
                            if gem_player.player.paused_before_scrubbing == Some(false) {
                                gem_player.player.sink.play();
                            }

                            gem_player.player.paused_before_scrubbing = None;
                        }

                        Flex::horizontal().justify(FlexJustify::SpaceBetween).width(500.0).show(ui, |flex| {
                            flex.add_ui(item().shrink(), |ui| {
                                let default_text_style = TextStyle::Body.resolve(ui.style());
                                let default_color = ui.visuals().text_color();
                                let data_format = TextFormat::simple(default_text_style.clone(), Color32::WHITE);

                                let mut job = text::LayoutJob::default();
                                job.append(&title, 0.0, data_format.clone());
                                job.append(" / ", 0.0, TextFormat::simple(default_text_style.clone(), default_color));
                                job.append(&artist, 0.0, data_format.clone());
                                job.append(" / ", 0.0, TextFormat::simple(default_text_style.clone(), default_color));
                                job.append(&album, 0.0, data_format.clone());

                                let song_label = Label::new(job).selectable(false).truncate();
                                ui.add(song_label);
                            });

                            flex.add_ui(item(), |ui| {
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

            flex.add_ui(item(), |ui| {
                let mut volume = gem_player.player.sink.volume();

                let volume_icon = match volume {
                    v if v == 0.0 => icons::ICON_VOLUME_OFF,
                    v if v <= 0.5 => icons::ICON_VOLUME_DOWN,
                    _ => icons::ICON_VOLUME_UP, // v > 0.5 && v <= 1.0
                };
                let tooltip = if gem_player.player.muted { "Unmute" } else { "Mute" };
                let response = ui.button(volume_icon).on_hover_text(tooltip);
                if response.clicked() {
                    gem_player.player.muted = !gem_player.player.muted;
                    if gem_player.player.muted {
                        gem_player.player.volume_before_mute = Some(volume);
                        volume = 0.0;
                    } else if let Some(v) = gem_player.player.volume_before_mute {
                        volume = v;
                    }
                }

                let volume_slider = Slider::new(&mut volume, 0.0..=1.0).trailing_fill(true).show_value(false);
                let changed = ui.add(volume_slider).changed();
                if changed {
                    gem_player.player.muted = false;
                    gem_player.player.volume_before_mute = if volume == 0.0 { None } else { Some(volume) }
                }

                gem_player.player.sink.set_volume(volume);
            });
        });
    });
}

pub fn render_library_ui(ui: &mut Ui, gem_player: &mut GemPlayer) {
    if gem_player.library.is_empty() {
        Frame::none()
            .outer_margin(Margin::symmetric(ui.available_width() * (1.0 / 4.0), 32.0))
            .show(ui, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add(unselectable_label(
                        "The library is empty. Try adding your music directory in the settings.",
                    ));
                });
            });

        return;
    }

    let mut library_copy: Vec<Song> = gem_player
        .library
        .iter()
        .filter(|song| {
            let search_lower = gem_player.ui_state.search_text.to_lowercase();
            let search_fields = [&song.title, &song.artist, &song.album];

            search_fields
                .iter()
                .any(|field| field.as_ref().map_or(false, |text| text.to_lowercase().contains(&search_lower)))
        })
        .cloned()
        .collect();

    sort_songs(&mut library_copy, gem_player.ui_state.sort_by, gem_player.ui_state.sort_order);

    let header_labels = [icons::ICON_MUSIC_NOTE, icons::ICON_ARTIST, icons::ICON_ALBUM, icons::ICON_HOURGLASS];

    let available_width = ui.available_width();
    let time_width = 64.0;
    let remaining_width = available_width - time_width;
    let title_width = remaining_width * 0.5;
    let artist_width = remaining_width * 0.25;
    let album_width = remaining_width * 0.25;

    // Since we are setting the widths of the table columns manually by dividing up the available width,
    // if we leave the default item spacing, the width taken up by the table will be greater than the available width,
    // casuing the right side of the table to be cut off by the window.
    ui.spacing_mut().item_spacing.x = 0.0;

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
            body.rows(26.0, library_copy.len(), |mut row| {
                let song = &library_copy[row.index()];

                let row_is_selected = gem_player.ui_state.selected_library_song.as_ref() == Some(song);
                row.set_selected(row_is_selected);

                row.col(|ui| {
                    ui.add_space(16.0);
                    ui.add(unselectable_label(song.title.as_deref().unwrap_or("Unknown Title")).truncate());
                });

                row.col(|ui| {
                    ui.add(unselectable_label(song.artist.as_deref().unwrap_or("Unknown Artist")).truncate());
                });

                row.col(|ui| {
                    ui.add(unselectable_label(song.album.as_deref().unwrap_or("Unknown")));
                });

                row.col(|ui| {
                    let duration_string = format_duration_to_mmss(song.duration);
                    ui.add(unselectable_label(duration_string));
                });

                let response = row.response();
                if response.clicked() {
                    gem_player.ui_state.selected_library_song = Some(song.clone());
                }

                if response.double_clicked() {
                    play_library_from_song(gem_player, song);
                }

                response.context_menu(|ui| {
                    if ui.button("Play Next").clicked() {
                        add_next_to_queue(&mut gem_player.player.queue, song.clone());
                        ui.close_menu();
                    }

                    if ui.button("Add to queue").clicked() {
                        add_to_queue(&mut gem_player.player.queue, song.clone());
                        ui.close_menu();
                    }

                    ui.separator();

                    if ui.button("Open file location").clicked() {
                        let maybe_folder = song.file_path.as_path().parent();
                        match maybe_folder {
                            Some(folder) => {
                                let result = open::that_detached(folder);
                                match result {
                                    Ok(_) => print_info(format!("Opening file location: {:?}", folder)),
                                    Err(e) => print_error(format!("Error opening file location: {:?}", e)),
                                }
                            }
                            None => {
                                print_info("No file location to open");
                            }
                        }

                        ui.close_menu();
                    }

                    if ui.button("Remove from library").clicked() {
                        ui.close_menu();
                    }
                });
            });
        });
}

pub fn render_queue_ui(ui: &mut Ui, queue: &mut Vec<Song>) {
    if queue.is_empty() {
        Frame::none()
            .outer_margin(Margin::symmetric(ui.available_width() * (1.0 / 4.0), 32.0))
            .show(ui, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add(unselectable_label("The queue is empty."));
                });
            });

        return;
    }

    let header_labels = [
        "",
        icons::ICON_MUSIC_NOTE,
        icons::ICON_ARTIST,
        icons::ICON_ALBUM,
        icons::ICON_HOURGLASS,
        "",
    ];

    let available_width = ui.available_width();
    let position_width = 64.0;
    let time_width = 80.0;
    let actions_width = 80.0;
    let remaining_width = available_width - position_width - time_width - actions_width;
    let title_width = remaining_width * (2.0 / 4.0);
    let artist_width = remaining_width * (1.0 / 4.0);
    let album_width = remaining_width * (1.0 / 4.0);

    ui.spacing_mut().item_spacing.x = 0.0; // See comment in render_library_ui for why we set item_spacing to 0.

    TableBuilder::new(ui)
        .striped(true)
        .sense(Sense::click())
        .cell_layout(Layout::left_to_right(Align::Center))
        .column(egui_extras::Column::exact(position_width))
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
                    ui.add(unselectable_label(RichText::new(*h).strong()));
                });
            }
        })
        .body(|body| {
            body.rows(26.0, queue.len(), |mut row| {
                let index = row.index();
                let song = queue[index].clone();

                row.col(|ui| {
                    ui.add_space(16.0);
                    ui.add(unselectable_label(format!("{}", index + 1)));
                });

                row.col(|ui| {
                    ui.add(unselectable_label(song.title.as_deref().unwrap_or("Unknown Title")));
                });

                row.col(|ui| {
                    ui.add(unselectable_label(song.artist.as_deref().unwrap_or("Unknown Artist")));
                });

                row.col(|ui| {
                    ui.add(unselectable_label(song.album.as_deref().unwrap_or("Unknown")));
                });

                row.col(|ui| {
                    let duration_string = format_duration_to_mmss(song.duration);
                    ui.add(unselectable_label(duration_string));
                });

                // We only display the actions column buttons if the row is hovered. There is a chicken and egg problem here.
                // We need to know if the row is hovered before we display the actions column buttons. So, we check if
                // either the row response (of the previous cells) or the actions column cell contains the pointer.
                let row_is_hovered = row.response().hovered();
                let mut actions_cell_contains_pointer = false;
                row.col(|ui| {
                    actions_cell_contains_pointer = ui.rect_contains_pointer(ui.max_rect());
                    let should_show_action_buttons = row_is_hovered || actions_cell_contains_pointer;

                    ui.add_space(8.0);

                    let response = ui.add_visible(should_show_action_buttons, Button::new(icons::ICON_ARROW_UPWARD));
                    if response.clicked() {
                        move_song_to_front(queue, index);
                    }

                    ui.add_space(8.0);

                    let response = ui.add_visible(should_show_action_buttons, Button::new(icons::ICON_CLOSE));
                    if response.clicked() {
                        remove_from_queue(queue, index);
                    }
                });
            });
        });
}

pub fn render_playlists_ui(ui: &mut Ui, playlists: &mut Vec<Playlist>, playlists_ui_state: &mut PlaylistsUIState) {
    if playlists_ui_state.confirm_delete_playlist_modal_is_open {
        let mut cancel_clicked = false;
        let mut confirm_clicked = false;

        let modal = containers::Modal::new(Id::new("Delete Playlist Modal")).show(ui.ctx(), |ui| {
            ui.set_width(200.0);
            Frame::none().outer_margin(Margin::same(4.0)).show(ui, |ui| {
                let label = unselectable_label(RichText::new("Are you sure you want to delete this playlist?").heading());
                ui.add(label);

                ui.separator();

                containers::Sides::new().show(
                    ui,
                    |ui| {
                        let response = ui.button(format!("\t{}\t", icons::ICON_CLOSE));
                        if response.clicked() {
                            cancel_clicked = true;
                        }
                    },
                    |ui| {
                        let response = ui.button(format!("\t{}\t", icons::ICON_CHECK));
                        if response.clicked() {
                            confirm_clicked = true;
                        }
                    },
                );
            });
        });

        if confirm_clicked {
            if let Some(index) = playlists_ui_state.selected_playlist_index {
                print_info(format!("Confirmed deletion of playlist: {}", playlists[index].name));
                playlists.remove(index);
                playlists_ui_state.selected_playlist_index = None;
            }
            playlists_ui_state.confirm_delete_playlist_modal_is_open = false;
        } else if cancel_clicked || modal.should_close() {
            playlists_ui_state.confirm_delete_playlist_modal_is_open = false;
        }
    }

    let size = ui.available_size();
    let playlists_width = size.x * (1.0 / 4.0);

    StripBuilder::new(ui)
        .size(Size::exact(playlists_width))
        .size(Size::remainder())
        .horizontal(|mut strip| {
            strip.cell(|ui| {
                TableBuilder::new(ui)
                    .striped(true)
                    .sense(Sense::click())
                    .cell_layout(Layout::left_to_right(Align::Center))
                    .column(egui_extras::Column::exact(playlists_width))
                    .header(36.0, |mut header| {
                        header.col(|ui| {
                            containers::Sides::new().height(ui.available_height()).show(
                                ui,
                                |ui| {
                                    ui.add_space(8.0);
                                    ui.add(unselectable_label(RichText::new("Playlists").heading().strong()));
                                },
                                |ui| {
                                    ui.add_space(8.0);

                                    let add_button = Button::new(icons::ICON_ADD);
                                    let response = ui.add(add_button).on_hover_text("Add playlist");
                                    if response.clicked() {
                                        print_info("Adding playlist");

                                        let new_playlist = Playlist {
                                            id: uuid::Uuid::new_v4(),
                                            name: format!("Playlist {}", playlists.len() + 1),
                                            creation_date_time: Utc::now(),
                                            songs: Vec::new(),
                                            path: None,
                                        };

                                        playlists.push(new_playlist);
                                    }
                                },
                            );
                        });
                    })
                    .body(|body| {
                        body.rows(36.0, playlists.len(), |mut row| {
                            let playlist = &mut playlists[row.index()];

                            row.col(|ui| {
                                ui.add_space(8.0);
                                ui.add(unselectable_label(&playlist.name));
                            });

                            let response = row.response();
                            if response.clicked() {
                                print_info(format!("Selected playlist: {}", playlist.name));
                                playlists_ui_state.selected_playlist_index = Some(row.index());
                            }
                        });
                    });
            });

            strip.cell(|ui| {
                let maybe_selected_playlist = playlists_ui_state
                    .selected_playlist_index
                    .and_then(|index| playlists.get_mut(index));
                render_playlist_content(
                    ui,
                    maybe_selected_playlist,
                    &mut playlists_ui_state.edit_playlist_name_info,
                    &mut playlists_ui_state.confirm_delete_playlist_modal_is_open,
                );
            });
        });
}

pub fn render_playlist_content(
    ui: &mut Ui,
    maybe_playlist: Option<&mut Playlist>,
    maybe_edit_playlist_name_info: &mut Option<(Uuid, String)>,
    confirm_delete_playlist_modal_is_open: &mut bool,
) {
    let Some(playlist) = maybe_playlist else {
        ui.add(unselectable_label(RichText::new("").heading()));

        Frame::none()
            .outer_margin(Margin::symmetric(ui.available_width() * (1.0 / 4.0), 32.0))
            .show(ui, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add(unselectable_label("No playlist selected"));
                });
            });

        return;
    };

    StripBuilder::new(ui)
        .size(Size::exact(64.0))
        .size(Size::remainder())
        .vertical(|mut strip| {
            strip.cell(|ui| {
                if let Some((_, name_buffer)) = maybe_edit_playlist_name_info {
                    // In edit mode
                    let mut discard_clicked = false;
                    let mut save_clicked = false;
                    containers::Sides::new().height(ui.available_height()).show(
                        ui,
                        |ui| {
                            ui.add_space(16.0);
                            let name_edit = TextEdit::singleline(name_buffer);
                            ui.add(name_edit);
                        },
                        |ui| {
                            ui.add_space(16.0);

                            let cancel_button = Button::new(icons::ICON_CANCEL);
                            let response = ui.add(cancel_button).on_hover_text("Discard");
                            if response.clicked() {
                                discard_clicked = true;
                            }

                            let confirm_button = Button::new(icons::ICON_SAVE);
                            let response = ui.add(confirm_button).on_hover_text("Save");
                            if response.clicked() {
                                save_clicked = true;
                            }
                        },
                    );
                    if discard_clicked {
                        *maybe_edit_playlist_name_info = None;
                    } else if save_clicked {
                        playlist.name = name_buffer.clone();
                        *maybe_edit_playlist_name_info = None;
                    }
                } else {
                    // Not in edit mode
                    let strip_contains_pointer = ui.rect_contains_pointer(ui.max_rect());
                    containers::Sides::new().height(ui.available_height()).show(
                        ui,
                        |ui| {
                            ui.add_space(16.0);
                            ui.add(unselectable_label(RichText::new(&playlist.name).heading().strong()));
                        },
                        |ui| {
                            if !strip_contains_pointer {
                                return;
                            }

                            ui.add_space(16.0);

                            let delete_button = Button::new(icons::ICON_DELETE);
                            let response = ui.add(delete_button).on_hover_text("Delete");
                            if response.clicked() {
                                print_info(format!("Opening delete playlist modal: {}", playlist.name));
                                *confirm_delete_playlist_modal_is_open = true;
                            }

                            let edit_name_button = Button::new(icons::ICON_EDIT);
                            let response = ui.add(edit_name_button).on_hover_text("Edit name");
                            if response.clicked() {
                                print_info(format!("Editing playlist name: {}", playlist.name));
                                *maybe_edit_playlist_name_info = Some((playlist.id, playlist.name.clone()));
                            }
                        },
                    );
                }

                ui.add(Separator::default().spacing(0.0));
            });

            strip.cell(|ui| {
                if playlist.songs.is_empty() {
                    Frame::none()
                        .outer_margin(Margin::symmetric(ui.available_width() * (1.0 / 4.0), 32.0))
                        .show(ui, |ui| {
                            ui.vertical_centered(|ui| {
                                ui.add(unselectable_label("The playlist is empty."));
                            });
                        });

                    return;
                }

                let header_labels = [icons::ICON_MUSIC_NOTE, icons::ICON_ARTIST, icons::ICON_ALBUM, icons::ICON_HOURGLASS];

                let available_width = ui.available_width();
                let position_width = 64.0;
                let time_width = 80.0;
                let remaining_width = available_width - position_width - time_width;
                let title_width = remaining_width * (2.0 / 4.0);
                let artist_width = remaining_width * (1.0 / 4.0);
                let album_width = remaining_width * (1.0 / 4.0);

                ui.spacing_mut().item_spacing.x = 0.0; // See comment in render_library_ui for why we set item_spacing to 0.

                TableBuilder::new(ui)
                    .striped(true)
                    .sense(Sense::click())
                    .cell_layout(Layout::left_to_right(Align::Center))
                    .column(egui_extras::Column::exact(position_width))
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
                        body.rows(26.0, playlist.songs.len(), |mut row| {
                            let index = row.index();
                            let song = playlist.songs[index].clone();

                            row.col(|ui| {
                                ui.add_space(16.0);
                                ui.add(unselectable_label(format!("{}", index + 1)));
                            });

                            row.col(|ui| {
                                ui.add(unselectable_label(song.title.as_deref().unwrap_or("Unknown Title")));
                            });

                            row.col(|ui| {
                                ui.add(unselectable_label(song.artist.as_deref().unwrap_or("Unknown Artist")));
                            });

                            row.col(|ui| {
                                ui.add(unselectable_label(song.album.as_deref().unwrap_or("Unknown")));
                            });

                            row.col(|ui| {
                                let duration_string = format_duration_to_mmss(song.duration);
                                ui.add(unselectable_label(duration_string));
                            });

                            let row_is_hovered = row.response().hovered();
                            let mut actions_cell_contains_pointer = false;
                            row.col(|ui| {
                                actions_cell_contains_pointer = ui.rect_contains_pointer(ui.max_rect());
                                let should_show_action_button = row_is_hovered || actions_cell_contains_pointer;

                                ui.add_space(8.0);

                                let _response = ui.add_visible(should_show_action_button, Button::new(icons::ICON_MORE));
                                // Show context menu
                            });
                        });
                    });
            });
        });
}

pub fn render_settings_ui(ui: &mut Ui, gem_player: &mut GemPlayer) {
    Frame::none()
        .outer_margin(Margin::symmetric(ui.available_width() * (1.0 / 4.0), 32.0))
        .show(ui, |ui| {
            ScrollArea::vertical().show(ui, |ui| {
                ui.add(unselectable_label(RichText::new("Music Library Path").heading()));

                ui.horizontal(|ui| {
                    let path = gem_player
                        .library_directory
                        .as_ref()
                        .map_or("No directory selected".to_string(), |p| p.to_string_lossy().to_string());
                    ui.label(path);

                    let response = ui.button(icons::ICON_FOLDER_OPEN);
                    if response.clicked() {
                        let maybe_directory = FileDialog::new().set_directory("/").pick_folder();
                        match maybe_directory {
                            Some(directory) => {
                                print_info(format!("Selected folder: {:?}", directory));
                                let _old_folder = gem_player.library_directory.clone();
                            }
                            None => {
                                print_info("No folder selected");
                            }
                        }
                    }
                });

                ui.add(Separator::default().spacing(32.0));

                ui.add(unselectable_label(RichText::new("Theme").heading()));
                ComboBox::from_label("Select Theme")
                    .selected_text(format!("{:?}", gem_player.ui_state.theme))
                    .show_ui(ui, |ui| {
                        let theme_name = |theme: Theme| match theme {
                            Theme::System => "System",
                            Theme::Dark => icons::ICON_NIGHTS_STAY,
                            Theme::Light => icons::ICON_SUNNY,
                        };

                        for theme in Theme::iter() {
                            ui.selectable_value(&mut gem_player.ui_state.theme, theme, theme_name(theme));
                        }
                    });

                ui.add(Separator::default().spacing(32.0));

                ui.add(unselectable_label(RichText::new("About Gem Player").heading()));
                let version = env!("CARGO_PKG_VERSION");
                ui.add(unselectable_label(format!("Version: {version}")));
                ui.add(unselectable_label("Gem Player is a lightweight music player."));

                ui.add(Separator::default().spacing(32.0));

                ui.add(unselectable_label(RichText::new("Author").heading()));
                ui.add(unselectable_label("James Moreau"));
                ui.hyperlink("https://jamesmoreau.github.io");
            });
        });
}

fn render_navigation_ui(ui: &mut Ui, gem_player: &mut GemPlayer) {
    Frame::none().inner_margin(Margin::symmetric(16.0, 4.0)).show(ui, |ui| {
        Flex::horizontal().w_full().justify(FlexJustify::SpaceBetween).show(ui, |flex| {
            flex.add_ui(item(), |ui| {
                let get_icon_and_tooltip = |view: &View| match view {
                    View::Library => icons::ICON_LIBRARY_MUSIC,
                    View::Queue => icons::ICON_QUEUE_MUSIC,
                    View::Playlists => icons::ICON_STAR,
                    View::Settings => icons::ICON_SETTINGS,
                };

                for view in View::iter() {
                    let icon = get_icon_and_tooltip(&view);
                    let response = ui
                        .selectable_label(gem_player.ui_state.current_view == view, format!("  {icon}  "))
                        .on_hover_text(format!("{:?}", view));
                    if response.clicked() {
                        switch_view(gem_player, view);
                    }

                    ui.add_space(4.0);
                }
            });

            flex.add_ui(item(), |ui| match gem_player.ui_state.current_view {
                View::Library => {
                    let songs_count_and_duration = get_count_and_duration_string_from_songs(&gem_player.library);
                    ui.add(unselectable_label(songs_count_and_duration));
                }
                View::Queue => {
                    let songs_count_and_duration = get_count_and_duration_string_from_songs(&gem_player.player.queue);
                    ui.add(unselectable_label(songs_count_and_duration));

                    ui.add_space(8.0);
                }
                View::Playlists => {}
                View::Settings => {}
            });

            flex.add_ui(item(), |ui| match gem_player.ui_state.current_view {
                View::Library => {
                    let refresh_button = Button::new(icons::ICON_REFRESH);
                    let response = ui.add(refresh_button).on_hover_text("Refresh library");
                    if response.clicked() {
                        let library = match &gem_player.library_directory {
                            Some(path) => {
                                let result = read_music_from_a_directory(path);
                                match result {
                                    Ok(songs) => songs,
                                    Err(e) => {
                                        print_error(e.to_string());
                                        gem_player.ui_state.toasts.error(format!("Error refreshing library: {}", e));
                                        Vec::new()
                                    }
                                }
                            }
                            None => Vec::new(),
                        };

                        gem_player.library = library;
                    }

                    ui.add_space(16.0);

                    search_and_filter_ui(ui, gem_player)
                }
                View::Queue => {
                    let queue_is_not_empty = !gem_player.player.queue.is_empty();
                    let shuffle_button = Button::new(RichText::new(icons::ICON_SHUFFLE));
                    let response = ui
                        .add_enabled(queue_is_not_empty, shuffle_button)
                        .on_hover_text("Shuffle")
                        .on_disabled_hover_text("Queue is empty");
                    if response.clicked() {
                        shuffle_queue(&mut gem_player.player.queue);
                    }

                    let repeat_button_color = if gem_player.player.repeat {
                        ui.visuals().selection.bg_fill
                    } else {
                        ui.visuals().text_color()
                    };
                    let repeat_button = Button::new(RichText::new(icons::ICON_REPEAT).color(repeat_button_color));
                    let clicked = ui.add(repeat_button).on_hover_text("Repeat").clicked();
                    if clicked {
                        gem_player.player.repeat = !gem_player.player.repeat;
                    }

                    ui.add_space(16.0);

                    let clear_button = Button::new(icons::ICON_CLEAR_ALL);
                    let response = ui
                        .add_enabled(queue_is_not_empty, clear_button)
                        .on_hover_text("Clear")
                        .on_disabled_hover_text("Queue is empty");
                    if response.clicked() {
                        gem_player.player.queue.clear();
                    }
                }
                View::Playlists => {}
                View::Settings => {}
            });
        });
    });
}

fn get_count_and_duration_string_from_songs(songs: &[Song]) -> String {
    let duration = get_duration_of_songs(songs);
    let duration_string = format_duration_to_hhmmss(duration);
    format!("{} songs / {}", songs.len(), duration_string)
}

fn search_and_filter_ui(ui: &mut Ui, gem_player: &mut GemPlayer) {
    let filter_icon = icons::ICON_FILTER_LIST;
    ui.menu_button(filter_icon, |ui| {
        for sort_by in SortBy::iter() {
            ui.radio_value(&mut gem_player.ui_state.sort_by, sort_by, format!("{:?}", sort_by));
        }

        ui.separator();

        for sort_order in SortOrder::iter() {
            ui.radio_value(&mut gem_player.ui_state.sort_order, sort_order, format!("{:?}", sort_order));
        }
    })
    .response
    .on_hover_text("Sort by and order");

    let search_bar = TextEdit::singleline(&mut gem_player.ui_state.search_text)
        .hint_text(format!("{} Search ...", icons::ICON_SEARCH))
        .desired_width(140.0);
    ui.add(search_bar).on_hover_text("Search");

    let clear_button_is_visible = !gem_player.ui_state.search_text.is_empty();
    let response = ui
        .add_visible(clear_button_is_visible, Button::new(icons::ICON_CLEAR))
        .on_hover_text("Clear search");
    if response.clicked() {
        gem_player.ui_state.search_text.clear();
    }
}

fn unselectable_label(text: impl Into<RichText>) -> Label {
    Label::new(text.into()).selectable(false)
}
