use egui::{epaint::MarginF32, Frame, Ui};

pub fn centered_frame_ui(ui: &mut Ui, add_contents: impl FnOnce(&mut Ui)) {
    let margin_factor = 1.0 / 4.0;

    Frame::new()
        .outer_margin(MarginF32::symmetric(
            ui.available_width() * margin_factor,
            ui.available_height() * margin_factor,
        ))
        .show(ui, |ui| {
            ui.vertical_centered(|ui| {
                add_contents(ui);
            });
        });
}
