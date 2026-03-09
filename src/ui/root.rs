use std::time::Duration;

use egui::{pos2, vec2, Color32, Context, Label, Rect, RichText, Sense, Separator, ThemePreference, Ui, WidgetText};
use egui_extras::{Size, StripBuilder};
use egui_material_icons::icons;
use egui_notify::Toasts;
use fully_pub::fully_pub;
use strum_macros::EnumIter;

use crate::{
    custom_window::custom_window,
    ui::{
        bottom_bar::bottom_bar,
        control_panel::control_panel_ui,
        file_drag_and_drop::drop_files_area_ui,
        library_view::{library_view, LibraryViewState},
        playlist_view::{playlists_view, PlaylistsViewState},
        queue_view::queue_view,
        settings_view::settings_view,
        widgets::marquee::Marquee,
    },
    GemPlayer,
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
            View::Library => icons::ICON_LIBRARY_MUSIC,
            View::Queue => icons::ICON_QUEUE_MUSIC,
            View::Playlists => icons::ICON_STAR,
            View::Settings => icons::ICON_SETTINGS,
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

    cached_artwork_uri: Option<String>,
    cached_artwork: Option<Vec<u8>>,

    library: LibraryViewState,
    playlists: PlaylistsViewState,

    library_and_playlists_are_loading: bool,

    toasts: Toasts,
}

pub fn gem_player_ui(gem: &mut GemPlayer, ctx: &Context) {
    custom_window(ctx, "", |ui| {
        let is_dropping_files = drop_files_area_ui(ui, gem);
        if is_dropping_files {
            return; // Don't render anything else if files are being dropped.
        }

        let control_ui_height = 80.0;
        let navigation_ui_height = 32.0;
        let separator_space = 2.0; // Even numbers seem to work better for getting pixel perfect placements.

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
                strip.cell(|ui| control_panel_ui(ui, gem));
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

pub fn playing_indicator(ui: &mut Ui) {
    let desired_height = ui.available_height() * 0.4;
    let desired_width = 18.0;

    let (rect, _response) = ui.allocate_exact_size(vec2(desired_width, desired_height), Sense::hover());

    let time = ui.ctx().input(|i| i.time) as f32;
    let display_bars = [
        ((time * 6.0).sin() * 0.4 + 0.6).max(0.2),
        ((time * 7.5).cos() * 0.4 + 0.6).max(0.2),
        ((time * 5.3).sin() * 0.4 + 0.6).max(0.2),
    ];

    let bar_gap = 1.0;
    let bar_radius = 1.0;
    let bar_width = rect.width() / display_bars.len() as f32;
    let min_bar_height = 2.0;

    for (i, value) in display_bars.into_iter().enumerate() {
        let height = (value * rect.height()).max(min_bar_height);
        let x = rect.left() + i as f32 * bar_width + bar_gap / 2.0;
        let y = rect.bottom();

        let bar_rect = Rect::from_min_max(pos2(x, y - height), pos2(x + bar_width - bar_gap, y));
        ui.painter().rect_filled(bar_rect, bar_radius, ui.visuals().selection.bg_fill);
    }
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
