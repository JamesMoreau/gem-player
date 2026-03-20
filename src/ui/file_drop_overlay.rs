use egui::{epaint::MarginF32, Frame, Ui};
use egui_material_icons::icons;

use crate::ui::root::unselectable_label;

pub fn file_drop_overlay(ui: &mut Ui) {
    Frame::new()
        .outer_margin(MarginF32::symmetric(
            ui.available_width() * (1.0 / 4.0),
            ui.available_height() * (1.0 / 4.0),
        ))
        .show(ui, |ui| {
            ui.vertical_centered(|ui| {
                ui.add(unselectable_label(format!(
                    "Drop tracks here to add them to your library. {}",
                    icons::ICON_DOWNLOAD
                )));
            });
        });
}
