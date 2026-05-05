use egui::Ui;
use egui_material_icons::icons::ICON_DOWNLOAD;

use crate::ui::{root::unselectable_label, widgets::centered_frame::centered_frame};

pub fn file_drop_overlay(ui: &mut Ui) {
    centered_frame(ui, |ui| {
        ui.add(unselectable_label("Drop tracks here to add them to your library."));
        ui.add(unselectable_label(ICON_DOWNLOAD));
    });
}
