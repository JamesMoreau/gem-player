use std::time::Duration;

use egui::{Align, Label, Layout, TextStyle, Ui};

pub struct Marquee {
    // TODO: direction left right.
    position: f32,
    speed: f32, // chars per second

    pause_timer: Duration,
    pause_duration: Duration,
}

impl Marquee {
    pub fn new() -> Self {
        Self {
            position: 0.0,
            speed: 5.0,
            pause_timer: Duration::from_secs(2),
            pause_duration: Duration::from_secs(2),
        }
    }

    pub fn pause_duration(mut self, duration: Duration) -> Self {
        self.pause_duration = duration;
        self
    }

    pub fn speed(mut self, chars_per_second: f32) -> Self {
        self.speed = chars_per_second;
        self
    }

    pub fn reset(&mut self) {
        self.position = 0.0;
        self.pause_timer = self.pause_duration;
    }
}

pub fn marquee_ui(ui: &mut Ui, marquee: &mut Marquee, text: &str) {
    let font_id = TextStyle::Body.resolve(ui.style());

    let chars_count = text.chars().count();
    let text_width: f32 = text.chars().map(|c| ui.ctx().fonts_mut(|r| r.glyph_width(&font_id, c))).sum();

    let average_char_width = text_width / chars_count as f32;

    let available_width = ui.available_width();
    let visible_chars = (available_width / average_char_width).floor() as usize;

    // If everything fits, no marquee needed
    if chars_count <= visible_chars {
        ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
            ui.add(Label::new(text).selectable(false).truncate());
        });
        return;
    }

    let dt = ui.input(|i| i.stable_dt);

    if marquee.pause_timer > Duration::ZERO {
        marquee.pause_timer = marquee.pause_timer.saturating_sub(Duration::from_secs_f32(dt));
    } else {
        marquee.position += marquee.speed * dt;

        if marquee.position >= chars_count as f32 {
            marquee.reset();
        }
    }

    let display_text: String = text
        .chars()
        .chain(text.chars())
        .skip(marquee.position.floor() as usize)
        .take(visible_chars)
        .collect();

    ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
        ui.add(Label::new(&display_text).selectable(false).truncate());
    });
}
