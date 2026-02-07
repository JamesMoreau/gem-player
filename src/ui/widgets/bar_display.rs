use egui::{Color32, Rect, Response, Sense, Ui, Widget, pos2, vec2};

pub struct BarDisplay<'a> {
    values: &'a [f32],

    desired_height: f32,
    bar_width: f32,
    bar_gap: f32,
    bar_radius: f32,
    min_bar_height: f32,
    color: Color32,
}

impl<'a> BarDisplay<'a> {
    pub fn new(values: &'a [f32], desired_height: f32, bar_width: f32, bar_gap: f32, color: Color32) -> Self {
        Self {
            values,
            desired_height,
            bar_width,
            bar_gap,
            bar_radius: 1.0,
            min_bar_height: 3.0,
            color,
        }
    }
}

impl Widget for BarDisplay<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let num_bars = self.values.len() as f32;
        let total_width = (num_bars * self.bar_width) + ((num_bars - 1.0).max(0.0) * self.bar_gap);

        let (rect, response) = ui.allocate_exact_size(vec2(total_width, self.desired_height), Sense::hover());

        let painter = ui.painter();

        for (i, &value) in self.values.iter().enumerate() {
            let height = (value.clamp(0.0, 1.0) * rect.height()).max(self.min_bar_height);

            let x = rect.left() + i as f32 * (self.bar_width + self.bar_gap);
            let y = rect.bottom();

            let bar_rect = Rect::from_min_max(pos2(x, y - height), pos2(x + self.bar_width, y));

            painter.rect_filled(bar_rect, self.bar_radius, self.color);
        }

        response
    }
}
