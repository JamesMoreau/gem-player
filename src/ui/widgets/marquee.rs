use std::time::Duration;

use egui::{Align, Label, Layout, TextStyle, Ui};

pub struct Marquee {
    offset: usize,
    accumulator: f32,

    speed: f32,

    state: MarqueeState,

    pause_timer: Duration,
    pause_duration: Duration,
}
enum MarqueeState {
    Paused,
    Scrolling,
}

impl Marquee {
    pub fn new() -> Self {
        Self {
            offset: 0,
            accumulator: 0.0,
            speed: 5.0,
            state: MarqueeState::Paused,
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
        self.offset = 0;
        self.accumulator = 0.0;
        self.pause_timer = self.pause_duration;
        self.state = MarqueeState::Paused;
    }
}

pub fn marquee_ui(ui: &mut Ui, marquee: &mut Marquee, text: &str) {
    if text.is_empty() {
        return;
    }

    let font_id = TextStyle::Body.resolve(ui.style());

    let chars_count = text.chars().count();
    let text_width: f32 = text.chars().map(|c| ui.fonts_mut(|r| r.glyph_width(&font_id, c))).sum();

    let average_char_width = text_width / chars_count as f32;

    let available_width = ui.available_width();
    let visible_chars = ((available_width / average_char_width).floor() as usize).max(1);

    // If everything fits, no marquee needed.
    if chars_count <= visible_chars {
        ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
            ui.add(Label::new(text).selectable(false).truncate());
        });

        marquee.reset();

        return;
    }

    let dt = ui.input(|i| i.stable_dt);

    match marquee.state {
        MarqueeState::Paused => {
            marquee.pause_timer = marquee.pause_timer.saturating_sub(Duration::from_secs_f32(dt));

            if marquee.pause_timer.is_zero() {
                marquee.state = MarqueeState::Scrolling;
            }
        }

        MarqueeState::Scrolling => {
            marquee.accumulator += marquee.speed * dt;

            while marquee.accumulator >= 1.0 {
                marquee.accumulator -= 1.0;
                marquee.offset += 1;

                if marquee.offset >= chars_count {
                    marquee.reset();
                    break;
                }
            }
        }
    }

    let display_text: String = text.chars().cycle().skip(marquee.offset).take(visible_chars).collect();

    ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
        ui.add(Label::new(display_text).selectable(false).truncate());
    });
}
