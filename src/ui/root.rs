use std::time::Duration;

use egui::{CentralPanel, Color32, Frame, Label, RichText, Separator, Stroke, ThemePreference, Ui, WidgetText};
use egui_extras::{Size, StripBuilder};
use egui_material_icons::icons::{ICON_LIBRARY_MUSIC, ICON_QUEUE_MUSIC, ICON_SETTINGS, ICON_STAR};
use egui_notify::Toasts;
use fully_pub::fully_pub;
use strum_macros::EnumIter;

use crate::{
    GemPlayer,
    custom_window::custom_window,
    ui::{
        bottom_bar::bottom_bar,
        control_panel::control_panel,
        file_drop_overlay::file_drop_overlay,
        library_view::{LibraryViewState, library_view},
        playlist_view::{PlaylistsViewState, playlists_view},
        queue_view::queue_view,
        settings_view::settings_view,
        widgets::{marquee::Marquee, track_artwork::Artwork},
    },
};

#[derive(Debug, Clone, PartialEq, Eq, EnumIter)]
pub enum View {
    Library,
    Playlists,
    Queue,
    Settings,
}

impl View {
    pub fn icon(&self) -> &'static str {
        match self {
            View::Library => ICON_LIBRARY_MUSIC.codepoint,
            View::Queue => ICON_QUEUE_MUSIC.codepoint,
            View::Playlists => ICON_STAR.codepoint,
            View::Settings => ICON_SETTINGS.codepoint,
        }
    }
}

#[fully_pub]
pub struct UIState {
    current_view: View,
    theme_preference: ThemePreference,
    marquee: Marquee,
    search: String,
    volume_popup_is_open: bool,

    cached_artwork: Option<Artwork>,

    library: LibraryViewState,
    playlists: PlaylistsViewState,

    toasts: Toasts,
}

pub fn gem_player_ui(ui: &mut Ui, gem: &mut GemPlayer) {
    CentralPanel::default()
        .frame(Frame::NONE.fill(ui.style().visuals.window_fill()))
        .show_inside(ui, |ui| {
            let is_hovering_files = ui.input(|i| !i.raw.hovered_files.is_empty());

            if is_hovering_files {
                file_drop_overlay(ui);
                return;
            }

            let control_ui_height = 80.0;
            let navigation_ui_height = 32.0;
            let separator_space = 2.0;

            StripBuilder::new(ui)
                .size(Size::exact(separator_space))
                .size(Size::exact(control_ui_height))
                .size(Size::exact(separator_space))
                .size(Size::remainder())
                .size(Size::exact(separator_space))
                .size(Size::exact(navigation_ui_height))
                .vertical(|mut strip| {
                    strip.cell(|ui| {
                        ui.add(Separator::default().spacing(separator_space));
                    });

                    strip.cell(|ui| control_panel(ui, gem));

                    strip.cell(|ui| {
                        ui.add(Separator::default().spacing(separator_space));
                    });

                    strip.cell(|ui| match gem.ui.current_view {
                        View::Library => library_view(ui, gem),
                        View::Queue => queue_view(ui, &mut gem.player),
                        View::Playlists => playlists_view(ui, gem),
                        View::Settings => settings_view(ui, gem),
                    });

                    strip.cell(|ui| {
                        ui.add(Separator::default().spacing(separator_space));
                    });

                    strip.cell(|ui| bottom_bar(ui, gem));
                });
        });
}

pub fn unselectable_label(text: impl Into<WidgetText>) -> Label {
    Label::new(text).selectable(false)
}

pub fn table_label(text: impl Into<String>, color: Option<Color32>) -> Label {
    let mut rich = RichText::new(text.into());
    if let Some(c) = color {
        rich = rich.color(c);
    }
    Label::new(rich).selectable(false).truncate()
}

pub fn format_duration_to_mmss(duration: Duration) -> String {
    let total_seconds = duration.as_secs();
    let seconds_per_minute = 60;
    let minutes = total_seconds / seconds_per_minute;
    let seconds = total_seconds % seconds_per_minute;

    format!("{}:{:02}", minutes, seconds)
}

pub fn format_duration_to_hhmmss(duration: Duration) -> String {
    let total_seconds = duration.as_secs();
    let seconds_per_minute = 60;
    let minutes_per_hour = 60;
    let hours = total_seconds / (minutes_per_hour * seconds_per_minute);
    let minutes = (total_seconds / seconds_per_minute) % minutes_per_hour;
    let seconds = total_seconds % seconds_per_minute;

    format!("{}:{:02}:{:02}", hours, minutes, seconds)
}
