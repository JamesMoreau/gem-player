use egui::{Frame, Label, Margin, RichText, Ui};

pub struct MetadataChip<'a> {
    text: &'a str,
}

impl<'a> MetadataChip<'a> {
    pub fn new(text: &'a str) -> Self {
        Self { text }
    }
}

impl<'a> egui::Widget for MetadataChip<'a> {
    fn ui(self, ui: &mut Ui) -> egui::Response {
        let label = Label::new(RichText::new(self.text).small().weak()).selectable(false);

        Frame::new()
            .stroke(ui.visuals().widgets.noninteractive.bg_stroke)
            .corner_radius(4.0)
            .inner_margin(Margin::same(2))
            .outer_margin(Margin::same(2))
            .show(ui, |ui| ui.add(label))
            .response
    }
}
