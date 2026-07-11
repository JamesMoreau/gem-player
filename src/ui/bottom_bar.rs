use egui::{Align, Button, Direction, Frame, Layout, Margin, Popup, PopupCloseBehavior, TextEdit, Ui};
use egui_material_icons::icons::{ICON_CLEAR, ICON_CLEAR_ALL, ICON_FILTER_LIST, ICON_SEARCH};
use log::info;
use strum::IntoEnumIterator;

use crate::{
    GemPlayer,
    player::clear_the_queue,
    playlist::PlaylistRetrieval,
    track::{SortBy, SortOrder, Track, calculate_total_duration},
    ui::root::{View, format_duration_to_hhmmss, unselectable_label},
};

pub fn bottom_bar(ui: &mut Ui, gem: &mut GemPlayer) {
    Frame::new().inner_margin(Margin::symmetric(16, 0)).show(ui, |ui| {
        ui.columns_const(|[left, center, right]| {
            left.with_layout(Layout::left_to_right(Align::Center), |ui| {
                if let Some(view) = view_selector(ui, gem.ui.current_view) {
                    info!("Switching to view: {:?}", view);
                    gem.ui.current_view = view;
                }
            });

            center.with_layout(Layout::centered_and_justified(Direction::TopDown), |ui| {
                if let Some(text) = get_status(gem) {
                    ui.add(unselectable_label(text));
                }
            });

            right.with_layout(Layout::right_to_left(Align::Center), |ui| {
                controls(ui, gem);
            });
        });
    });
}

fn view_selector(ui: &mut Ui, current_view: View) -> Option<View> {
    let mut selected = None;

    for view in View::iter() {
        if ui
            .selectable_label(current_view == view, format!("  {}  ", view.icon()))
            .on_hover_text(format!("{:?}", view))
            .clicked()
        {
            selected = Some(view);
        }

        ui.add_space(4.0);
    }

    selected
}

fn get_status(gem: &GemPlayer) -> Option<String> {
    match gem.ui.current_view {
        View::Library => Some(get_count_and_duration_string_from_tracks(&gem.library)),
        View::Queue => Some(get_count_and_duration_string_from_tracks(&gem.player.queue)),
        View::Playlists => {
            let playlist_key = gem.ui.playlists.selected_playlist_key.as_ref()?;
            let playlist = gem.playlists.get_by_path(playlist_key);

            Some(get_count_and_duration_string_from_tracks(&playlist.tracks))
        }
        View::Settings => None,
    }
}

fn controls(ui: &mut Ui, gem: &mut GemPlayer) {
    match gem.ui.current_view {
        View::Library => {
            let search_was_changed = search(ui, &mut gem.ui.search);
            if search_was_changed {
                // We reset both caches since there is only one search text state variable.
                gem.ui.library.cache_dirty = true;
                gem.ui.library.selected_tracks.clear();
                gem.ui.playlists.cache_dirty = true;
                gem.ui.playlists.selected_tracks.clear();
            }

            let sort_was_changed = sort_and_order_by(ui, &mut gem.ui.library.sort_by, &mut gem.ui.library.sort_order);
            if sort_was_changed {
                gem.ui.library.cache_dirty = true;
            }
        }
        View::Queue => {
            let queue_is_not_empty = !gem.player.queue.is_empty();

            let clear_button = Button::new(ICON_CLEAR_ALL);
            let response = ui
                .add_enabled(queue_is_not_empty, clear_button)
                .on_hover_text("Clear")
                .on_disabled_hover_text("Queue is empty");
            if response.clicked() {
                clear_the_queue(&mut gem.player);
            }
        }
        View::Playlists => {
            let search_changed = search(ui, &mut gem.ui.search);
            if search_changed {
                // Same as above.
                gem.ui.library.cache_dirty = true;
                gem.ui.library.selected_tracks.clear();
                gem.ui.playlists.cache_dirty = true;
                gem.ui.playlists.selected_tracks.clear();
            }
        }
        _ => {}
    }
}

fn sort_and_order_by(ui: &mut Ui, sort_by: &mut SortBy, sort_order: &mut SortOrder) -> bool {
    let response = ui.button(ICON_FILTER_LIST).on_hover_text("Sort by and order");

    let mut sort_by_changed = false;
    let mut sort_order_changed = false;

    Popup::menu(&response)
        .gap(4.0)
        .close_behavior(PopupCloseBehavior::CloseOnClickOutside)
        .show(|ui| {
            for sb in SortBy::iter() {
                sort_by_changed |= ui.radio_value(sort_by, sb, sb.label()).changed();
            }
            ui.separator();
            for so in SortOrder::iter() {
                sort_order_changed |= ui.radio_value(sort_order, so, format!("{:?}", so)).changed();
            }
        });

    sort_by_changed || sort_order_changed
}

fn search(ui: &mut Ui, search_text: &mut String) -> bool {
    let mut changed = false;
    let clear_button_is_visible = !search_text.is_empty();
    let response = ui
        .add_visible(clear_button_is_visible, Button::new(ICON_CLEAR))
        .on_hover_text("Clear search");
    if response.clicked() {
        search_text.clear();
        changed = true;
    }

    let search_bar = TextEdit::singleline(search_text)
        .prefix(ICON_SEARCH)
        .hint_text("Search ...")
        .desired_width(140.0)
        .char_limit(20);

    if ui.add(search_bar).changed() {
        changed = true;
    }

    changed
}

fn get_count_and_duration_string_from_tracks(tracks: &[Track]) -> String {
    let duration = calculate_total_duration(tracks);
    let duration_string = format_duration_to_hhmmss(duration);
    format!("{} tracks / {}", tracks.len(), duration_string)
}
