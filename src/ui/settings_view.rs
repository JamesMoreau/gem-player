use std::path::Path;

use egui::{Frame, RichText, ScrollArea, Separator, ThemePreference, Ui, epaint::MarginF32};
use egui_material_icons::icons::ICON_FOLDER_OPEN;
use log::{error, info};

use crate::{
    APP_NAME, GemPlayer, apply_visuals_for_mode,
    library::spawn_library_folder_picker,
    localization::language_label,
    menu::{create_menu, init_menu, remove_menu},
    resolve_theme_mode,
    ui::{root::unselectable_label, widgets::toggle_switch::toggle},
};

#[cfg(target_os = "windows")]
use crate::apply_menu_theme_for_windows;

pub fn settings_view(ui: &mut Ui, _frame: &mut eframe::Frame, gem: &mut GemPlayer) {
    Frame::new()
        .outer_margin(MarginF32::symmetric(ui.available_width() * (1.0 / 4.0), 32.0))
        .show(ui, |ui| {
            ScrollArea::vertical().show(ui, |ui| {
                let divider_spacing = 32.0;

                ui.add(unselectable_label(RichText::new(t!("music_library_path")).heading()));
                ui.add_space(8.0);
                ui.add(unselectable_label(t!("playlists_stored_here")));
                ui.horizontal_wrapped(|ui| {
                    let no_directory = t!("no_directory_selected").to_string();

                    let (display_path, full_path) = match gem.library.directory.as_ref() {
                        Some(p) => (elide_path(p, 80), p.to_string_lossy().to_string()),
                        None => (no_directory.clone(), no_directory),
                    };

                    ui.label(display_path).on_hover_text(full_path);

                    let start_dir = gem.library.directory.as_deref().unwrap_or_else(|| Path::new("/")).to_path_buf();

                    if ui.button(ICON_FOLDER_OPEN).on_hover_text(t!("change")).clicked() {
                        let receiver = spawn_library_folder_picker(&start_dir);
                        gem.library.folder_picker_receiver = Some(receiver);
                    }
                });

                ui.add(Separator::default().spacing(divider_spacing));

                ui.add(unselectable_label(RichText::new(t!("theme")).heading()));
                ui.add_space(8.0);

                let previous_preference = gem.ui.theme_preference;

                ThemePreference::radio_buttons(&mut gem.ui.theme_preference, ui);

                if gem.ui.theme_preference != previous_preference {
                    let mode = resolve_theme_mode(gem.ui.theme_preference);
                    apply_visuals_for_mode(ui.ctx(), mode);

                    #[cfg(target_os = "windows")]
                    apply_menu_theme_for_windows(_frame, &gem.menubar.menu, mode);
                }

                ui.add(Separator::default().spacing(divider_spacing));

                ui.add(unselectable_label(RichText::new(t!("language")).heading()));
                ui.add_space(8.0);

                ui.horizontal_wrapped(|ui| {
                    let settings = &mut gem.ui.language_settings;

                    let system_label = language_label(None);
                    let mut changed = ui.selectable_value(&mut settings.selected_locale, None, system_label).changed();

                    for locale in settings.languages.clone() {
                        let label = language_label(Some(&locale));

                        changed |= ui.selectable_value(&mut settings.selected_locale, Some(locale), label).changed();
                    }

                    if changed {
                        settings.apply();

                        #[cfg(target_os = "windows")]
                        if let Err(e) = remove_menu(&gem.menubar.menu, _frame) {
                            error!("Unable to remove menu: {e}");
                            return;
                        }

                        #[cfg(target_os = "macos")]
                        remove_menu(&gem.menubar.menu);

                        let (menu, receiver) = create_menu();

                        #[cfg(target_os = "windows")]
                        let result = init_menu(&menu, gem.ui.theme_preference, _frame);

                        #[cfg(target_os = "macos")]
                        let result = init_menu(&menu);

                        match result {
                            Ok(()) => {
                                gem.menubar.menu = menu;
                                gem.menubar.menu_receiver = receiver;
                            }
                            Err(e) => {
                                error!("Unable to initialize menu: {e}");
                            }
                        }
                    }
                });

                ui.add(Separator::default().spacing(divider_spacing));

                ui.add(unselectable_label(RichText::new(t!("sleep_mode_blocker")).heading()));

                ui.add_space(8.0);

                ui.add(unselectable_label(t!("sleep_mode_blocker_description")));

                let mut enabled = gem.nosleep_manager.is_enabled();

                ui.add_space(8.0);

                if ui.add(toggle(&mut enabled)).changed() {
                    if enabled {
                        gem.nosleep_manager.enable();
                    } else {
                        gem.nosleep_manager.disable()
                    }

                    info!("Sleep inhibitor is now {}.", if enabled { "enabled" } else { "disabled" });
                }

                ui.add(Separator::default().spacing(divider_spacing));

                ui.add(unselectable_label(RichText::new(t!("about_app", name = APP_NAME)).heading()));
                ui.add_space(8.0);

                ui.add(unselectable_label(t!("app_description")));

                ui.add_space(8.0);

                let repo_link = env!("CARGO_PKG_REPOSITORY");

                ui.horizontal_wrapped(|ui| {
                    let version = env!("CARGO_PKG_VERSION");
                    ui.add(unselectable_label(t!("version", version = version)));

                    ui.add(unselectable_label(" / "));

                    let release_link = format!("{repo_link}/releases/tag/v{version}");
                    ui.hyperlink_to(t!("release_notes"), release_link);

                    ui.add(unselectable_label(" / "));

                    ui.hyperlink_to(t!("source"), repo_link);
                });

                ui.add_space(8.0);

                ui.horizontal_wrapped(|ui| {
                    ui.add(unselectable_label(t!("feedback_prompt")));

                    let issue_link = format!("{repo_link}/issues");
                    ui.hyperlink_to(t!("issue_tracker"), issue_link);
                });

                ui.add_space(8.0);

                ui.horizontal_wrapped(|ui| {
                    ui.add(unselectable_label(t!("author")));

                    ui.add(unselectable_label(env!("CARGO_PKG_AUTHORS")));

                    ui.add(unselectable_label(" / "));

                    ui.hyperlink_to("jamesmoreau.github.io", "https://jamesmoreau.github.io");
                });

                ui.add_space(8.0);

                ui.horizontal_wrapped(|ui| {
                    ui.add(unselectable_label(t!("support_author_prompt")));
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
