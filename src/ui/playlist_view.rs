use std::path::{Path, PathBuf};

use egui::{containers, Align, Button, Color32, Frame, Id, Label, Layout, Margin, Popup, RichText, Sense, Separator, TextEdit, Ui};
use egui_extras::{Size, StripBuilder, TableBuilder};
use egui_material_icons::icons;
use fully_pub::fully_pub;
use log::{error, info};

use crate::{
    format_duration_to_mmss,
    player::{clear_the_queue, enqueue, enqueue_next, play_next},
    playlist::{create, delete, remove_from_playlist, rename, PlaylistRetrieval},
    track::{open_file_location, Track, TrackRetrieval},
    ui::root::{playing_indicator, table_label, unselectable_label},
    GemPlayer,
};

#[fully_pub]
struct PlaylistsViewState {
    selected_playlist_key: Option<PathBuf>, // None: no playlist is selected. Some: the path of the selected playlist.
    selected_tracks: Vec<PathBuf>,

    cached_playlist_tracks: Option<Vec<Track>>,

    rename_buffer: Option<String>, // If Some, the playlist pointed to by selected_track's name is being edited and a buffer for the new name.
    delete_modal_open: bool,       // The menu is open for selected_playlist_path.
}

pub fn playlists_view(ui: &mut Ui, gem: &mut GemPlayer) {
    if gem.ui.library_and_playlists_are_loading {
        Frame::new()
            .outer_margin(Margin::symmetric((ui.available_width() * (1.0 / 4.0)) as i8, 32))
            .show(ui, |ui| {
                ui.vertical_centered(|ui| {
                    ui.spinner();
                });
            });

        return;
    }

    if gem.library_directory.is_none() {
        Frame::new()
            .outer_margin(Margin::symmetric((ui.available_width() * (1.0 / 4.0)) as i8, 32))
            .show(ui, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add(unselectable_label("Try adding your music directory in the settings"));
                });
            });

        return;
    };

    delete_playlist_modal(ui, gem);

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

                                    let add_button = Button::new(icons::ICON_ADD);
                                    let response = ui.add(add_button).on_hover_text("Add playlist");
                                    if response.clicked() {
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

                            let response = row.response();
                            if response.clicked() {
                                info!("Selected playlist: {}", playlist.name);
                                gem.ui.playlists.selected_playlist_key = Some(playlist.m3u_path.clone());

                                gem.ui.playlists.rename_buffer = None; // In case we were currently editing
                                gem.ui.playlists.cached_playlist_tracks = None;
                                gem.ui.playlists.selected_tracks.clear();
                            }
                        });
                    });
            });

            strip.cell(|ui| {
                ui.add(Separator::default().vertical());
            });

            strip.cell(|ui| playlist_ui(ui, gem));
        });
}

fn delete_playlist_modal(ui: &mut Ui, gem: &mut GemPlayer) {
    if !gem.ui.playlists.delete_modal_open {
        return;
    }

    let Some(playlist_key) = gem.ui.playlists.selected_playlist_key.clone() else {
        error!("The delete playlist is open but no playlist is selected.");
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

fn playlist_ui(ui: &mut Ui, gem: &mut GemPlayer) {
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

                                let cancel_button = Button::new(icons::ICON_CANCEL);
                                let response = ui.add(cancel_button).on_hover_text("Discard");
                                discard_clicked = response.clicked();

                                ui.add_space(8.0);

                                let confirm_button = Button::new(icons::ICON_SAVE);
                                let response = ui.add(confirm_button).on_hover_text("Save");
                                save_clicked = response.clicked();
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

                                    let play = Button::new(icons::ICON_PLAY_ARROW);
                                    let response = ui.add(play);
                                    play_clicked = response.clicked();
                                }
                            },
                            |ui| {
                                if !strip_contains_pointer {
                                    return;
                                }

                                ui.add_space(16.0);

                                let delete_button = Button::new(icons::ICON_DELETE);
                                let response = ui.add(delete_button).on_hover_text("Delete");
                                delete_clicked = response.clicked();

                                ui.add_space(8.0);

                                let edit_name_button = Button::new(icons::ICON_EDIT);
                                let response = ui.add(edit_name_button).on_hover_text("Edit name");
                                edit_clicked = response.clicked();
                            },
                        );

                        // We have to do this pattern since we want to access gem across
                        // the two captures used by containers::Sides.
                        if play_clicked {
                            let path = &gem.playlists.get_by_path(&playlist_key).m3u_path;
                            if let Err(e) = play_playlist(gem, &path.clone(), None) {
                                error!("{}", e);
                                gem.ui.toasts.error("Error playing from playlist");
                            }
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

            strip.cell(|ui| playlist_tracks_ui(ui, gem));
        });
}

fn playlist_tracks_ui(ui: &mut Ui, gem: &mut GemPlayer) {
    let Some(playlist_key) = gem.ui.playlists.selected_playlist_key.clone() else {
        Frame::new()
            .outer_margin(Margin::symmetric((ui.available_width() * (1.0 / 4.0)) as i8, 32))
            .show(ui, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add(unselectable_label("No playlist selected"));
                });
            });

        return;
    };

    let playlist_length = gem.playlists.get_by_path(&playlist_key).tracks.len();
    if playlist_length == 0 {
        Frame::new()
            .outer_margin(Margin::symmetric((ui.available_width() * (1.0 / 4.0)) as i8, 32))
            .show(ui, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add(unselectable_label("The playlist is empty."));
                });
            });

        return;
    }

    let cached_playlist_tracks = gem.ui.playlists.cached_playlist_tracks.get_or_insert_with(|| {
        // Regenerate the cache.

        let filtered: Vec<Track> = gem
            .playlists
            .get_by_path(&playlist_key)
            .tracks
            .iter()
            .filter(|track| {
                let search_lower = gem.ui.search.to_lowercase();

                let matches_search = |field: &Option<String>| {
                    field
                        .as_ref()
                        .map(|text| text.to_lowercase().contains(&search_lower))
                        .unwrap_or(false)
                };

                matches_search(&track.title) || matches_search(&track.artist) || matches_search(&track.album)
            })
            .cloned()
            .collect();

        filtered
    });

    let header_labels = [
        icons::ICON_TAG,
        icons::ICON_MUSIC_NOTE,
        icons::ICON_ARTIST,
        icons::ICON_ALBUM,
        icons::ICON_HOURGLASS,
    ];

    let available_width = ui.available_width();
    let position_width = 64.0;
    let time_width = 64.0;
    let more_width = 48.0;
    let remaining_width = available_width - position_width - time_width - more_width;
    let title_width = remaining_width * (2.0 / 4.0);
    let artist_width = remaining_width * (1.0 / 4.0);
    let album_width = remaining_width * (1.0 / 4.0);

    ui.spacing_mut().item_spacing.x = 0.0; // See comment in library_view() for why we set item_spacing to 0.

    // Used to determine if selection should be extended.
    let shift_is_pressed = ui.input(|i| i.modifiers.shift);

    let mut should_play_playlist = None;
    let mut context_menu_action = None;

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
            body.rows(26.0, cached_playlist_tracks.len(), |mut row| {
                let index = row.index();
                let track = &cached_playlist_tracks[index];
                let track_is_playing = gem.player.playing.as_ref().is_some_and(|t| t == track);

                let row_is_selected = gem.ui.playlists.selected_tracks.contains(&track.path);
                row.set_selected(row_is_selected);

                let text_color = if track_is_playing && !row_is_selected {
                    Some(playing_color)
                } else {
                    None
                };

                row.col(|ui| {
                    ui.add_space(16.0);
                    let label = table_label(format!("{}", index + 1), text_color);
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
                    let should_show_more_button = rest_of_row_is_hovered || more_cell_contains_pointer || row_is_selected;

                    if should_show_more_button {
                        let more_button = Button::new(icons::ICON_MORE_HORIZ);
                        let response = ui.add(more_button).on_hover_text("More");

                        Popup::menu(&response).show(|ui| {
                            let selected_tracks_count = gem.ui.playlists.selected_tracks.len();
                            let maybe_action = playlist_context_menu_ui(ui, selected_tracks_count);
                            if let Some(action) = maybe_action {
                                context_menu_action = Some(action);
                            }
                        });
                    } else if track_is_playing {
                        playing_indicator(ui);
                    }
                });

                let response = row.response();

                let secondary_clicked = response.secondary_clicked();
                let primary_clicked = response.clicked() || response.double_clicked();
                let already_selected = gem.ui.playlists.selected_tracks.contains(&track.path);

                if primary_clicked || secondary_clicked {
                    let selected_tracks = &mut gem.ui.playlists.selected_tracks;

                    if secondary_clicked {
                        if selected_tracks.is_empty() || !already_selected {
                            selected_tracks.clear();
                            selected_tracks.push(track.path.clone());
                        }
                    } else if shift_is_pressed && !selected_tracks.is_empty() {
                        let last_selected_track = selected_tracks.last().unwrap();
                        let last_index = cached_playlist_tracks.iter().position(|t| &t.path == last_selected_track).unwrap();
                        let clicked_index = cached_playlist_tracks.iter().position(|t| t.path == track.path).unwrap();

                        let start = last_index.min(clicked_index);
                        let end = last_index.max(clicked_index);
                        for t in &cached_playlist_tracks[start..=end] {
                            if !selected_tracks.contains(&t.path) {
                                selected_tracks.push(t.path.clone());
                            }
                        }
                    } else {
                        selected_tracks.clear();
                        selected_tracks.push(track.path.clone());
                    }
                }

                if response.double_clicked() {
                    should_play_playlist = Some((playlist_key.clone(), track.path.clone()));
                }

                Popup::context_menu(&response).show(|ui| {
                    let selected_tracks_count = gem.ui.playlists.selected_tracks.len();
                    let maybe_action = playlist_context_menu_ui(ui, selected_tracks_count);
                    if let Some(action) = maybe_action {
                        context_menu_action = Some(action);
                    }
                });
            });
        });

    if let Some(action) = context_menu_action {
        handle_playlist_context_menu_action(gem, action, &playlist_key);
    }

    if let Some((playlist_key, track_key)) = should_play_playlist {
        if let Err(e) = play_playlist(gem, &playlist_key, Some(&track_key)) {
            error!("{}", e);
            gem.ui.toasts.error("Error playing from playlist");
        }
    }
}

#[derive(Debug)]
enum PlaylistContextMenuAction {
    RemoveFromPlaylist,
    EnqueueNext,
    Enqueue,
    OpenFileLocation,
}

fn handle_playlist_context_menu_action(gem: &mut GemPlayer, action: PlaylistContextMenuAction, playlist_key: &Path) {
    match action {
        PlaylistContextMenuAction::RemoveFromPlaylist => {
            let Some(playlist_key) = &gem.ui.playlists.selected_playlist_key else {
                error!("No playlist selected for removing track from playlist.");
                return;
            };

            if gem.ui.playlists.selected_tracks.is_empty() {
                error!("No track(s) selected for removing track from playlist next.");
                return;
            };

            let playlist = gem.playlists.get_by_path_mut(playlist_key);

            let mut added_count = 0;
            for track_key in &gem.ui.playlists.selected_tracks {
                if let Err(e) = remove_from_playlist(playlist, track_key) {
                    error!("Failed to remove track from playlist: {}", e);
                } else {
                    added_count += 1;
                }
            }

            gem.ui.playlists.cached_playlist_tracks = None;

            if added_count > 0 {
                let message = format!("Removed {} track(s) from playlist '{}'", added_count, playlist.name);
                info!("{}", message);
                gem.ui.toasts.success(message);
            } else {
                gem.ui.toasts.error("No tracks were removed.");
            }
        }
        PlaylistContextMenuAction::EnqueueNext => {
            if gem.ui.playlists.selected_tracks.is_empty() {
                error!("No track(s) selected for enqueue next");
                return;
            };

            let playlist = gem.playlists.get_by_path(playlist_key);
            for track_key in &gem.ui.playlists.selected_tracks {
                let track = playlist.tracks.get_by_path(track_key);
                enqueue_next(&mut gem.player, track.clone());
            }
        }
        PlaylistContextMenuAction::Enqueue => {
            if gem.ui.playlists.selected_tracks.is_empty() {
                error!("No track(s) selected for enqueue");
                return;
            };

            let playlist = gem.playlists.get_by_path(playlist_key);
            for track_key in &gem.ui.playlists.selected_tracks {
                let track = playlist.tracks.get_by_path(track_key);
                enqueue(&mut gem.player, track.clone());
            }
        }
        PlaylistContextMenuAction::OpenFileLocation => {
            // We take the first one since we cannot open / reveal multiple tracks.
            let Some(first_track_key) = gem.ui.playlists.selected_tracks.first() else {
                error!("No track(s) selected for opening file location");
                return;
            };

            let playlist = gem.playlists.get_by_path(playlist_key);
            let first_track = playlist.tracks.get_by_path(first_track_key);
            if let Err(e) = open_file_location(first_track) {
                error!("Failed to open track location: {}", e);
            } else {
                info!("Opening track location: {}", first_track.path.display());
            }
        }
    }
}

fn playlist_context_menu_ui(ui: &mut Ui, selected_tracks_count: usize) -> Option<PlaylistContextMenuAction> {
    let modal_width = 220.0;
    ui.set_width(modal_width);

    ui.add_enabled(false, Label::new(format!("{} track(s) selected", selected_tracks_count)));

    ui.separator();

    let mut action = None;

    let response = ui.button(format!("{} Remove from Playlist", icons::ICON_DELETE));
    if response.clicked() {
        action = Some(PlaylistContextMenuAction::RemoveFromPlaylist);
    }

    ui.separator();

    let response = ui.button(format!("{} Play Next", icons::ICON_PLAY_ARROW));
    if response.clicked() {
        action = Some(PlaylistContextMenuAction::EnqueueNext);
    }

    let response = ui.button(format!("{} Add to Queue", icons::ICON_ADD));
    if response.clicked() {
        action = Some(PlaylistContextMenuAction::Enqueue);
    }

    ui.separator();

    let response = ui.button(format!("{} Open File Location", icons::ICON_FOLDER));
    if response.clicked() {
        action = Some(PlaylistContextMenuAction::OpenFileLocation);
    }

    action
}

pub fn play_playlist(gem: &mut GemPlayer, playlist_key: &Path, starting_track_key: Option<&Path>) -> Result<(), String> {
    clear_the_queue(&mut gem.player);

    let playlist = gem.playlists.get_by_path(playlist_key);

    let mut start_index = 0;
    if let Some(key) = starting_track_key {
        start_index = playlist.tracks.get_position_by_path(key);
    }

    // Add tracks from the starting index to the end, then from the beginning up to the starting index.
    for i in start_index..playlist.tracks.len() {
        gem.player.queue.push(playlist.tracks[i].clone());
    }
    for i in 0..start_index {
        gem.player.queue.push(playlist.tracks[i].clone());
    }

    play_next(&mut gem.player)?;

    Ok(())
}
