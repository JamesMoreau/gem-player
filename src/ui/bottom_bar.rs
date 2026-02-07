use egui::{Align, Button, Direction, Frame, Layout, Margin, Popup, PopupCloseBehavior, TextEdit, Ui};
use egui_material_icons::icons;
use log::info;
use strum::IntoEnumIterator;

use crate::{
    player::clear_the_queue,
    playlist::PlaylistRetrieval,
    track::{calculate_total_duration, sort_by_label, SortBy, SortOrder, Track},
    ui::root::{format_duration_to_hhmmss, unselectable_label, UIState, View},
    GemPlayer,
};

pub fn bottom_bar(ui: &mut Ui, gem: &mut GemPlayer) {
    Frame::new().inner_margin(Margin::symmetric(16, 0)).show(ui, |ui| {
        ui.columns_const(|[left, center, right]| {
            left.with_layout(Layout::left_to_right(Align::Center), |ui| {
                let get_icon = |view: &View| match view {
                    View::Library => icons::ICON_LIBRARY_MUSIC,
                    View::Queue => icons::ICON_QUEUE_MUSIC,
                    View::Playlists => icons::ICON_STAR,
                    View::Settings => icons::ICON_SETTINGS,
                };

                for view in View::iter() {
                    let icon = get_icon(&view);
                    let response = ui
                        .selectable_label(gem.ui.current_view == view, format!("  {icon}  "))
                        .on_hover_text(format!("{:?}", view));
                    if response.clicked() {
                        switch_view(&mut gem.ui, view);
                    }

                    ui.add_space(4.0);
                }
            });

            center.with_layout(Layout::centered_and_justified(Direction::TopDown), |ui| match gem.ui.current_view {
                View::Library => {
                    let tracks_count_and_duration = get_count_and_duration_string_from_tracks(&gem.library);
                    ui.add(unselectable_label(tracks_count_and_duration));
                }
                View::Queue => {
                    let tracks_count_and_duration = get_count_and_duration_string_from_tracks(&gem.player.queue);
                    ui.add(unselectable_label(tracks_count_and_duration));
                }
                View::Playlists => {
                    let Some(playlist_key) = &gem.ui.playlists.selected_playlist_key else {
                        return;
                    };

                    let playlist = gem.playlists.get_by_path(playlist_key);

                    let tracks_count_and_duration = get_count_and_duration_string_from_tracks(&playlist.tracks);
                    ui.add(unselectable_label(tracks_count_and_duration));
                }
                View::Settings => {}
            });

            right.with_layout(Layout::right_to_left(Align::Center), |ui| match gem.ui.current_view {
                View::Library => {
                    let search_was_changed = search_ui(ui, &mut gem.ui.search);
                    if search_was_changed {
                        // We reset both caches since there is only one search text state variable.
                        gem.ui.library.cached_library = None;
                        gem.ui.library.selected_tracks.clear();
                        gem.ui.playlists.cached_playlist_tracks = None;
                        gem.ui.playlists.selected_tracks.clear();
                    }

                    let sort_was_changed = sort_and_order_by_ui(ui, &mut gem.ui.library.sort_by, &mut gem.ui.library.sort_order);
                    if sort_was_changed {
                        gem.ui.library.cached_library = None;
                    }
                }
                View::Queue => {
                    let queue_is_not_empty = !gem.player.queue.is_empty();

                    let clear_button = Button::new(icons::ICON_CLEAR_ALL);
                    let response = ui
                        .add_enabled(queue_is_not_empty, clear_button)
                        .on_hover_text("Clear")
                        .on_disabled_hover_text("Queue is empty");
                    if response.clicked() {
                        clear_the_queue(&mut gem.player);
                    }
                }
                View::Playlists => {
                    let search_changed = search_ui(ui, &mut gem.ui.search);
                    if search_changed {
                        // Same as above.
                        gem.ui.library.cached_library = None;
                        gem.ui.library.selected_tracks.clear();
                        gem.ui.playlists.cached_playlist_tracks = None;
                        gem.ui.playlists.selected_tracks.clear();
                    }
                }
                _ => {}
            });
        });
    });
}

fn sort_and_order_by_ui(ui: &mut Ui, sort_by: &mut SortBy, sort_order: &mut SortOrder) -> bool {
    let response = ui.button(icons::ICON_FILTER_LIST).on_hover_text("Sort by and order");

    let mut sort_by_changed = false;
    let mut sort_order_changed = false;

    Popup::menu(&response)
        .gap(4.0)
        .close_behavior(PopupCloseBehavior::CloseOnClickOutside)
        .show(|ui| {
            for sb in SortBy::iter() {
                sort_by_changed |= ui.radio_value(sort_by, sb, sort_by_label(sb)).changed();
            }
            ui.separator();
            for so in SortOrder::iter() {
                sort_order_changed |= ui.radio_value(sort_order, so, format!("{:?}", so)).changed();
            }
        });

    sort_by_changed || sort_order_changed
}

fn search_ui(ui: &mut Ui, search_text: &mut String) -> bool {
    let mut changed = false;
    let clear_button_is_visible = !search_text.is_empty();
    let response = ui
        .add_visible(clear_button_is_visible, Button::new(icons::ICON_CLEAR))
        .on_hover_text("Clear search");
    if response.clicked() {
        search_text.clear();
        changed = true;
    }

    let search_bar = TextEdit::singleline(search_text)
        .hint_text(format!("{} Search ...", icons::ICON_SEARCH))
        .desired_width(140.0)
        .char_limit(20);

    let response = ui.add(search_bar);
    if response.changed() {
        changed = true;
    }

    changed
}

fn switch_view(ui: &mut UIState, view: View) {
    info!("Switching to view: {:?}", view);
    ui.current_view = view;
}

fn get_count_and_duration_string_from_tracks(tracks: &[Track]) -> String {
    let duration = calculate_total_duration(tracks);
    let duration_string = format_duration_to_hhmmss(duration);
    format!("{} tracks / {}", tracks.len(), duration_string)
}
