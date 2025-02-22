use std::time::Duration;

use dark_light::Mode;
use eframe::egui::{
    containers, include_image, popup, text, AboveOrBelow, Align, Align2, Button, CentralPanel, Color32, Context, FontId, Frame, Id, Image,
    Label, Layout, Margin, PointerButton, RichText, ScrollArea, Sense, Separator, Slider, Style, TextEdit, TextFormat, TextStyle,
    TextureFilter, TextureOptions, ThemePreference, Ui, UiBuilder, Vec2, ViewportCommand, Visuals,
};
use egui_extras::{Size, StripBuilder, TableBuilder};
use egui_flex::{item, Flex, FlexJustify};
use egui_material_icons::icons;
use egui_notify::Toasts;
use fully_pub::fully_pub;
use log::{error, info, warn};
use rfd::FileDialog;
use strum::IntoEnumIterator;
use strum_macros::EnumIter;
use uuid::Uuid;

use crate::{
    format_duration_to_hhmmss, format_duration_to_mmss,
    player::{
        add_next_to_queue, is_playing, move_song_to_front, play_or_pause, read_music_and_playlists_from_directory, remove_from_queue,
        shuffle_queue, GemPlayer, PlayerAction, KEY_COMMANDS,
    },
    playlist::{
        add_a_song_to_playlist, create_a_new_playlist, delete_playlist, find_playlist_mut, remove_a_song_from_playlist, rename_playlist,
        Playlist,
    },
    song::{find_song, get_duration_of_songs, open_song_file_location, sort_songs, SortBy, SortOrder},
    Song,
};

#[derive(Debug, Clone, PartialEq, Eq, EnumIter)]
pub enum View {
    Library,
    Playlists,
    Queue,
    Settings,
}

#[fully_pub]
pub struct UIState {
    current_view: View,
    theme_preference: ThemePreference,
    library_view_state: LibraryViewState,
    playlists_view_state: PlaylistsViewState,
    toasts: Toasts,
}

#[fully_pub]
pub struct LibraryViewState {
    selected_song: Option<Uuid>,
    song_menu_is_open: Option<Uuid>, // None: no song menu open, Some: the id of the song with the menu.
    sort_by: SortBy,
    sort_order: SortOrder,
    search_text: String,
}

#[fully_pub]
pub struct PlaylistsViewState {
    selected_playlist_id: Option<Uuid>,
    playlist_rename: Option<(Uuid, String)>, // None: no playlist is being edited. Some: the id of the playlist being edited and a buffer for the new name.
    delete_playlist_modal_is_open: Option<Uuid>, // None: the modal is not open. Some: the modal for a specific playlist.
    selected_song_id: Option<Uuid>,
}

pub fn update_theme(gem_player: &mut GemPlayer, ctx: &Context) {
    match gem_player.ui_state.theme_preference {
        ThemePreference::Dark => ctx.set_visuals(Visuals::dark()),
        ThemePreference::Light => ctx.set_visuals(Visuals::light()),
        ThemePreference::System => {
            let visuals = match dark_light::detect() {
                Ok(Mode::Light) => Visuals::light(),
                _ => Visuals::dark(), // Covers both Mode::Dark, Mode::Unspecified, and errors
            };
            ctx.set_visuals(visuals);
        }
    }
}

pub fn render_gem_player(gem_player: &mut GemPlayer, ctx: &Context) {
    custom_window_frame(ctx, "", |ui| {
        let control_ui_height = 64.0;
        let navigation_ui_height = 32.0;
        let separator_space = 2.0; // Even numbers seem to work better for getting pixel perfect placements.

        StripBuilder::new(ui)
            .size(Size::exact(separator_space))
            .size(Size::exact(control_ui_height))
            .size(Size::exact(separator_space))
            .size(Size::remainder())
            .size(Size::exact(separator_space))
            .size(Size::exact(navigation_ui_height))
            .vertical(|mut strip| {
                strip.cell(|ui| {
                    ui.add(Separator::default().spacing(separator_space));
                });
                strip.cell(|ui| {
                    render_control_ui(ui, gem_player);
                });
                strip.cell(|ui| {
                    ui.add(Separator::default().spacing(separator_space));
                });
                strip.cell(|ui| match gem_player.ui_state.current_view {
                    View::Library => render_library_ui(ui, gem_player),
                    View::Queue => render_queue_ui(ui, &mut gem_player.player.queue),
                    View::Playlists => render_playlists_ui(ui, gem_player),
                    View::Settings => render_settings_ui(ui, gem_player),
                });
                strip.cell(|ui| {
                    ui.add(Separator::default().spacing(separator_space));
                });
                strip.cell(|ui| {
                    render_navigation_ui(ui, gem_player);
                });
            });
    });
}

pub fn custom_window_frame(ctx: &Context, title: &str, add_contents: impl FnOnce(&mut Ui)) {
    let panel_frame = Frame {
        fill: ctx.style().visuals.window_fill(),
        corner_radius: 10.0.into(),
        stroke: ctx.style().visuals.widgets.noninteractive.bg_stroke,
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
        .shrink2(Vec2::new(2.0, 0.0));
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
    // painter.line_segment(
    //     [
    //         title_bar_rect.left_bottom() + vec2(1.0, 0.0),
    //         title_bar_rect.right_bottom() + vec2(-1.0, 0.0),
    //     ],
    //     ui.visuals().widgets.noninteractive.bg_stroke,
    // );

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

pub fn switch_view(ui_state: &mut UIState, view: View) {
    info!("Switching to view: {:?}", view);
    ui_state.current_view = view;
}

pub fn render_control_ui(ui: &mut Ui, gem_player: &mut GemPlayer) {
    Frame::new().inner_margin(Margin::symmetric(16, 0)).show(ui, |ui| {
        Flex::horizontal()
            .h_full()
            .w_full()
            .justify(FlexJustify::SpaceBetween)
            .show(ui, |flex| {
                flex.add_ui(item(), |ui| {
                    let previous_button = Button::new(RichText::new(icons::ICON_SKIP_PREVIOUS));
                    let is_previous_enabled = gem_player.player.current_song.is_some() || !gem_player.player.history.is_empty();

                    let response = ui
                        .add_enabled(is_previous_enabled, previous_button)
                        .on_hover_text("Previous")
                        .on_disabled_hover_text("No previous song");
                    if response.clicked() {
                        gem_player.player.actions.push(PlayerAction::PlayPrevious);
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
                        gem_player.player.actions.push(PlayerAction::PlayNext);
                    }
                });

                flex.add_ui(item(), |ui| {
                    let artwork_texture_options = TextureOptions::LINEAR.with_mipmap_mode(Some(TextureFilter::Linear));
                    let artwork_size = Vec2::splat(ui.available_height());

                    let mut artwork = Image::new(include_image!("../assets/music_note_24dp_E8EAED_FILL0_wght400_GRAD0_opsz24.svg"));
                    if let Some(song) = &gem_player.player.current_song {
                        if let Some(artwork_bytes) = &song.artwork {
                            let artwork_uri = format!("bytes://artwork-{}", song.id);
                            artwork = Image::from_bytes(artwork_uri, artwork_bytes.clone())
                        }
                    }

                    ui.add(
                        artwork
                            .texture_options(artwork_texture_options)
                            .fit_to_exact_size(artwork_size)
                            .maintain_aspect_ratio(false)
                            .corner_radius(2.0),
                    );

                    Flex::vertical().h_full().justify(FlexJustify::Center).show(ui, |flex| {
                        flex.add_ui(item(), |ui| {
                            let mut title = "None";
                            let mut artist = "None";
                            let mut album = "None";
                            let mut position_as_secs = 0.0;
                            let mut song_duration_as_secs = 0.1; // We set to 0.1 so that when no song is playing, the slider is at the start.

                            if let Some(song) = &gem_player.player.current_song {
                                title = song.title.as_deref().unwrap_or("Unknown Title");
                                artist = song.artist.as_deref().unwrap_or("Unknown Artist");
                                album = song.album.as_deref().unwrap_or("Unknown Album");
                                position_as_secs = gem_player.player.sink.get_pos().as_secs_f32();
                                song_duration_as_secs = song.duration.as_secs_f32();
                            }

                            let playback_progress_slider_width = 500.0;
                            ui.style_mut().spacing.slider_width = playback_progress_slider_width;
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
                                info!("Seeking to {} of {}", format_duration_to_mmss(new_position), title);
                                if let Err(e) = gem_player.player.sink.try_seek(new_position) {
                                    error!("Error seeking to new position: {:?}", e);
                                }

                                // Resume playback if the player was not paused before scrubbing
                                if gem_player.player.paused_before_scrubbing == Some(false) {
                                    gem_player.player.sink.play();
                                }

                                gem_player.player.paused_before_scrubbing = None;
                            }

                            ui.add_space(8.0);

                            // Placing the song info after the slider ensures that the playback position display is accurate. The seek operation is only
                            // executed after the slider thumb is released. If we placed the display before, the current position would not be reflected.
                            Flex::horizontal()
                                .justify(FlexJustify::SpaceBetween)
                                .width(playback_progress_slider_width)
                                .show(ui, |flex| {
                                    flex.add_ui(item().basis(playback_progress_slider_width * (4.0 / 5.0)), |ui| {
                                        let leading_space = 0.0;
                                        let style = ui.style();
                                        let text_color = ui.visuals().text_color();
                                        let divider_color = ui.visuals().weak_text_color();

                                        let get_text_format =
                                            |style: &Style, color: Color32| TextFormat::simple(TextStyle::Body.resolve(style), color);

                                        let mut job = text::LayoutJob::default();
                                        job.append(title, leading_space, get_text_format(style, text_color));
                                        job.append(" / ", leading_space, get_text_format(style, divider_color));
                                        job.append(artist, leading_space, get_text_format(style, text_color));
                                        job.append(" / ", leading_space, get_text_format(style, divider_color));
                                        job.append(album, leading_space, get_text_format(style, text_color));

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
    // TODO: right click should select the song (as with left click).
    if gem_player.library.is_empty() {
        Frame::new()
            .outer_margin(Margin::symmetric((ui.available_width() * (1.0 / 4.0)) as i8, 32))
            .show(ui, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add(unselectable_label(
                        "The library is empty. Try adding your music directory in the settings.",
                    ));
                });
            });

        return;
    }

    if gem_player.ui_state.library_view_state.song_menu_is_open.is_some() {
        render_library_song_menu_modal(ui, gem_player);
    }

    let mut library_copy: Vec<Song> = gem_player
        .library
        .iter()
        .filter(|song| {
            let search_lower = gem_player.ui_state.library_view_state.search_text.to_lowercase();
            let search_fields = [&song.title, &song.artist, &song.album];

            search_fields
                .iter()
                .any(|field| field.as_ref().map_or(false, |text| text.to_lowercase().contains(&search_lower)))
        })
        .cloned()
        .collect();

    sort_songs(
        &mut library_copy,
        gem_player.ui_state.library_view_state.sort_by,
        gem_player.ui_state.library_view_state.sort_order,
    );

    let header_labels = [icons::ICON_MUSIC_NOTE, icons::ICON_ARTIST, icons::ICON_ALBUM, icons::ICON_HOURGLASS];

    let available_width = ui.available_width();
    let time_width = 64.0;
    let more_width = 48.0;
    let remaining_width = available_width - time_width - more_width;
    let title_width = remaining_width * 0.5;
    let artist_width = remaining_width * 0.25;
    let album_width = remaining_width * 0.25;

    // Since we are setting the widths of the table columns manually by dividing up the available width,
    // if we leave the default item spacing, the width taken up by the table will be greater than the available width,
    // causing the right side of the table to be cut off by the window.
    ui.spacing_mut().item_spacing.x = 0.0;

    TableBuilder::new(ui)
        .striped(true)
        .sense(Sense::click())
        .cell_layout(Layout::left_to_right(Align::Center))
        .column(egui_extras::Column::exact(title_width))
        .column(egui_extras::Column::exact(artist_width))
        .column(egui_extras::Column::exact(album_width))
        .column(egui_extras::Column::exact(time_width))
        .column(egui_extras::Column::exact(more_width))
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

                let row_is_selected = gem_player
                    .ui_state
                    .library_view_state
                    .selected_song
                    .as_ref()
                    .map_or(false, |selected_song| *selected_song == song.id);
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

                let rest_of_row_is_hovered = row.response().hovered();
                let mut more_cell_contains_pointer = false;
                row.col(|ui| {
                    more_cell_contains_pointer = ui.rect_contains_pointer(ui.max_rect());
                    let should_show_more_button: bool = rest_of_row_is_hovered || more_cell_contains_pointer || row_is_selected;

                    ui.add_space(8.0);

                    ui.scope_builder(
                        {
                            if should_show_more_button {
                                UiBuilder::new()
                            } else {
                                UiBuilder::new().invisible()
                            }
                        },
                        |ui| {
                            let more_button = Button::new(icons::ICON_MORE_HORIZ);
                            let response = ui.add(more_button).on_hover_text("More");
                            if response.clicked() {
                                gem_player.ui_state.library_view_state.selected_song = Some(song.id);
                                gem_player.ui_state.library_view_state.song_menu_is_open = Some(song.id);
                            }
                        },
                    );
                });

                let response = row.response();

                if response.clicked() {
                    gem_player.ui_state.library_view_state.selected_song = Some(song.id);
                }

                if response.double_clicked() {
                    gem_player.player.actions.push(PlayerAction::PlayFromLibrary { song_id: song.id });
                }
            });
        });
}

pub fn render_library_song_menu_modal(ui: &mut Ui, gem_player: &mut GemPlayer) {
    let Some(song_id) = gem_player.ui_state.library_view_state.song_menu_is_open else {
        return;
    };

    let modal_width = 220.0;

    let modal = containers::Modal::new(Id::new("library_song_menu_modal"))
        .backdrop_color(Color32::TRANSPARENT)
        .show(ui.ctx(), |ui| {
            ui.set_width(220.0);

            ui.vertical_centered_justified(|ui| {
                let song = find_song(song_id, &gem_player.library);
                if let Some(song) = song {
                    ui.label(RichText::new(song.title.as_deref().unwrap_or("Unknown Title")).strong());

                    ui.add_space(8.0);

                    let add_to_playlists_enabled = !gem_player.playlists.is_empty();
                    ui.add_enabled_ui(add_to_playlists_enabled, |ui| {
                        ui.menu_button("Add to Playlist", |ui| {
                            ui.set_min_width(modal_width);

                            ScrollArea::vertical().max_height(164.0).show(ui, |ui| {
                                for playlist in gem_player.playlists.iter_mut() {
                                    if ui.button(&playlist.name).clicked() {
                                        let result = add_a_song_to_playlist(playlist, song.clone());
                                        if let Err(e) = result {
                                            error!("{}", e);
                                            gem_player.ui_state.toasts.error(format!("{}", e));
                                        }
                                    }
                                }
                            });
                        });
                    });

                    ui.separator();

                    let response = ui.button(format!("{} Play Next", icons::ICON_PLAY_ARROW));
                    if response.clicked() {
                        add_next_to_queue(&mut gem_player.player.queue, song.clone());
                        gem_player.ui_state.library_view_state.song_menu_is_open = None;
                    }

                    let response = ui.button(format!("{} Add to Queue", icons::ICON_ADD));
                    if response.clicked() {
                        gem_player
                            .player
                            .actions
                            .push(PlayerAction::AddSongToQueueFromLibrary { song_id: song.id });
                    }

                    ui.separator();

                    let response = ui.button(format!("{} Open File Location", icons::ICON_FOLDER));
                    if response.clicked() {
                        let result = open_song_file_location(song);
                        match result {
                            Ok(_) => info!("Opening song location"),
                            Err(e) => error!("{}", e),
                        }
                    }
                } else {
                    ui.label("Error: Song not found.");
                }
            });
        });

    if modal.should_close() {
        gem_player.ui_state.library_view_state.song_menu_is_open = None;
    }
}

pub fn render_queue_ui(ui: &mut Ui, queue: &mut Vec<Song>) {
    if queue.is_empty() {
        Frame::new()
            .outer_margin(Margin::symmetric((ui.available_width() * (1.0 / 4.0)) as i8, 32))
            .show(ui, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add(unselectable_label("The queue is empty."));
                });
            });

        return;
    }

    let header_labels = [
        icons::ICON_TAG,
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
        .sense(Sense::hover())
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
                let song = &queue[index];

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

pub fn render_playlists_ui(ui: &mut Ui, gem_player: &mut GemPlayer) {
    if gem_player.library_directory.is_none() {
        Frame::new()
            .outer_margin(Margin::symmetric((ui.available_width() * (1.0 / 4.0)) as i8, 32))
            .show(ui, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add(unselectable_label("Try adding your music directory in the settings."));
                });
            });

        return;
    };

    if gem_player.ui_state.playlists_view_state.delete_playlist_modal_is_open.is_some() {
        render_delete_playlist_modal(ui, gem_player);
    }

    let size = ui.available_size();
    let playlists_width = size.x * (1.0 / 4.0);

    ui.spacing_mut().item_spacing.x = 0.0; // See comment in render_library_ui() as to why we do this.

    StripBuilder::new(ui)
        .size(Size::exact(playlists_width))
        .size(Size::exact(6.0))
        .size(Size::remainder())
        .horizontal(|mut strip| {
            strip.cell(|ui| {
                render_playlists_list(ui, gem_player);
            });

            strip.cell(|ui| {
                ui.add(Separator::default().vertical());
            });

            strip.cell(|ui| {
                render_playlist_content(ui, gem_player);
            });
        });
}

pub fn render_delete_playlist_modal(ui: &mut Ui, gem_player: &mut GemPlayer) {
    let Some(playlist_id) = gem_player.ui_state.playlists_view_state.delete_playlist_modal_is_open else {
        return;
    };

    let mut cancel_clicked = false;
    let mut confirm_clicked = false;

    let modal = containers::Modal::new(Id::new("delete_playlist_modal"))
        .backdrop_color(Color32::TRANSPARENT)
        .show(ui.ctx(), |ui| {
            ui.set_width(200.0);
            Frame::new().outer_margin(Margin::same(4)).show(ui, |ui| {
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

                            let result = delete_playlist(playlist_id, &mut gem_player.playlists);
                            if let Err(e) = result {
                                error!("{}", e);
                                return;
                            }

                            let message =
                                "Playlist was deleted successfully. If this was a mistake, the m3u file can be found in the trash.";
                            info!("{}", message);
                            gem_player.ui_state.toasts.success(message);
                            gem_player.ui_state.playlists_view_state.selected_playlist_id = None;
                        }
                    },
                );
            });
        });

    if confirm_clicked || cancel_clicked || modal.should_close() {
        gem_player.ui_state.playlists_view_state.delete_playlist_modal_is_open = None;
    }
}

pub fn render_playlists_list(ui: &mut Ui, gem_player: &mut GemPlayer) {
    let width = ui.available_width();
    TableBuilder::new(ui)
        .striped(true)
        .sense(Sense::click())
        .cell_layout(Layout::left_to_right(Align::Center))
        .column(egui_extras::Column::exact(width))
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
                            let maybe_library_directory = &gem_player.library_directory;
                            match maybe_library_directory {
                                Some(directory) => {
                                    let new_playlist_name = format!("Playlist {}", gem_player.playlists.len() + 1);
                                    let result = create_a_new_playlist(new_playlist_name, directory);
                                    match result {
                                        Ok(new_playlist) => {
                                            info!("Created and saved: {}.", &new_playlist.name);
                                            gem_player.playlists.push(new_playlist);
                                        }
                                        Err(e) => {
                                            let error_message = format!("Failed to create: {}.", e);
                                            error!("{}", &error_message);
                                            gem_player.ui_state.toasts.error(&error_message);
                                        }
                                    }
                                }
                                None => {
                                    error!("This should be unreachable state. We checked library directory is Some earlier!");
                                }
                            }
                        }
                    },
                );
            });
        })
        .body(|body| {
            body.rows(36.0, gem_player.playlists.len(), |mut row| {
                let playlist = &mut gem_player.playlists[row.index()];

                if let Some(id) = gem_player.ui_state.playlists_view_state.selected_playlist_id {
                    let playlist_is_selected = id == playlist.id;
                    row.set_selected(playlist_is_selected);
                }

                row.col(|ui| {
                    ui.add_space(8.0);
                    ui.add(unselectable_label(&playlist.name));
                });

                let response = row.response();
                if response.clicked() {
                    info!("Selected playlist: {}", playlist.name);
                    gem_player.ui_state.playlists_view_state.selected_playlist_id = Some(playlist.id);

                    // Reset in case we were currently editing.
                    gem_player.ui_state.playlists_view_state.playlist_rename = None;
                }
            });
        });
}

pub fn render_playlist_content(ui: &mut Ui, gem_player: &mut GemPlayer) {
    StripBuilder::new(ui)
        .size(Size::exact(64.0))
        .size(Size::remainder())
        .vertical(|mut strip| {
            strip.cell(|ui| {
                let maybe_selected_playlist = gem_player
                    .ui_state
                    .playlists_view_state
                    .selected_playlist_id
                    .and_then(|id| find_playlist_mut(id, &mut gem_player.playlists));

                let Some(playlist) = maybe_selected_playlist else {
                    return;
                };

                Frame::new().fill(ui.visuals().faint_bg_color).show(ui, |ui| {
                    if let Some((_, name_buffer)) = &mut gem_player.ui_state.playlists_view_state.playlist_rename {
                        // In edit mode
                        let mut discard_clicked = false;
                        let mut save_clicked = false;

                        containers::Sides::new().height(ui.available_height()).show(
                            ui,
                            |ui| {
                                ui.add_space(16.0);
                                let name_edit = TextEdit::singleline(name_buffer).char_limit(50);
                                ui.add(name_edit);
                            },
                            |ui| {
                                ui.add_space(16.0);

                                let cancel_button = Button::new(icons::ICON_CANCEL);
                                let response = ui.add(cancel_button).on_hover_text("Discard");
                                if response.clicked() {
                                    discard_clicked = true;
                                }

                                ui.add_space(8.0);

                                let confirm_button = Button::new(icons::ICON_SAVE);
                                let response = ui.add(confirm_button).on_hover_text("Save");
                                if response.clicked() {
                                    save_clicked = true;
                                }
                            },
                        );

                        if discard_clicked {
                            gem_player.ui_state.playlists_view_state.playlist_rename = None;
                        } else if save_clicked {
                            let name_buffer_clone = name_buffer.to_owned();
                            let result = rename_playlist(playlist, name_buffer_clone);
                            if let Err(e) = result {
                                error!("{}", e);
                            }

                            gem_player.ui_state.playlists_view_state.playlist_rename = None;
                        }

                        return;
                    }

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
                                info!("Opening delete playlist modal: {}", playlist.name);
                                gem_player.ui_state.playlists_view_state.delete_playlist_modal_is_open = Some(playlist.id);
                            }

                            ui.add_space(8.0);

                            let edit_name_button = Button::new(icons::ICON_EDIT);
                            let response = ui.add(edit_name_button).on_hover_text("Edit name");
                            if response.clicked() {
                                info!("Editing playlist name: {}", playlist.name);
                                gem_player.ui_state.playlists_view_state.playlist_rename = Some((playlist.id, playlist.name.clone()));
                            }
                        },
                    );
                });
            });

            strip.cell(|ui| {
                render_playlist_songs(ui, gem_player);
            });
        });
}

pub fn render_playlist_songs(ui: &mut Ui, gem_player: &mut GemPlayer) {
    let maybe_selected_playlist_id = gem_player.ui_state.playlists_view_state.selected_playlist_id;
    let Some(playlist_id) = maybe_selected_playlist_id else {
        Frame::new()
            .outer_margin(Margin::symmetric((ui.available_width() * (1.0 / 4.0)) as i8, 32))
            .show(ui, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add(unselectable_label("No playlist selected"));
                });
            });

        return;
    };

    let Some(playlist) = find_playlist_mut(playlist_id, &mut gem_player.playlists) else {
        return; // If we have an id for a playlist but cannot find it, then there's nothing to do.
    };

    if playlist.songs.is_empty() {
        Frame::new()
            .outer_margin(Margin::symmetric((ui.available_width() * (1.0 / 4.0)) as i8, 32))
            .show(ui, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add(unselectable_label("The playlist is empty."));
                });
            });

        return;
    }

    let header_labels = [
        icons::ICON_TAG,
        icons::ICON_MUSIC_NOTE,
        icons::ICON_ARTIST,
        icons::ICON_ALBUM,
        icons::ICON_HOURGLASS,
    ];

    let available_width = ui.available_width();
    let position_width = 64.0;
    let time_width = 80.0;
    let more_width = 48.0;
    let remaining_width = available_width - position_width - time_width - more_width;
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
        .column(egui_extras::Column::exact(more_width))
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

                let rest_of_row_is_hovered = row.response().hovered();
                let mut more_cell_contains_pointer = false;
                row.col(|ui| {
                    more_cell_contains_pointer = ui.rect_contains_pointer(ui.max_rect());
                    let should_show_more_button = rest_of_row_is_hovered || more_cell_contains_pointer;

                    ui.add_space(8.0);

                    ui.scope_builder(
                        {
                            if should_show_more_button {
                                UiBuilder::new()
                            } else {
                                UiBuilder::new().invisible()
                            }
                        },
                        |ui| {
                            ui.menu_button(icons::ICON_MORE_HORIZ, |ui| {
                                playlist_content_context_menu(ui, playlist, &song);
                            });
                        },
                    );
                });

                let response = row.response();

                if response.clicked() {
                    gem_player.ui_state.playlists_view_state.selected_song_id = Some(song.id);
                }

                if response.double_clicked() {
                    gem_player.player.actions.push(PlayerAction::PlayFromPlaylist {
                        playlist_id: playlist.id,
                        song_id: song.id,
                    });
                }

                response.context_menu(|ui| {
                    playlist_content_context_menu(ui, playlist, &song);
                });
            });
        });
}

pub fn playlist_content_context_menu(ui: &mut Ui, playlist: &mut Playlist, song: &Song) {
    ui.set_min_width(128.0);

    if ui.button("Remove from playlist").clicked() {
        let result = remove_a_song_from_playlist(playlist, song.id);
        match result {
            Ok(_) => info!("Removed song from playlist: {}", song.title.as_deref().unwrap_or("Unknown Title")),
            Err(e) => error!("Error removing song from playlist: {:?}", e),
        }

        ui.close_menu();
    }

    ui.separator();

    if ui.button("Open file location").clicked() {
        let result = open_song_file_location(song);
        match result {
            Ok(_) => info!("Opening song location"),
            Err(e) => error!("{}", e),
        }

        ui.close_menu();
    }
}

pub fn render_settings_ui(ui: &mut Ui, gem_player: &mut GemPlayer) {
    Frame::new()
        .outer_margin(Margin::symmetric((ui.available_width() * (1.0 / 4.0)) as i8, 32))
        .show(ui, |ui| {
            ScrollArea::vertical().show(ui, |ui| {
                ui.add(unselectable_label(RichText::new("Music Library Path").heading()));
                ui.add_space(8.0);
                ui.add(unselectable_label("Playlists are also stored here as .m3u files."));
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
                                info!("Selected folder: {:?}", directory);

                                let (found_music, found_playlists) = read_music_and_playlists_from_directory(&directory);
                                gem_player.library = found_music;
                                gem_player.playlists = found_playlists;
                                gem_player.library_directory = Some(directory);
                            }
                            None => {
                                info!("No folder selected");
                            }
                        }
                    }
                });

                ui.add(Separator::default().spacing(32.0));

                ui.add(unselectable_label(RichText::new("Theme").heading()));
                ui.add_space(8.0);

                ThemePreference::radio_buttons(&mut gem_player.ui_state.theme_preference, ui);

                ui.add(Separator::default().spacing(32.0));

                ui.add(unselectable_label(RichText::new("About Gem Player").heading()));
                ui.add_space(8.0);
                let version = env!("CARGO_PKG_VERSION");
                ui.add(unselectable_label(format!("Version: {version}")));
                ui.add(unselectable_label("Gem Player is a lightweight and minimalist music player."));

                ui.add(Separator::default().spacing(32.0));

                ui.add(unselectable_label(RichText::new("Author").heading()));
                ui.add_space(8.0);
                ui.add(unselectable_label("James Moreau"));
                ui.hyperlink("https://jamesmoreau.github.io");

                ui.add(Separator::default().spacing(32.0));

                ui.add(unselectable_label(RichText::new("Key Commands").heading()));
                ui.add_space(8.0);
                for (key, binding) in KEY_COMMANDS.iter() {
                    containers::Sides::new().show(
                        ui,
                        |ui| {
                            ui.add(unselectable_label(format!("{:?}", key)));
                        },
                        |ui| {
                            ui.add_space(16.0);
                            ui.label(*binding);
                        },
                    );
                }
            });
        });
}

fn render_navigation_ui(ui: &mut Ui, gem_player: &mut GemPlayer) {
    Frame::new().inner_margin(Margin::symmetric(16, 0)).show(ui, |ui| {
        Flex::horizontal()
            .h_full()
            .w_full()
            .justify(FlexJustify::SpaceBetween)
            .show(ui, |flex| {
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
                            switch_view(&mut gem_player.ui_state, view);
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
                            match &gem_player.library_directory {
                                Some(directory) => {
                                    let (found_music, found_playlists) = read_music_and_playlists_from_directory(directory);
                                    gem_player.library = found_music;
                                    gem_player.playlists = found_playlists;
                                }
                                None => warn!("Cannot refresh library, as there is no library path."),
                            }
                        }

                        ui.add_space(16.0);

                        render_sort_by_and_search(ui, gem_player)
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

fn render_sort_by_and_search(ui: &mut Ui, gem_player: &mut GemPlayer) {
    let response = ui.button(icons::ICON_FILTER_LIST).on_hover_text("Sort by and order");
    let popup_id = ui.make_persistent_id("filter_popup");
    if response.clicked() {
        ui.memory_mut(|mem| mem.toggle_popup(popup_id));
    }

    let below = AboveOrBelow::Above;
    let close_on_click_outside = popup::PopupCloseBehavior::CloseOnClickOutside;
    popup::popup_above_or_below_widget(ui, popup_id, &response, below, close_on_click_outside, |ui| {
        ui.set_min_width(100.0);

        for sort_by in SortBy::iter() {
            ui.radio_value(
                &mut gem_player.ui_state.library_view_state.sort_by,
                sort_by,
                format!("{:?}", sort_by),
            );
        }

        ui.separator();

        for sort_order in SortOrder::iter() {
            ui.radio_value(
                &mut gem_player.ui_state.library_view_state.sort_order,
                sort_order,
                format!("{:?}", sort_order),
            );
        }
    });

    let search_bar = TextEdit::singleline(&mut gem_player.ui_state.library_view_state.search_text)
        .hint_text(format!("{} Search ...", icons::ICON_SEARCH))
        .desired_width(140.0)
        .char_limit(20);
    ui.add(search_bar);

    let clear_button_is_visible = !gem_player.ui_state.library_view_state.search_text.is_empty();
    let response = ui
        .add_visible(clear_button_is_visible, Button::new(icons::ICON_CLEAR))
        .on_hover_text("Clear search");
    if response.clicked() {
        gem_player.ui_state.library_view_state.search_text.clear();
    }
}

fn unselectable_label(text: impl Into<RichText>) -> Label {
    Label::new(text.into()).selectable(false)
}
