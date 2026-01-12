use std::path::PathBuf;

use egui::{Align, Button, Frame, Label, Layout, Margin, Popup, RichText, ScrollArea, Sense, Ui};
use egui_extras::TableBuilder;
use egui_material_icons::icons;
use fully_pub::fully_pub;
use log::{error, info};

use crate::{
    format_duration_to_mmss, play_library,
    player::{enqueue, enqueue_next},
    playlist::{add_to_playlist, Playlist, PlaylistRetrieval},
    track::{open_file_location, sort, SortBy, SortOrder, Track, TrackRetrieval},
    ui::root::{playing_indicator, table_label, unselectable_label},
    GemPlayer,
};

#[fully_pub]
struct LibraryViewState {
    selected_tracks: Vec<PathBuf>,
    cached_library: Option<Vec<Track>>,

    sort_by: SortBy,
    sort_order: SortOrder,
}

pub fn library_view(ui: &mut Ui, gem: &mut GemPlayer) {
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

    if gem.library.is_empty() {
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

    let cached_library = gem.ui.library.cached_library.get_or_insert_with(|| {
        // Regenerate the cache.

        let mut filtered_and_sorted: Vec<Track> = gem
            .library
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

        sort(&mut filtered_and_sorted, gem.ui.library.sort_by, gem.ui.library.sort_order);

        filtered_and_sorted
    });

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

    // Used to determine if selection should be extended.
    let shift_is_pressed = ui.input(|i| i.modifiers.shift);

    let mut should_play_library = None;
    let mut context_menu_action = None;

    let playing_color = ui.visuals().selection.bg_fill;

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
            body.rows(26.0, cached_library.len(), |mut row| {
                let track = &cached_library[row.index()];
                let track_is_playing = gem.player.playing.as_ref().is_some_and(|t| t == track);

                let row_is_selected = gem.ui.library.selected_tracks.contains(&track.path);
                row.set_selected(row_is_selected);

                let text_color = if track_is_playing && !row_is_selected {
                    Some(playing_color)
                } else {
                    None
                };

                row.col(|ui| {
                    ui.add_space(16.0);
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
                    let should_show_more_button: bool = rest_of_row_is_hovered || more_cell_contains_pointer || row_is_selected;

                    if should_show_more_button {
                        let more_button = Button::new(icons::ICON_MORE_HORIZ);
                        let response = ui.add(more_button).on_hover_text("More");

                        if response.clicked() {
                            gem.ui.library.selected_tracks.push(track.path.clone());
                        }

                        Popup::menu(&response).show(|ui| {
                            let selected_tracks_count = gem.ui.library.selected_tracks.len();
                            let maybe_action = library_context_menu_ui(ui, selected_tracks_count, &gem.playlists);
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
                let already_selected = gem.ui.library.selected_tracks.contains(&track.path);

                if primary_clicked || secondary_clicked {
                    let selected_tracks = &mut gem.ui.library.selected_tracks;

                    if secondary_clicked {
                        if selected_tracks.is_empty() || !already_selected {
                            selected_tracks.clear();
                            selected_tracks.push(track.path.clone());
                        }
                    } else if shift_is_pressed && !selected_tracks.is_empty() {
                        let last_selected = selected_tracks.last().unwrap();
                        let last_index = cached_library.iter().position(|t| &t.path == last_selected).unwrap();
                        let clicked_index = cached_library.iter().position(|t| t.path == track.path).unwrap();

                        let start = last_index.min(clicked_index);
                        let end = last_index.max(clicked_index);
                        for t in &cached_library[start..=end] {
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
                    should_play_library = Some(track.clone());
                }

                Popup::context_menu(&response).show(|ui| {
                    let selected_tracks_count = gem.ui.library.selected_tracks.len();
                    let maybe_action = library_context_menu_ui(ui, selected_tracks_count, &gem.playlists);
                    if let Some(action) = maybe_action {
                        context_menu_action = Some(action);
                    }
                });
            });
        });

    // Perform actions AFTER rendering the table to avoid borrow checker issues that come with mutating state inside closures.

    if let Some(track) = should_play_library {
        if let Err(e) = play_library(gem, Some(&track)) {
            error!("{}", e);
            gem.ui.toasts.error("Error playing from library");
        }
    }

    if let Some(action) = context_menu_action {
        handle_library_context_menu_action(gem, action);
    }
}

#[derive(Debug)]
enum LibraryContextMenuAction {
    AddToPlaylist(PathBuf),
    EnqueueNext,
    Enqueue,
    OpenFileLocation,
}

fn handle_library_context_menu_action(gem: &mut GemPlayer, action: LibraryContextMenuAction) {
    match action {
        LibraryContextMenuAction::AddToPlaylist(playlist_key) => {
            if gem.ui.library.selected_tracks.is_empty() {
                error!("No track(s) selected for adding to playlist.");
                return;
            }

            let playlist = gem.playlists.get_by_path_mut(&playlist_key);

            let mut added_count = 0;
            for track_key in &gem.ui.library.selected_tracks {
                let track = gem.library.get_by_path(track_key);
                if let Err(e) = add_to_playlist(playlist, track.clone()) {
                    error!("Failed to add track to playlist: {}", e);
                } else {
                    added_count += 1;
                }
            }

            gem.ui.playlists.cached_playlist_tracks = None;

            if added_count > 0 {
                let message = format!("Added {} track(s) to playlist '{}'.", added_count, playlist.name);
                info!("{}", message);
                gem.ui.toasts.success(message);
            } else {
                gem.ui.toasts.error("No tracks were added.");
            }
        }
        LibraryContextMenuAction::EnqueueNext => {
            if gem.ui.library.selected_tracks.is_empty() {
                error!("No track(s) selected for enqueue next");
                return;
            }

            for track_key in &gem.ui.library.selected_tracks {
                let track = gem.library.get_by_path(track_key);
                enqueue_next(&mut gem.player, track.clone());
            }
        }
        LibraryContextMenuAction::Enqueue => {
            if gem.ui.library.selected_tracks.is_empty() {
                error!("No track(s) selected for enqueue");
                return;
            }

            for track_key in &gem.ui.library.selected_tracks {
                let track = gem.library.get_by_path(track_key);
                enqueue(&mut gem.player, track.clone());
            }
        }
        LibraryContextMenuAction::OpenFileLocation => {
            // We just grab the first since we cannot reveal multiple file locations.
            let Some(first_track_key) = gem.ui.library.selected_tracks.first() else {
                error!("No track(s) were selected for opening file location.");
                return;
            };

            let first_track = gem.library.get_by_path(first_track_key);
            if let Err(e) = open_file_location(first_track) {
                error!("Failed to open track location: {}", e);
            } else {
                info!("Opening track location: {}", first_track.path.display());
            }
        }
    }
}

fn library_context_menu_ui(ui: &mut Ui, selected_tracks_count: usize, playlists: &[Playlist]) -> Option<LibraryContextMenuAction> {
    let modal_width = 220.0;
    ui.set_width(modal_width);

    ui.add_enabled(false, Label::new(format!("{} track(s) selected", selected_tracks_count)));

    ui.separator();

    let mut action = None;

    let add_to_playlists_enabled = !playlists.is_empty();
    ui.add_enabled_ui(add_to_playlists_enabled, |ui| {
        ui.menu_button("Add to Playlist", |ui| {
            ui.set_min_width(modal_width);

            ScrollArea::vertical().max_height(164.0).show(ui, |ui| {
                for playlist in playlists.iter() {
                    let response = ui.button(&playlist.name);
                    if response.clicked() {
                        action = Some(LibraryContextMenuAction::AddToPlaylist(playlist.m3u_path.clone()));
                    }
                }
            });
        });
    });

    ui.separator();

    let response = ui.button(format!("Play Next {}", icons::ICON_PLAY_ARROW));
    if response.clicked() {
        action = Some(LibraryContextMenuAction::EnqueueNext);
    }

    let response = ui.button(format!("Add to Queue {}", icons::ICON_QUEUE_MUSIC));
    if response.clicked() {
        action = Some(LibraryContextMenuAction::Enqueue);
    }

    ui.separator();

    let response = ui.button(format!("Open File Location {}", icons::ICON_FOLDER));
    if response.clicked() {
        action = Some(LibraryContextMenuAction::OpenFileLocation);
    }

    action
}
