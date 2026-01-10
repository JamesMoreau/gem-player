use eframe::egui::{Align, Button, Frame, Layout, Margin, RichText, Sense, Ui};
use egui_extras::TableBuilder;
use egui_material_icons::icons;

use crate::{
    format_duration_to_mmss,
    player::{move_to_position, remove_from_queue, Player},
    ui::root::unselectable_label,
};

pub fn queue_view(ui: &mut Ui, player: &mut Player) {
    if player.queue.is_empty() {
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
    let time_width = 64.0;
    let actions_width = 80.0;
    let remaining_width = available_width - position_width - time_width - actions_width;
    let title_width = remaining_width * (2.0 / 4.0);
    let artist_width = remaining_width * (1.0 / 4.0);
    let album_width = remaining_width * (1.0 / 4.0);

    ui.spacing_mut().item_spacing.x = 0.0; // See comment in library_view() for why we set item_spacing to 0.

    // We only operate on the queue after we are done iterating over it.
    let mut to_be_removed = None;
    let mut to_be_moved_to_front = None;

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
            body.rows(26.0, player.queue.len(), |mut row| {
                let index = row.index();
                let track = &player.queue[index];

                row.col(|ui| {
                    ui.add_space(16.0);
                    ui.add(unselectable_label(format!("{}", index + 1)));
                });

                row.col(|ui| {
                    ui.add_space(4.0);
                    ui.add(unselectable_label(track.title.as_deref().unwrap_or("-")));
                });

                row.col(|ui| {
                    ui.add_space(4.0);
                    ui.add(unselectable_label(track.artist.as_deref().unwrap_or("-")));
                });

                row.col(|ui| {
                    ui.add_space(4.0);
                    ui.add(unselectable_label(track.album.as_deref().unwrap_or("-")));
                });

                row.col(|ui| {
                    ui.add_space(4.0);
                    let duration_string = format_duration_to_mmss(track.duration);
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
                        to_be_moved_to_front = Some(index);
                    }

                    ui.add_space(8.0);

                    let response = ui.add_visible(should_show_action_buttons, Button::new(icons::ICON_CLOSE));
                    if response.clicked() {
                        to_be_removed = Some(index);
                    }
                });
            });
        });

    if let Some(index) = to_be_removed {
        remove_from_queue(player, index);
    }

    if let Some(index) = to_be_moved_to_front {
        move_to_position(player, index, 0);
    }
}
