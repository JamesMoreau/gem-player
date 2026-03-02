use std::fs::copy;

use anyhow::{anyhow, bail, Context, Result};
use egui::{DroppedFile, Frame, Margin, Ui};
use egui_material_icons::icons;

use crate::{track::is_audio_file, ui::root::unselectable_label, GemPlayer};

pub fn drop_files_area_ui(ui: &mut Ui, gem: &mut GemPlayer) -> bool {
    let mut drop_area_is_active = false;

    let files_are_hovered = ui.ctx().input(|i| !i.raw.hovered_files.is_empty());
    let files_were_dropped = ui.ctx().input(|i| !i.raw.dropped_files.is_empty());

    if files_were_dropped {
        ui.ctx().input(|i| {
            for dropped_file in &i.raw.dropped_files {
                let result = handle_dropped_file(dropped_file, gem);
                if let Err(e) = result {
                    gem.ui.toasts.error(format!("Error adding file: {}", e));
                } else {
                    let file_name = dropped_file
                        .path
                        .as_ref()
                        .and_then(|p| p.file_name())
                        .and_then(|f| f.to_str())
                        .unwrap_or("Unnamed file");

                    gem.ui.toasts.success(format!("Added '{}' to Library.", file_name));
                }
            }
        });
    }

    if files_are_hovered {
        Frame::new()
            .outer_margin(Margin::symmetric(
                (ui.available_width() * (1.0 / 4.0)) as i8,
                (ui.available_height() * (1.0 / 4.0)) as i8,
            ))
            .show(ui, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add(unselectable_label(format!(
                        "Drop tracks here to add them to your library.{}",
                        icons::ICON_DOWNLOAD
                    )));
                });
            });
        drop_area_is_active = true;
    }

    drop_area_is_active
}

pub fn handle_dropped_file(dropped_file: &DroppedFile, gem: &mut GemPlayer) -> Result<()> {
    let path = dropped_file.path.as_ref().ok_or_else(|| anyhow!("Dropped file has no path"))?;

    let library_path = gem.library_directory.as_ref().ok_or_else(|| anyhow!("No library directory set"))?;

    let file_name = path.file_name().ok_or_else(|| anyhow!("Dropped file has no file name"))?;

    if !is_audio_file(path) {
        bail!("Dropped file is not a supported audio file");
    }

    let destination = library_path.join(file_name);

    copy(path, destination).with_context(|| format!("Failed to copy '{}' to library", path.display()))?;

    Ok(())
}
