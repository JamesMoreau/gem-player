use egui::Ui;
use egui_material_icons::icons;

use crate::ui::{root::unselectable_label, widgets::centered_frame::centered_frame_ui};

pub fn file_drop_overlay(ui: &mut Ui) {
    centered_frame_ui(ui, |ui| {
        ui.vertical_centered(|ui| {
            ui.add(unselectable_label(format!(
                "Drop tracks here to add them to your library. {}",
                icons::ICON_DOWNLOAD
            )));
        });
    });
}
