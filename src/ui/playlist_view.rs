use egui::{Align, Button, Color32, Frame, Id, Label, Layout, Margin, Popup, RichText, Sense, Separator, TextEdit, Ui, containers};
use egui_extras::{Size, StripBuilder, TableBuilder};
use egui_material_icons::icons::{
    ICON_ADD, ICON_ALBUM, ICON_ARTIST, ICON_CANCEL, ICON_CHECK, ICON_CLOSE, ICON_DELETE, ICON_EDIT, ICON_FOLDER, ICON_HOURGLASS,
    ICON_MORE_HORIZ, ICON_MUSIC_NOTE, ICON_PLAY_ARROW, ICON_SAVE, ICON_TAG,
};
use fully_pub::fully_pub;
use log::{error, info};
use std::path::PathBuf;

use crate::{
    GemPlayer,
    commands::GemCommand,
    playlist::{PlaylistRetrieval, create, delete, rename},
    track::{Track, filter},
    ui::{
        root::{format_duration_to_mmss, playing_indicator, table_label, unselectable_label},
        widgets::centered_frame::centered_frame,
    },
};

#[fully_pub]
struct PlaylistsViewState {
    selected_playlist_key: Option<PathBuf>, // None: no playlist is selected. Some: the path of the selected playlist.
    selected_tracks: Vec<PathBuf>,

    cached_playlist_tracks: Vec<Track>,
    cache_dirty: bool,

    rename_buffer: Option<String>, // If Some, the playlist pointed to by selected_track's name is being edited and a buffer for the new name.
    delete_modal_open: bool,       // The menu is open for selected_playlist_path.
}

pub fn playlists_view(ui: &mut Ui, gem: &mut GemPlayer) {
    ui.scope(|ui| {
        if gem.library_directory.is_none() {
            centered_frame(ui, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add(unselectable_label("Try adding your music directory in the settings"));
                });
            });

            return;
        };

        if gem.ui.playlists.delete_modal_open {
            delete_playlist_modal(ui, gem);
        }

        let size = ui.available_size();
        let playlists_width = size.x * (1.0 / 4.0);

        ui.spacing_mut().item_spacing.x = 0.0; // See comment in library_view as to why we do this.

        StripBuilder::new(ui)
            .size(Size::exact(playlists_width))
            .size(Size::exact(6.0))
            .size(Size::remainder())
            .horizontal(|mut strip| {
                strip.cell(|ui| {
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

                                        let add_button = Button::new(ICON_ADD);
                                        if ui.add(add_button).on_hover_text("Add playlist").clicked() {
                                            let directory = gem.library_directory.as_ref().unwrap(); // We checked earlier so this is safe.
                                            let new_playlist_name = format!("Playlist {}", gem.playlists.len() + 1);
                                            let result = create(new_playlist_name, directory);
                                            match result {
                                                Err(e) => {
                                                    let error_message = format!("Failed to create: {}.", e);
                                                    error!("{}", &error_message);
                                                    gem.ui.toasts.error(&error_message);
                                                }
                                                Ok(new_playlist) => {
                                                    info!("Created and saved {} to {:?}", &new_playlist.name, &new_playlist.m3u_path);
                                                    gem.playlists.push(new_playlist);
                                                }
                                            }
                                        }
                                    },
                                );
                            });
                        })
                        .body(|body| {
                            body.rows(36.0, gem.playlists.len(), |mut row| {
                                let playlist = &mut gem.playlists[row.index()];

                                if let Some(playlist_key) = &gem.ui.playlists.selected_playlist_key {
                                    let playlist_is_selected = playlist.m3u_path == *playlist_key;
                                    row.set_selected(playlist_is_selected);
                                }

                                row.col(|ui| {
                                    ui.add_space(8.0);
                                    ui.add(unselectable_label(&playlist.name));
                                });

                                if row.response().clicked() {
                                    info!("Selected playlist: {}", playlist.name);
                                    gem.ui.playlists.selected_playlist_key = Some(playlist.m3u_path.clone());

                                    gem.ui.playlists.rename_buffer = None; // In case we were currently editing
                                    gem.ui.playlists.cache_dirty = true;
                                    gem.ui.playlists.selected_tracks.clear();
                                }
                            });
                        });
                });

                strip.cell(|ui| {
                    ui.add(Separator::default().vertical());
                });

                strip.cell(|ui| playlist(ui, gem));
            });
    });
}

fn delete_playlist_modal(ui: &mut Ui, gem: &mut GemPlayer) {
    debug_assert!(gem.ui.playlists.delete_modal_open);

    let Some(playlist_key) = gem.ui.playlists.selected_playlist_key.clone() else {
        error!("The delete playlist modal is open but no playlist is selected.");
        gem.ui.playlists.delete_modal_open = false;
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
                        if ui.button(("\t{}\t", ICON_CLOSE)).clicked() {
                            cancel_clicked = true;
                        }
                    },
                    |ui| {
                        if ui.button(("\t{}\t", ICON_CHECK)).clicked() {
                            confirm_clicked = true;

                            let result = delete(&playlist_key, &mut gem.playlists);
                            if let Err(e) = result {
                                error!("{}", e);
                            } else {
                                let message =
                                    "Playlist was deleted successfully. If this was a mistake, the m3u file can be found in the trash.";
                                info!("{}", message);
                                gem.ui.toasts.success(message);
                                gem.ui.playlists.selected_playlist_key = None;
                            }
                        }
                    },
                );
            });
        });

    if confirm_clicked || cancel_clicked || modal.should_close() {
        gem.ui.playlists.delete_modal_open = false;
    }
}

fn playlist(ui: &mut Ui, gem: &mut GemPlayer) {
    let Some(playlist_key) = gem.ui.playlists.selected_playlist_key.clone() else {
        return; // No playlist selected, do nothing
    };

    StripBuilder::new(ui)
        .size(Size::exact(64.0))
        .size(Size::remainder())
        .vertical(|mut strip| {
            strip.cell(|ui| {
                Frame::new().fill(ui.visuals().faint_bg_color).show(ui, |ui| {
                    if let Some(name_buffer) = &mut gem.ui.playlists.rename_buffer {
                        // Editing mode
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

                                let cancel_button = Button::new(ICON_CANCEL);
                                discard_clicked = ui.add(cancel_button).on_hover_text("Discard").clicked();

                                ui.add_space(8.0);

                                save_clicked = ui.add(Button::new(ICON_SAVE)).on_hover_text("Save").clicked();
                            },
                        );

                        if save_clicked {
                            let name_buffer_clone = name_buffer.to_owned();

                            let playlist = &mut gem.playlists.get_by_path_mut(&playlist_key);
                            let result = rename(playlist, name_buffer_clone);
                            match result {
                                Err(e) => {
                                    let message = format!("Error renaming playlist: {}", e);
                                    error!("{}", message);
                                    gem.ui.toasts.error(message);
                                }
                                Ok(_) => {
                                    // Update the selected playlist with the new path so that we remain selected.
                                    gem.ui.playlists.selected_playlist_key = Some(playlist.m3u_path.clone());
                                }
                            }

                            gem.ui.playlists.rename_buffer = None;
                        }

                        if discard_clicked {
                            gem.ui.playlists.rename_buffer = None;
                        }
                    } else {
                        // Not edit mode
                        let strip_contains_pointer = ui.rect_contains_pointer(ui.max_rect());
                        let mut play_clicked = false;
                        let mut delete_clicked = false;
                        let mut edit_clicked = false;

                        containers::Sides::new().height(ui.available_height()).show(
                            ui,
                            |ui| {
                                ui.add_space(16.0);

                                let name = &gem.playlists.get_by_path(&playlist_key).name;
                                ui.add(unselectable_label(RichText::new(name).heading().strong()));

                                if strip_contains_pointer {
                                    ui.add_space(16.0);

                                    let play = Button::new(ICON_PLAY_ARROW);
                                    play_clicked = ui.add(play).clicked();
                                }
                            },
                            |ui| {
                                if !strip_contains_pointer {
                                    return;
                                }

                                ui.add_space(16.0);

                                let delete_button = Button::new(ICON_DELETE);
                                delete_clicked = ui.add(delete_button).on_hover_text("Delete").clicked();

                                ui.add_space(8.0);

                                let edit_name_button = Button::new(ICON_EDIT);
                                edit_clicked = ui.add(edit_name_button).on_hover_text("Edit name").clicked();
                            },
                        );

                        // We have to do this pattern since we want to access gem across
                        // the two captures used by containers::Sides.
                        if play_clicked {
                            let track_keys = gem
                                .playlists
                                .get_by_path(&playlist_key)
                                .tracks
                                .iter()
                                .map(|t| t.path.clone())
                                .collect();

                            gem.commands.push(GemCommand::PlayTrackList {
                                track_keys,
                                start_at: None,
                            });
                        }

                        if delete_clicked {
                            info!("Opening delete playlist modal");
                            gem.ui.playlists.delete_modal_open = true;
                        }

                        if edit_clicked {
                            let playlist = &mut gem.playlists.get_by_path(&playlist_key);
                            info!("Editing playlist name: {}", playlist.name);
                            gem.ui.playlists.rename_buffer = Some(playlist.name.clone());
                        }
                    }
                });
            });

            strip.cell(|ui| playlist_tracks(ui, gem));
        });
}

fn playlist_tracks(ui: &mut Ui, gem: &mut GemPlayer) {
    ui.scope(|ui| {
        let Some(playlist_key) = gem.ui.playlists.selected_playlist_key.clone() else {
            centered_frame(ui, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add(unselectable_label("No playlist selected"));
                });
            });

            return;
        };

        let playlist_length = gem.playlists.get_by_path(&playlist_key).tracks.len();
        if playlist_length == 0 {
            centered_frame(ui, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add(unselectable_label("The playlist is empty."));
                });
            });

            return;
        }

        if gem.ui.playlists.cache_dirty {
            gem.ui.playlists.cached_playlist_tracks = filter(&gem.playlists.get_by_path(&playlist_key).tracks, &gem.ui.search);
            gem.ui.library.cache_dirty = false;
        }

        let header_labels = [ICON_TAG, ICON_MUSIC_NOTE, ICON_ARTIST, ICON_ALBUM, ICON_HOURGLASS];

        let available_width = ui.available_width();
        let position_width = 64.0;
        let time_width = 64.0;
        let more_width = 48.0;
        let remaining_width = available_width - position_width - time_width - more_width;
        let title_width = remaining_width * (2.0 / 4.0);
        let artist_width = remaining_width * (1.0 / 4.0);
        let album_width = remaining_width * (1.0 / 4.0);

        ui.spacing_mut().item_spacing.x = 0.0; // See comment in library_view() as to why we do this.

        // Used to determine if selection should be extended.
        let shift_is_pressed = ui.input(|i| i.modifiers.shift);

        let mut maybe_command = None;

        let playing_color = ui.visuals().selection.bg_fill;

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
                body.rows(26.0, gem.ui.playlists.cached_playlist_tracks.len(), |mut row| {
                    let index = row.index();

                    let track = &gem.ui.playlists.cached_playlist_tracks[index];
                    let track_key = track.path.clone();

                    let track_is_playing = gem.player.playing.as_ref().is_some_and(|t| t == track);

                    let track_is_selected = gem.ui.playlists.selected_tracks.contains(&track.path);
                    row.set_selected(track_is_selected);

                    let text_color = if track_is_playing && !track_is_selected {
                        Some(playing_color)
                    } else {
                        None
                    };

                    row.col(|ui| {
                        ui.add_space(16.0);
                        let label = table_label((index + 1).to_string(), text_color);
                        ui.add(label);
                    });

                    row.col(|ui| {
                        ui.add_space(4.0);
                        let label = table_label(track.title.as_deref().unwrap_or("-"), text_color);
                        ui.add(label);
                    });

                    row.col(|ui| {
                        ui.add_space(4.0);
                        let label = table_label(track.artist.as_deref().unwrap_or("-"), text_color);
                        ui.add(label);
                    });

                    row.col(|ui| {
                        ui.add_space(4.0);
                        let label = table_label(track.album.as_deref().unwrap_or("-"), text_color);
                        ui.add(label);
                    });

                    row.col(|ui| {
                        ui.add_space(4.0);
                        let duration_string = format_duration_to_mmss(track.duration);
                        let label = table_label(duration_string, text_color);
                        ui.add(label);
                    });

                    let rest_of_row_is_hovered = row.response().hovered();
                    let mut more_cell_contains_pointer = false;
                    row.col(|ui| {
                        ui.add_space(8.0);

                        more_cell_contains_pointer = ui.rect_contains_pointer(ui.max_rect());
                        let should_show_more_button = rest_of_row_is_hovered || more_cell_contains_pointer || track_is_selected;

                        if should_show_more_button {
                            let response = ui.add(Button::new(ICON_MORE_HORIZ)).on_hover_text("More");

                            if response.clicked() {
                                let selected_tracks = &mut gem.ui.playlists.selected_tracks;

                                if selected_tracks.is_empty() || !selected_tracks.contains(&track_key) {
                                    selected_tracks.clear();
                                    selected_tracks.push(track_key.clone());
                                }
                            }

                            Popup::menu(&response).show(|ui| {
                                if let Some(action) = playlist_context_menu(ui, gem) {
                                    maybe_command = Some(action);
                                }
                            });
                        } else if track_is_playing {
                            playing_indicator(ui);
                        }
                    });

                    let response = row.response();

                    if response.clicked() || response.double_clicked() || response.secondary_clicked() {
                        let selected_tracks = &mut gem.ui.playlists.selected_tracks;

                        if response.secondary_clicked() {
                            if selected_tracks.is_empty() || !track_is_selected {
                                selected_tracks.clear();
                                selected_tracks.push(track_key.clone());
                            }
                        } else if shift_is_pressed && !selected_tracks.is_empty() {
                            let last_selected_track = selected_tracks.last().unwrap();
                            let last_index = gem
                                .ui
                                .playlists
                                .cached_playlist_tracks
                                .iter()
                                .position(|t| &t.path == last_selected_track)
                                .unwrap();

                            let start = last_index.min(index);
                            let end = last_index.max(index);
                            for t in &gem.ui.playlists.cached_playlist_tracks[start..=end] {
                                if !selected_tracks.contains(&t.path) {
                                    selected_tracks.push(t.path.clone());
                                }
                            }
                        } else {
                            selected_tracks.clear();
                            selected_tracks.push(track_key.clone());
                        }
                    }

                    if response.double_clicked() {
                        let track_keys = gem.ui.playlists.cached_playlist_tracks.iter().map(|t| t.path.clone()).collect();
                        maybe_command = Some(GemCommand::PlayTrackList {
                            track_keys,
                            start_at: Some(track_key.clone()),
                        });
                    }

                    Popup::context_menu(&response).show(|ui| {
                        if let Some(command) = playlist_context_menu(ui, gem) {
                            maybe_command = Some(command);
                        }
                    });
                });
            });

        if let Some(command) = maybe_command {
            gem.commands.push(command);
        }
    });
}

fn playlist_context_menu(ui: &mut Ui, gem: &GemPlayer) -> Option<GemCommand> {
    let playlist_key = gem
        .ui
        .playlists
        .selected_playlist_key
        .as_ref()
        .expect("The selected playlist should be set before the playlist context menu is opened.");

    let track_keys = &gem.ui.playlists.selected_tracks;

    let modal_width = 220.0;
    ui.set_width(modal_width);

    ui.add_enabled(false, Label::new(format!("{} track(s) selected", track_keys.len())));

    ui.separator();

    let mut command = None;

    if ui.button(("Remove from Playlist", ICON_DELETE)).clicked() {
        command = Some(GemCommand::RemoveTracksFromPlaylist {
            playlist_key: playlist_key.clone(),
            track_keys: track_keys.clone(),
        });
    }

    ui.separator();

    if ui.button(("Play Next", ICON_PLAY_ARROW)).clicked() {
        command = Some(GemCommand::EnqueueTracksNext {
            track_keys: track_keys.clone(),
        });
    }

    if ui.button(("Add to Queue", ICON_ADD)).clicked() {
        command = Some(GemCommand::EnqueueTracks {
            track_keys: track_keys.clone(),
        });
    }

    ui.separator();

    if ui.button(("Open File Location", ICON_FOLDER)).clicked() {
        if let Some(first) = track_keys.first() {
            command = Some(GemCommand::OpenTrackLocation(first.clone()));
        } else {
            error!("No track(s) selected for opening file location");
        }
    }

    command
}
