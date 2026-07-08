use egui::{Image, TextureFilter, TextureOptions, Ui, Vec2, include_image};

pub fn artwork_ui(ui: &mut Ui, artwork_uri: Option<&str>, width: f32) {
    let artwork = if let Some(uri) = artwork_uri {
        Image::from_uri(uri)
    } else {
        Image::new(include_image!("../../../assets/icon.png"))
    };

    ui.add(
        artwork
            .texture_options(TextureOptions::LINEAR.with_mipmap_mode(Some(TextureFilter::Linear)))
            .show_loading_spinner(false)
            .fit_to_exact_size(Vec2::splat(width))
            .maintain_aspect_ratio(false)
            .corner_radius(2.0),
    );
}
