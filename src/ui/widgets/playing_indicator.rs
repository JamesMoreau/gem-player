use egui::Ui;

use crate::ui::widgets::bar_display::BarDisplay;

pub fn playing_indicator_ui(ui: &mut Ui) {
    let time = ui.input(|i| i.time) as f32;

    let values = [
        ((time * 6.0).sin() * 0.4 + 0.6).max(0.2),
        ((time * 7.5).cos() * 0.4 + 0.6).max(0.2),
        ((time * 5.3).sin() * 0.4 + 0.6).max(0.2),
    ];

    ui.add(BarDisplay::new(
        &values,
        ui.available_height() * 0.4,
        5.0,
        1.0,
        ui.visuals().selection.bg_fill,
    ));
}
