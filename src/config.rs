use anyhow::Result;
use confy::{load, store};
use egui::ThemePreference;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::GemPlayer;

const DEFAULT_VOLUME: f32 = 0.6;

#[derive(Serialize, Deserialize)]
pub struct GemConfig {
    pub library_directory: Option<PathBuf>,
    pub theme_preference: ThemePreference,
    pub volume: f32,
}

impl Default for GemConfig {
    fn default() -> Self {
        Self {
            library_directory: None,
            theme_preference: ThemePreference::System,
            volume: DEFAULT_VOLUME,
        }
    }
}

pub fn save_config(gem: &GemPlayer) -> Result<()> {
    let config = GemConfig {
        library_directory: gem.library_directory.clone(),
        theme_preference: gem.ui.theme_preference,
        volume: gem.player.backend.as_ref().map(|b| b.player.volume()).unwrap_or(DEFAULT_VOLUME),
    };

    store(env!("CARGO_PKG_NAME"), None, config)?;

    Ok(())
}

pub fn load_config() -> GemConfig {
    match load(env!("CARGO_PKG_NAME"), None) {
        Ok(cfg) => cfg,
        Err(err) => {
            eprintln!("Config load failed, using defaults: {err}");
            GemConfig::default()
        }
    }
}
