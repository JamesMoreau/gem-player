use std::path::Path;

use egui::{Image, TextureFilter, TextureOptions, Ui, Vec2, include_image};
use fully_pub::fully_pub;

#[fully_pub]
struct Artwork {
    uri: String,
    bytes: Vec<u8>,
}

// Use track path as a unique/stable key for egui
pub fn compute_uri(path: &Path) -> String {
    format!("bytes://{}", path.to_string_lossy())
}

pub fn artwork_ui(ui: &mut Ui, maybe_artwork: Option<&Artwork>, width: f32) {
    let artwork = if let Some(a) = maybe_artwork {
        Image::from_bytes(a.uri.to_owned(), a.bytes.to_vec())
    } else {
        Image::new(include_image!("../../../assets/icon.png")) // placeholder
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
