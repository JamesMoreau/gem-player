use std::time::Duration;

use egui::{Align, Button, Frame, Layout, Margin, Popup, RectAlign, RichText, Slider, Ui, Vec2};
use egui_extras::{Size, StripBuilder};
use egui_material_icons::icons::{
    ICON_PAUSE, ICON_PLAY_ARROW, ICON_REPEAT, ICON_SHUFFLE, ICON_SKIP_NEXT, ICON_SKIP_PREVIOUS, ICON_VOLUME_DOWN, ICON_VOLUME_OFF,
    ICON_VOLUME_UP,
};

use crate::{
    GemPlayer,
    commands::GemCommand,
    player::{Player, get_position},
    track::{Track, file_type_name},
    ui::{
        root::{format_duration_to_mmss, unselectable_label},
        widgets::{
            bar_display::BarDisplay,
            marquee::{Marquee, marquee_ui},
            metadata_chip::MetadataChip,
            track_artwork::track_artwork_ui,
        },
    },
    visualizer::smooth_bars,
};

pub fn control_panel(ui: &mut Ui, gem: &mut GemPlayer) {
    // Specifying the widths of the elements in the track info component before-hand allows us to center them horizontally.
    let button_size = 20.0;
    let gap = 10.0;
    let artwork_width = ui.available_height() - 4.0; // leave some space for the track info frame background.
    let slider_width = 420.0;

    Frame::new().inner_margin(Margin::symmetric(16, 0)).show(ui, |ui| {
        StripBuilder::new(ui)
            .size(Size::remainder())
            .size(Size::exact(gap + button_size + gap + artwork_width + gap + slider_width + gap))
            .size(Size::remainder())
            .horizontal(|mut strip| {
                strip.cell(|ui| {
                    ui.with_layout(Layout::left_to_right(Align::Center), |ui| playback_controls(ui, gem));
                });

                strip.cell(|ui| layout_track_display(ui, gem, button_size, gap, artwork_width, slider_width));

                strip.cell(|ui| {
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        display_visualizer(ui, gem);

                        ui.add_space(16.0);

                        volume_control_button(ui, gem);
                    });
                });
            });
    });
}

fn volume_control_button(ui: &mut Ui, gem: &mut GemPlayer) {
    let has_backend = gem.player.backend.is_some();

    let mut volume = gem.player.backend.as_ref().map(|b| b.player.volume()).unwrap_or(0.0);

    let volume_icon = match volume {
        0.0 => ICON_VOLUME_OFF,
        v if v <= 0.5 => ICON_VOLUME_DOWN,
        _ => ICON_VOLUME_UP, // v > 0.5 && v <= 1.0
    };

    let volume_button = Button::new(RichText::new(volume_icon).size(18.0));
    let response = ui.add_enabled(has_backend, volume_button);

    if !has_backend {
        // In case the backend disappears while the popup is already open, cleanup.
        gem.ui.volume_popup_is_open = false;

        // Since there is no backend, no popup or interaction can occur.
        return;
    }

    let mut button_is_hovered = false;
    let mut popup_is_hovered = false;

    Popup::menu(&response)
        .open(gem.ui.volume_popup_is_open)
        .align(RectAlign::RIGHT)
        .gap(4.0)
        .show(|ui| {
            let volume_slider = Slider::new(&mut volume, 0.0..=1.0).trailing_fill(true).show_value(false);
            let changed = ui.add(volume_slider).changed();
            if changed {
                gem.player.muted = false;
                gem.player.volume_before_mute = if volume == 0.0 { None } else { Some(volume) };

                gem.commands.push(GemCommand::SetVolume(volume));
            }

            if ui.rect_contains_pointer(ui.max_rect().expand(8.0)) {
                popup_is_hovered = true;
            }
        });

    if ui.rect_contains_pointer(response.rect.expand(8.0)) {
        button_is_hovered = true;
    }

    gem.ui.volume_popup_is_open = button_is_hovered || popup_is_hovered;

    if response.clicked() {
        gem.commands.push(GemCommand::ToggleMute);
    }
}

fn playback_controls(ui: &mut Ui, gem: &mut GemPlayer) {
    let has_backend = gem.player.backend.is_some();
    let track_is_playing = gem.player.playing.is_some();

    let previous_button = Button::new(ICON_SKIP_PREVIOUS.rich_text().size(18.0));
    let previous_track_exists = !gem.player.history.is_empty();
    let previous_enabled = has_backend && (track_is_playing || previous_track_exists);

    let response = ui
        .add_enabled(previous_enabled, previous_button)
        .on_hover_text("Previous")
        .on_disabled_hover_text("No previous track");
    if response.clicked() {
        gem.commands.push(GemCommand::PreviousTrack);
    }

    let sink_is_paused = gem.player.backend.as_ref().is_some_and(|b| b.player.is_paused());
    let play_pause_icon = if sink_is_paused { ICON_PLAY_ARROW } else { ICON_PAUSE };
    let tooltip = if sink_is_paused { "Play" } else { "Pause" };
    let play_pause_enabled = has_backend && track_is_playing;
    let play_pause_button = Button::new(RichText::new(play_pause_icon).size(28.0));
    let response = ui
        .add_enabled(play_pause_enabled, play_pause_button)
        .on_hover_text(tooltip)
        .on_disabled_hover_text("No current track");

    if response.clicked() {
        gem.commands.push(GemCommand::TogglePlayback);
    }

    let next_button = Button::new(ICON_SKIP_NEXT.rich_text().size(18.0));
    let next_enabled = has_backend && !gem.player.queue.is_empty();
    let response = ui
        .add_enabled(next_enabled, next_button)
        .on_hover_text("Next")
        .on_disabled_hover_text("No next track");
    if response.clicked() {
        gem.commands.push(GemCommand::NextTrack);
    }
}

fn layout_track_display(ui: &mut Ui, gem: &mut GemPlayer, button_size: f32, gap: f32, artwork_width: f32, slider_width: f32) {
    ui.scope(|ui| {
        ui.spacing_mut().item_spacing = Vec2::splat(0.0);

        Frame::new().corner_radius(4.0).fill(ui.visuals().faint_bg_color).show(ui, |ui| {
            StripBuilder::new(ui)
                .size(Size::exact(gap))
                .size(Size::exact(button_size))
                .size(Size::exact(gap))
                .size(Size::exact(artwork_width))
                .size(Size::exact(gap))
                .size(Size::exact(slider_width))
                .size(Size::exact(gap))
                .horizontal(|mut strip| {
                    strip.empty();
                    strip.cell(|ui| display_repeat_and_shuffle_buttons(ui, gem, button_size));
                    strip.empty();
                    strip.cell(|ui| {
                        ui.centered_and_justified(|ui| {
                            track_artwork_ui(ui, gem.ui.cached_artwork.as_ref(), artwork_width);
                        });
                    });
                    strip.empty();
                    strip.cell(|ui| layout_playback_slider_and_track_info(ui, gem, slider_width));
                    strip.empty();
                });
        });
    });
}

fn display_repeat_and_shuffle_buttons(ui: &mut Ui, gem: &mut GemPlayer, button_size: f32) {
    ui.scope(|ui| {
        ui.spacing_mut().item_spacing = Vec2::splat(0.0);

        let vertical_pad = 8.0;
        let starting_point = (ui.available_height() / 2.0) - (vertical_pad / 2.0) - button_size; // this is how we align the buttons vertically center.
        ui.add_space(starting_point);

        let get_button_color = |ui: &Ui, is_enabled: bool| {
            if is_enabled {
                ui.visuals().selection.bg_fill
            } else {
                ui.visuals().text_color()
            }
        };

        let color = get_button_color(ui, gem.player.repeat);
        let repeat_button = Button::new(ICON_REPEAT.rich_text().color(color)).min_size(Vec2::splat(button_size));

        if ui.add(repeat_button).on_hover_text("Repeat").clicked() {
            gem.commands.push(GemCommand::ToggleRepeat);
        }

        ui.add_space(vertical_pad);

        let color = get_button_color(ui, gem.player.shuffle.is_some());
        let shuffle_button = Button::new(ICON_SHUFFLE.rich_text().color(color)).min_size(Vec2::splat(button_size));
        let shuffle_enabled = !gem.player.queue.is_empty();

        let response = ui
            .add_enabled(shuffle_enabled, shuffle_button)
            .on_hover_text("Shuffle")
            .on_disabled_hover_text("Queue is empty");

        if response.clicked() {
            gem.commands.push(GemCommand::ToggleShuffle);
        }
    });
}

fn layout_playback_slider_and_track_info(ui: &mut Ui, gem: &mut GemPlayer, slider_width: f32) {
    // We retrieve the position here so that scrubbing using the slider will be
    // reflected in the playback position ui.
    let mut position = get_position(&gem.player).unwrap_or_default();

    StripBuilder::new(ui).sizes(Size::relative(1.0 / 2.0), 2).vertical(|mut strip| {
        strip.cell(|ui| {
            ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                playback_slider(ui, gem, &mut position, slider_width);
            });
        });

        strip.cell(|ui| {
            layout_marquee_and_playback_position_and_metadata(ui, &gem.player, position, &mut gem.ui.marquee);
        });
    });
}

fn playback_slider(ui: &mut Ui, gem: &mut GemPlayer, position: &mut Duration, slider_width: f32) {
    ui.scope(|ui| {
        ui.spacing_mut().slider_width = slider_width;

        let slider_enabled = gem.player.backend.is_some() && gem.player.playing.is_some();

        let track_duration = gem.player.playing.as_ref().map_or(Duration::ZERO, |t| t.duration);

        let mut position_as_secs = position.as_secs_f32();

        let slider = Slider::new(&mut position_as_secs, 0.0..=track_duration.as_secs_f32().max(0.1))
            .trailing_fill(true)
            .show_value(false)
            .step_by(1.0);

        let response = ui.add_enabled(slider_enabled, slider);

        if !slider_enabled {
            return;
        }

        // Handle scrubbing.

        *position = Duration::from_secs_f32(position_as_secs);

        if response.dragged() && gem.player.paused_before_scrubbing.is_none() {
            let is_paused = gem
                .player
                .backend
                .as_ref()
                .expect("backend should exist if slider is enabled")
                .player
                .is_paused();

            gem.player.paused_before_scrubbing = Some(is_paused);

            gem.commands.push(GemCommand::Pause);
        }

        if response.drag_stopped() {
            gem.commands.push(GemCommand::SeekTo(*position));
        }
    });
}

fn layout_marquee_and_playback_position_and_metadata(ui: &mut Ui, player: &Player, position: Duration, marquee: &mut Marquee) {
    let duration = if let Some(track) = &player.playing {
        track.duration
    } else {
        Duration::ZERO
    };

    // Placing the track info after the slider ensures that the playback position display is accurate. The seek operation is only
    // executed after the slider thumb is released. If we placed the display before, the current position would not be reflected.
    StripBuilder::new(ui)
        .size(Size::relative(3.0 / 4.0))
        .size(Size::relative(1.0 / 4.0))
        .horizontal(|mut hstrip| {
            hstrip.cell(|ui| display_track_marquee(ui, player.playing.as_ref(), marquee));
            hstrip.cell(|ui| {
                StripBuilder::new(ui).sizes(Size::relative(1.0 / 2.0), 2).vertical(|mut strip| {
                    strip.cell(|ui| {
                        ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
                            display_playback_time(ui, position, duration);
                        });
                    });

                    strip.cell(|ui| {
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            if let Some(playing) = &player.playing {
                                display_track_metadata(ui, playing);
                            }
                        });
                    });
                });
            });
        });
}

fn display_track_marquee(ui: &mut Ui, maybe_track: Option<&Track>, marquee: &mut Marquee) {
    let mut title = "-";
    let mut artist = "-";
    let mut album = "-";

    if let Some(playing_track) = maybe_track {
        title = playing_track.title.as_deref().unwrap_or("Unknown Title");
        artist = playing_track.artist.as_deref().unwrap_or("Unknown Artist");
        album = playing_track.album.as_deref().unwrap_or("Unknown Album");
    }

    let padding = "        ";
    let text = format!("{} / {} / {}{}", title, artist, album, padding);

    marquee_ui(ui, marquee, &text);
}

fn display_playback_time(ui: &mut Ui, position: Duration, duration: Duration) {
    let time_label_text = format!("{} / {}", format_duration_to_mmss(position), format_duration_to_mmss(duration));

    let time_label = unselectable_label(time_label_text);
    ui.add(time_label);
}

fn display_track_metadata(ui: &mut Ui, track: &Track) {
    let codec_string = file_type_name(track.codec);
    ui.add(MetadataChip::new(codec_string));

    ui.add_space(4.0);

    if let Some(sr) = track.sample_rate {
        let sample_rate_string = format!("{:.1} kHz", sr.get() as f32 / 1000.0);
        ui.add(MetadataChip::new(&sample_rate_string));
    }
}

fn display_visualizer(ui: &mut Ui, gem: &mut GemPlayer) {
    let dt = ui.input(|i| i.stable_dt);

    let targets = gem.player.visualizer.bands_receiver.try_iter().last();

    smooth_bars(&mut gem.player.visualizer.display_bands, targets.as_deref(), dt);

    let display_bands = &gem.player.visualizer.display_bands;

    let display = BarDisplay::new(display_bands, ui.available_height() * 0.5, 10.0, 4.0, ui.visuals().text_color());

    ui.add(display);
}
