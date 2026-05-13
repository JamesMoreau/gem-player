use std::{
    fs::copy,
    path::{Path, PathBuf},
};

use egui::{Align, Button, Label, Layout, Popup, RichText, ScrollArea, Sense, Ui};
use egui_extras::TableBuilder;
use egui_material_icons::icons::{
    ICON_ALBUM, ICON_ARTIST, ICON_FOLDER, ICON_HOURGLASS, ICON_MORE_HORIZ, ICON_MUSIC_NOTE, ICON_PLAY_ARROW, ICON_QUEUE_MUSIC,
};
use fully_pub::fully_pub;
use log::{error, info};

use crate::{
    GemPlayer,
    commands::GemCommand,
    resources::resource_path,
    track::{SortBy, SortOrder, Track, filter, sort},
    ui::{
        root::{format_duration_to_mmss, playing_indicator, table_label, unselectable_label},
        widgets::centered_frame::centered_frame,
    },
};

#[fully_pub]
struct LibraryViewState {
    selected_tracks: Vec<PathBuf>,

    cached_library: Vec<Track>,
    cache_dirty: bool,

    sort_by: SortBy,
    sort_order: SortOrder,
}

pub fn library_view(ui: &mut Ui, gem: &mut GemPlayer) {
    ui.scope(|ui| {
        let Some(library_directory) = &gem.library_directory else {
            centered_frame(ui, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add(unselectable_label(
                        "No library directory set. Add your music folder in the settings.",
                    ));
                });
            });

            return;
        };

        if gem.library.is_empty() {
            centered_frame(ui, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add(unselectable_label("The library is empty."));
                    ui.add_space(16.0);

                    if ui.button("Add a sample track").clicked() {
                        add_sample_track_to_library(library_directory);
                    }
                });
            });

            return;
        }

        if gem.ui.library.cache_dirty {
            gem.ui.library.cached_library = filter(&gem.library, &gem.ui.search);
            sort(
                &mut gem.ui.library.cached_library,
                gem.ui.library.sort_by,
                gem.ui.library.sort_order,
            );
            gem.ui.library.cache_dirty = false;
        }

        let header_labels = [ICON_MUSIC_NOTE, ICON_ARTIST, ICON_ALBUM, ICON_HOURGLASS];

        let time_width = 64.0;
        let more_width = 48.0;

        let available_width = ui.available_width();
        let remaining_width = available_width - time_width - more_width;

        let title_width = remaining_width * (1.0 / 2.0);
        let artist_width = remaining_width * (1.0 / 4.0);
        let album_width = remaining_width * (1.0 / 4.0);

        // Since we are setting the widths of the table columns manually by dividing up the available width,
        // if we leave the default item spacing, the width taken up by the table will be greater than the available width,
        // causing the right side of the table to be cut off by the window.
        ui.spacing_mut().item_spacing.x = 0.0;

        // Used to determine if selection should be extended.
        let shift_is_pressed = ui.input(|i| i.modifiers.shift);

        let mut maybe_command = None;

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
                body.rows(26.0, gem.ui.library.cached_library.len(), |mut row| {
                    let track = &gem.ui.library.cached_library[row.index()];
                    let track_key = track.path.clone();

                    let track_is_playing = gem.player.playing.as_ref().is_some_and(|t| t == track);

                    let track_is_selected = gem.ui.library.selected_tracks.contains(&track_key);
                    row.set_selected(track_is_selected);

                    let text_color = if track_is_playing && !track_is_selected {
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
                        let should_show_more_button = rest_of_row_is_hovered || more_cell_contains_pointer || track_is_selected;

                        if should_show_more_button {
                            let more_button = Button::new(ICON_MORE_HORIZ);
                            let response = ui.add(more_button).on_hover_text("More");

                            if response.clicked() {
                                let selected_tracks = &mut gem.ui.library.selected_tracks;

                                if selected_tracks.is_empty() || !selected_tracks.contains(&track_key) {
                                    selected_tracks.clear();
                                    selected_tracks.push(track_key.clone());
                                }
                            }

                            Popup::menu(&response).show(|ui| {
                                if let Some(command) = library_context_menu(ui, gem) {
                                    maybe_command = Some(command);
                                }
                            });
                        } else if track_is_playing {
                            playing_indicator(ui);
                        }
                    });

                    let response = row.response();

                    if response.clicked() || response.double_clicked() || response.secondary_clicked() {
                        let selected_tracks = &mut gem.ui.library.selected_tracks;

                        if response.secondary_clicked() {
                            if selected_tracks.is_empty() || !track_is_selected {
                                selected_tracks.clear();
                                selected_tracks.push(track_key.clone());
                            }
                        } else if shift_is_pressed && !selected_tracks.is_empty() {
                            let last_selected = selected_tracks.last().unwrap();

                            let last_index = gem.ui.library.cached_library.iter().position(|t| &t.path == last_selected).unwrap();

                            let start = last_index.min(row.index());
                            let end = last_index.max(row.index());

                            for t in &gem.ui.library.cached_library[start..=end] {
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
                        let track_keys = gem.ui.library.cached_library.iter().map(|t| t.path.clone()).collect();
                        maybe_command = Some(GemCommand::PlayTrackList {
                            track_keys,
                            start_at: Some(track_key.clone()),
                        });
                    }

                    Popup::context_menu(&response).show(|ui| {
                        if let Some(command) = library_context_menu(ui, gem) {
                            maybe_command = Some(command);
                        }
                    });
                });
            });

        // Queue commands AFTER rendering the table to avoid borrow checker issues that come with mutating state inside closures.
        if let Some(command) = maybe_command {
            gem.commands.push(command);
        }
    });
}

fn library_context_menu(ui: &mut Ui, gem: &GemPlayer) -> Option<GemCommand> {
    let mut maybe_command = None;

    let modal_width = 220.0;
    ui.set_width(modal_width);

    ui.add_enabled(
        false,
        Label::new(format!("{} track(s) selected", gem.ui.library.selected_tracks.len())),
    );

    ui.separator();

    let add_to_playlists_enabled = !gem.playlists.is_empty();
    ui.add_enabled_ui(add_to_playlists_enabled, |ui| {
        ui.menu_button("Add to Playlist", |ui| {
            ui.set_min_width(modal_width);

            ScrollArea::vertical().max_height(164.0).show(ui, |ui| {
                for playlist in &gem.playlists {
                    let response = ui.button(&playlist.name);
                    if response.clicked() {
                        maybe_command = Some(GemCommand::AddTracksToPlaylist {
                            playlist_key: playlist.m3u_path.clone(),
                            track_keys: gem.ui.library.selected_tracks.clone(),
                        });
                    }
                }
            });
        });
    });

    ui.separator();

    let response = ui.button(("Play Next", ICON_PLAY_ARROW));
    if response.clicked() {
        maybe_command = Some(GemCommand::EnqueueTracksNext {
            track_keys: gem.ui.library.selected_tracks.clone(),
        });
    }

    let response = ui.button(("Add to Queue", ICON_QUEUE_MUSIC));
    if response.clicked() {
        maybe_command = Some(GemCommand::EnqueueTracks {
            track_keys: gem.ui.library.selected_tracks.clone(),
        });
    }

    ui.separator();

    let response = ui.button(("Open File Location", ICON_FOLDER));
    if response.clicked()
        && let Some(track_path) = gem.ui.library.selected_tracks.first()
    {
        maybe_command = Some(GemCommand::OpenTrackLocation(track_path.clone()));
    }

    maybe_command
}

const SAMPLE_TRACK_NAME: &str = "clair_de_lune.mp3";

fn add_sample_track_to_library(library_dir: &Path) {
    let resource_path = resource_path(SAMPLE_TRACK_NAME);
    let dest_path = library_dir.join(SAMPLE_TRACK_NAME);

    if copy(&resource_path, &dest_path).is_err() {
        error!("Failed to copy sample track to library directory.");
        return;
    }

    info!("Added sample track.");
}
