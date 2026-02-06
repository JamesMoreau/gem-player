use std::path::Path;

use egui::{containers, ComboBox, Frame, Margin, RichText, ScrollArea, Separator, ThemePreference, Ui};
use egui_material_icons::icons;
use fully_pub::fully_pub;
use log::error;
use rodio::{Device, DeviceTrait};

use crate::{
    apply_theme,
    player::{get_audio_output_devices_and_names, switch_audio_devices},
    spawn_folder_picker,
    ui::root::unselectable_label,
    GemPlayer, KEY_COMMANDS,
};

#[fully_pub]
struct SettingsViewState {
    audio_output_devices_cache: Vec<(Device, String)>,
}

pub fn settings_view(ui: &mut Ui, gem: &mut GemPlayer) {
    Frame::new()
        .outer_margin(Margin::symmetric((ui.available_width() * (1.0 / 4.0)) as i8, 32))
        .show(ui, |ui| {
            ScrollArea::vertical().show(ui, |ui| {
                ui.add(unselectable_label(RichText::new("Music Library Path").heading()));
                ui.add_space(8.0);
                ui.add(unselectable_label("Playlists are also stored here as m3u files."));
                ui.horizontal(|ui| {
                    let (display_path, full_path) = match gem.library_directory.as_ref() {
                        Some(p) => (elide_path(p, 80), p.to_string_lossy().to_string()),
                        None => ("No directory selected".to_string(), "No directory selected".to_string()),
                    };

                    ui.label(display_path).on_hover_text(full_path);

                    let start_dir = gem.library_directory.as_deref().unwrap_or_else(|| Path::new("/")).to_path_buf();

                    if ui.button(icons::ICON_FOLDER_OPEN).on_hover_text("Change").clicked() {
                        let receiver = spawn_folder_picker(&start_dir);
                        gem.folder_picker_receiver = Some(receiver);
                    }
                });

                ui.add(Separator::default().spacing(32.0));

                ui.add(unselectable_label(RichText::new("Audio").heading()));

                ui.add_space(8.0);

                let selected_device_text = gem
                    .player
                    .backend
                    .as_ref()
                    .and_then(|b| b.device.name().ok())
                    .unwrap_or_else(|| "No device".to_string());

                let inner = ComboBox::from_label("Output device")
                    .selected_text(selected_device_text)
                    .show_ui(ui, |ui| {
                        for (device, name) in &gem.ui.settings.audio_output_devices_cache {
                            let maybe_backend = gem.player.backend.as_ref();
                            let mut is_selected = maybe_backend.is_some_and(|b| b.device.name().ok() == Some(name.clone()));

                            let response = ui.selectable_value(&mut is_selected, true, name.clone());

                            if response.clicked() {
                                if let Err(err) = switch_audio_devices(&mut gem.player, device.clone()) {
                                    error!("Failed to switch device: {}", err);
                                    gem.ui.toasts.error("Failed to switch audio device.");
                                }
                            }
                        }
                    });

                if inner.response.clicked() {
                    gem.ui.settings.audio_output_devices_cache = get_audio_output_devices_and_names();
                }

                ui.add(Separator::default().spacing(32.0));

                ui.add(unselectable_label(RichText::new("Theme").heading()));
                ui.add_space(8.0);

                let before = gem.ui.theme_preference;
                ThemePreference::radio_buttons(&mut gem.ui.theme_preference, ui);
                let after = gem.ui.theme_preference;

                let theme_was_changed = before != after;
                if theme_was_changed {
                    apply_theme(ui.ctx(), after);
                }

                ui.add(Separator::default().spacing(32.0));

                ui.add(unselectable_label(RichText::new("Controls").heading()));

                ui.add_space(8.0);
                for (key, binding) in KEY_COMMANDS.iter() {
                    containers::Sides::new().show(
                        ui,
                        |ui| {
                            ui.add(unselectable_label(format!("{:?}", key)));
                        },
                        |ui| {
                            ui.add_space(16.0);
                            ui.add(unselectable_label(binding.to_string()));
                        },
                    );
                }

                containers::Sides::new().show(
                    ui,
                    |ui| {
                        ui.add(unselectable_label("Shift + Click"));
                    },
                    |ui| {
                        ui.add_space(16.0);
                        ui.add(unselectable_label("Select multiple tracks"));
                    },
                );

                ui.add(Separator::default().spacing(32.0));

                ui.add(unselectable_label(RichText::new("About Gem Player").heading()));
                ui.add_space(8.0);

                let description = env!("CARGO_PKG_DESCRIPTION");
                ui.add(unselectable_label(description));

                ui.add_space(8.0);

                let repo_link = env!("CARGO_PKG_REPOSITORY");

                ui.horizontal_wrapped(|ui| {
                    let version = env!("CARGO_PKG_VERSION");
                    ui.add(unselectable_label(format!("Version: {version}")));

                    ui.add(unselectable_label(" / "));

                    let release_link = format!("{}/releases/tag/v{}", repo_link, version);
                    ui.hyperlink_to("release notes", release_link);

                    ui.add(unselectable_label(" / "));

                    ui.hyperlink_to("source", repo_link);
                });

                ui.add_space(8.0);

                ui.horizontal(|ui| {
                    ui.add(unselectable_label(
                        "Bug reports, feature requests, and feedback may be submitted to the",
                    ));
                    let issue_link = format!("{}/issues", repo_link);
                    ui.hyperlink_to("issue tracker", issue_link);
                });

                ui.add_space(8.0);

                ui.horizontal_wrapped(|ui| {
                    ui.add(unselectable_label("Author:"));

                    ui.add(unselectable_label("James Moreau"));

                    ui.add(unselectable_label(" / "));

                    ui.hyperlink_to("jamesmoreau.github.io", "https://jamesmoreau.github.io");
                });

                ui.add_space(8.0);

                ui.horizontal_wrapped(|ui| {
                    ui.add(unselectable_label("If you like this project, consider supporting the author:"));
                    ui.hyperlink_to("Ko-fi", "https://ko-fi.com/jamesmoreau");
                });
            });
        });
}

/// Elide a path string to something like `/Users/user1/…/Music`
/// Keeps both start and end parts if the path is too long.
pub fn elide_path(path: &Path, max_len: usize) -> String {
    let full = path.to_string_lossy();
    let full_len = full.len();

    if full_len <= max_len {
        return full.into_owned();
    }

    // Split budget roughly in half: keep some start, some end
    let keep_each_side = (max_len.saturating_sub(1)) / 2; // subtract 1 for the ellipsis

    let start = &full[..keep_each_side];
    let end = &full[full_len - keep_each_side..];

    format!("{start}…{end}")
}
