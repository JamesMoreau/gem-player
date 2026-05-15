use egui::{Frame, Ui, epaint::MarginF32};

pub fn centered_frame(ui: &mut Ui, add_contents: impl FnOnce(&mut Ui)) {
    let margin = ui.available_width().min(ui.available_height()) * 0.25;

    Frame::new().outer_margin(MarginF32::same(margin)).show(ui, |ui| {
        ui.vertical_centered(add_contents);
    });
}
