use std::{path::PathBuf, time::Duration};

use eframe::egui::{
    include_image, pos2, text, vec2, Align, Button, Frame, Image, Label, Layout, Margin, Popup, Rect, RectAlign, RichText, Sense, Slider,
    TextFormat, TextStyle, TextureFilter, TextureOptions, Ui, Vec2,
};
use egui_extras::{Size, StripBuilder};
use egui_material_icons::icons;
use fully_pub::fully_pub;
use log::{error, info};

use crate::{
    format_duration_to_mmss, maybe_play_next, maybe_play_previous,
    player::{mute_or_unmute, play_or_pause, toggle_shuffle, Player},
    track::{file_type_name, Track},
    ui::root::unselectable_label,
    visualizer::calculate_bands,
    GemPlayer,
};

const MARQUEE_SPEED: f32 = 5.0; // chars per second
const MARQUEE_PAUSE_DURATION: Duration = Duration::from_secs(2);

#[fully_pub]
struct MarqueeState {
    track_key: Option<PathBuf>, // We need to know when the track changes to reset.
    position: f32,
    pause_timer: Duration,
}

pub fn control_panel_ui(ui: &mut Ui, gem: &mut GemPlayer) {
    // Specifying the widths of the elements in the track info component before-hand allows us to center them horizontally.
    let button_width = 20.0;
    let gap = 10.0;
    let artwork_width = ui.available_height() - 4.0; // leave some space for the track info frame background.
    let slider_width = 420.0;

    Frame::new().inner_margin(Margin::symmetric(16, 0)).show(ui, |ui| {
        StripBuilder::new(ui)
            .size(Size::remainder())
            .size(Size::exact(gap + button_width + gap + artwork_width + gap + slider_width + gap))
            .size(Size::remainder())
            .horizontal(|mut strip| {
                strip.cell(|ui| {
                    ui.with_layout(Layout::left_to_right(Align::Center), |ui| playback_controls_ui(ui, gem));
                });

                strip.cell(|ui| {
                    layout_playing_track_ui(
                        ui,
                        &mut gem.player,
                        &mut gem.ui.marquee,
                        button_width,
                        gap,
                        artwork_width,
                        slider_width,
                    )
                });

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
    let mut volume = gem.player.backend.as_ref().map(|b| b.sink.volume()).unwrap_or(0.0);

    let volume_icon = match volume {
        0.0 => icons::ICON_VOLUME_OFF,
        v if v <= 0.5 => icons::ICON_VOLUME_DOWN,
        _ => icons::ICON_VOLUME_UP, // v > 0.5 && v <= 1.0
    };

    let volume_button = Button::new(RichText::new(volume_icon).size(18.0));
    let response = ui.add(volume_button);

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

                if let Some(backend) = &gem.player.backend {
                    backend.sink.set_volume(volume);
                }
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
        mute_or_unmute(&mut gem.player);
    }
}

fn playback_controls_ui(ui: &mut Ui, gem: &mut GemPlayer) {
    let track_is_playing = gem.player.playing.is_some();

    let previous_button = Button::new(RichText::new(icons::ICON_SKIP_PREVIOUS).size(18.0));
    let previous_track_exists = !gem.player.history.is_empty();
    let is_previous_enabled = track_is_playing || previous_track_exists;

    let response = ui
        .add_enabled(is_previous_enabled, previous_button)
        .on_hover_text("Previous")
        .on_disabled_hover_text("No previous track");
    if response.clicked() {
        maybe_play_previous(gem)
    }

    let sink_is_paused = gem.player.backend.as_ref().is_some_and(|b| b.sink.is_paused());
    let play_pause_icon = if sink_is_paused {
        icons::ICON_PLAY_ARROW
    } else {
        icons::ICON_PAUSE
    };
    let tooltip = if sink_is_paused { "Play" } else { "Pause" };
    let play_pause_button = Button::new(RichText::new(play_pause_icon).size(28.0));
    let response = ui
        .add_enabled(track_is_playing, play_pause_button)
        .on_hover_text(tooltip)
        .on_disabled_hover_text("No current track");
    if response.clicked() {
        if let Some(backend) = &mut gem.player.backend {
            play_or_pause(&mut backend.sink);
        }
    }

    let next_button = Button::new(RichText::new(icons::ICON_SKIP_NEXT).size(18.0));
    let next_track_exists = !gem.player.queue.is_empty();
    let response = ui
        .add_enabled(next_track_exists, next_button)
        .on_hover_text("Next")
        .on_disabled_hover_text("No next track");
    if response.clicked() {
        maybe_play_next(gem);
    }
}

fn layout_playing_track_ui(
    ui: &mut Ui,
    player: &mut Player,
    marquee: &mut MarqueeState,
    button_size: f32,
    gap: f32,
    artwork_width: f32,
    slider_width: f32,
) {
    let previous_item_spacing = ui.spacing().item_spacing;
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
                strip.cell(|ui| display_repeat_and_shuffle_buttons(ui, player, button_size));
                strip.empty();
                strip.cell(|ui| {
                    ui.centered_and_justified(|ui| display_playing_artwork(ui, player, artwork_width));
                });
                strip.empty();
                strip.cell(|ui| layout_playback_slider_and_track_info_ui(ui, player, marquee, slider_width));
                strip.empty();
            });
    });

    ui.spacing_mut().item_spacing = previous_item_spacing;
}

fn display_repeat_and_shuffle_buttons(ui: &mut Ui, player: &mut Player, button_size: f32) {
    ui.spacing_mut().item_spacing = Vec2::splat(0.0);
    let starting_point = (ui.available_height() / 2.0) - button_size; // this is how we align the buttons vertically center.
    ui.add_space(starting_point);

    let get_button_color = |ui: &Ui, is_enabled: bool| {
        if is_enabled {
            ui.visuals().selection.bg_fill
        } else {
            ui.visuals().text_color()
        }
    };

    let color = get_button_color(ui, player.repeat);
    let repeat_button = Button::new(RichText::new(icons::ICON_REPEAT).color(color)).min_size(Vec2::splat(button_size));
    let response = ui.add(repeat_button).on_hover_text("Repeat");
    if response.clicked() {
        player.repeat = !player.repeat;
    }

    ui.add_space(4.0);

    let color = get_button_color(ui, player.shuffle.is_some());
    let shuffle_button = Button::new(RichText::new(icons::ICON_SHUFFLE).color(color)).min_size(Vec2::splat(button_size));
    let shuffle_enabled = !player.queue.is_empty();
    let response = ui
        .add_enabled(shuffle_enabled, shuffle_button)
        .on_hover_text("Shuffle")
        .on_disabled_hover_text("Queue is empty");
    if response.clicked() {
        toggle_shuffle(player);
    }
}

fn display_playing_artwork(ui: &mut Ui, player: &mut Player, artwork_width: f32) {
    let artwork_texture_options = TextureOptions::LINEAR.with_mipmap_mode(Some(TextureFilter::Linear));
    let artwork_size = Vec2::splat(artwork_width);

    let placeholder = include_image!("../../assets/icon.png");
    let mut artwork = Image::new(placeholder);

    if let (Some(track), Some(bytes)) = (&player.playing, &player.playing_artwork) {
        // Use track path as a unique/stable key for egui
        let uri = format!("bytes://{}", track.path.to_string_lossy());
        artwork = Image::from_bytes(uri, bytes.clone());
    }

    ui.add(
        artwork
            .texture_options(artwork_texture_options)
            .show_loading_spinner(false)
            .fit_to_exact_size(artwork_size)
            .maintain_aspect_ratio(false)
            .corner_radius(2.0),
    );
}

fn layout_playback_slider_and_track_info_ui(ui: &mut Ui, player: &mut Player, marquee: &mut MarqueeState, slider_width: f32) {
    let (mut position_as_secs, track_duration_as_secs) = if let Some(track) = &player.playing {
        let pos = player.backend.as_ref().map_or(0.0, |b| b.sink.get_pos().as_secs_f32());
        (pos, track.duration.as_secs_f32())
    } else {
        (0.0, 0.1) // We set to 0.1 so that when no track is playing, the slider is at the start.
    };

    StripBuilder::new(ui).sizes(Size::relative(1.0 / 2.0), 2).vertical(|mut strip| {
        strip.cell(|ui| {
            ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                display_playback_slider(ui, player, &mut position_as_secs, track_duration_as_secs, slider_width)
            });
        });
        strip.cell(|ui| {
            layout_marquee_and_playback_position_and_metadata(
                ui,
                player.playing.as_ref(),
                marquee,
                position_as_secs,
                track_duration_as_secs,
            )
        });
    });
}

fn display_playback_slider(ui: &mut Ui, player: &mut Player, position: &mut f32, duration: f32, slider_width: f32) {
    let previous_slider_width = ui.style_mut().spacing.slider_width;
    ui.style_mut().spacing.slider_width = slider_width;

    let playback_progress_slider = Slider::new(position, 0.0..=duration)
        .trailing_fill(true)
        .show_value(false)
        .step_by(1.0); // Step by 1 second.
    let response = ui.add(playback_progress_slider);

    let Some(backend) = &player.backend else {
        // TODO: is this correct?
        return;
    };

    if response.dragged() && player.paused_before_scrubbing.is_none() {
        player.paused_before_scrubbing = Some(backend.sink.is_paused());
        backend.sink.pause(); // Pause playback during scrubbing
    }

    if response.drag_stopped() {
        let new_position = Duration::from_secs_f32(*position);
        info!("Seeking to {}", format_duration_to_mmss(new_position));
        if let Err(e) = backend.sink.try_seek(new_position) {
            error!("Error seeking to new position: {:?}", e);
        }

        // Resume playback if the player was not paused before scrubbing
        if player.paused_before_scrubbing == Some(false) {
            backend.sink.play();
        }

        player.paused_before_scrubbing = None;
    }

    ui.style_mut().spacing.slider_width = previous_slider_width;
}

fn layout_marquee_and_playback_position_and_metadata(
    ui: &mut Ui,
    playing: Option<&Track>,
    marquee: &mut MarqueeState,
    position: f32,
    duration: f32,
) {
    // Placing the track info after the slider ensures that the playback position display is accurate. The seek operation is only
    // executed after the slider thumb is released. If we placed the display before, the current position would not be reflected.
    StripBuilder::new(ui)
        .size(Size::relative(7.0 / 10.0))
        .size(Size::relative(3.0 / 10.0))
        .horizontal(|mut hstrip| {
            hstrip.cell(|ui| display_track_marquee(ui, playing, marquee));
            hstrip.cell(|ui| {
                StripBuilder::new(ui).sizes(Size::relative(1.0 / 2.0), 2).vertical(|mut strip| {
                    strip.cell(|ui| {
                        ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
                            display_playback_time(ui, position, duration);
                        });
                    });

                    strip.cell(|ui| {
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            if let Some(playing) = playing {
                                display_track_metadata(ui, playing);
                            }
                        });
                    });
                });
            });
        });
}

fn display_track_marquee(ui: &mut Ui, maybe_track: Option<&Track>, marquee: &mut MarqueeState) {
    ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
        let mut title = "None";
        let mut artist = "None";
        let mut album = "None";
        let mut track_key = Some(PathBuf::from("none"));

        if let Some(playing_track) = maybe_track {
            title = playing_track.title.as_deref().unwrap_or("Unknown Title");
            artist = playing_track.artist.as_deref().unwrap_or("Unknown Artist");
            album = playing_track.album.as_deref().unwrap_or("Unknown Album");
            track_key = Some(playing_track.path.clone());
        }

        let padding = "        ";
        let text = format!("{} / {} / {}{}", title, artist, album, padding);
        let text_color = ui.visuals().text_color();
        let divider_color = ui.visuals().weak_text_color();
        let style = ui.style();

        let format_colored_marquee_text = |s: &str| {
            let mut job = text::LayoutJob::default();

            for (i, part) in s.split(" / ").enumerate() {
                if i > 0 {
                    job.append(" / ", 0.0, TextFormat::simple(TextStyle::Body.resolve(style), divider_color));
                }
                job.append(part, 0.0, TextFormat::simple(TextStyle::Body.resolve(style), text_color));
            }

            job
        };

        let galley = ui.ctx().fonts_mut(|fonts| fonts.layout_job(format_colored_marquee_text(&text)));

        let text_width = galley.size().x;
        let available_width = ui.available_width();
        let character_count = text.chars().count();
        let average_char_width = text_width / character_count as f32;
        let visible_chars = (available_width / average_char_width).floor() as usize;

        // If everything fits, no marquee needed
        if character_count <= visible_chars {
            ui.add(Label::new(format_colored_marquee_text(&text)).selectable(false).truncate());
            return;
        }

        // Reset marquee state if track changes.
        if marquee.track_key != track_key || marquee.track_key.is_none() {
            marquee.track_key = track_key.clone();
            marquee.position = 0.0;
            marquee.pause_timer = MARQUEE_PAUSE_DURATION;
        }

        let dt = ui.input(|i| i.stable_dt);

        if marquee.pause_timer > Duration::ZERO {
            marquee.pause_timer = marquee.pause_timer.saturating_sub(Duration::from_secs_f32(dt));
        } else {
            marquee.position += MARQUEE_SPEED * dt;

            if marquee.position >= character_count as f32 {
                marquee.position = 0.0;
                marquee.pause_timer = MARQUEE_PAUSE_DURATION;
            }
        }

        let display_text: String = text
            .chars()
            .chain(text.chars())
            .skip(marquee.position.floor() as usize)
            .take(visible_chars)
            .collect();
        ui.add(Label::new(format_colored_marquee_text(&display_text)).selectable(false).truncate());
    });
}

fn display_playback_time(ui: &mut Ui, position: f32, duration: f32) {
    let position = Duration::from_secs_f32(position);
    let track_duration = Duration::from_secs_f32(duration);
    let time_label_text = format!(
        "{} / {}",
        format_duration_to_mmss(position),
        format_duration_to_mmss(track_duration)
    );

    let time_label = unselectable_label(time_label_text);
    ui.add(time_label);
}

fn display_track_metadata(ui: &mut Ui, track: &Track) {
    let codec_string = file_type_name(track.codec);
    metadata_chip(ui, codec_string);

    ui.add_space(4.0);

    if let Some(sr) = track.sample_rate {
        let sample_rate_string = format!("{:.1} kHz", sr as f32 / 1000.0);
        metadata_chip(ui, &sample_rate_string);
    }
}

fn metadata_chip(ui: &mut Ui, text: &str) {
    Frame::new()
        .stroke(ui.visuals().widgets.noninteractive.bg_stroke)
        .corner_radius(4.0)
        .inner_margin(Margin::same(2))
        .outer_margin(Margin::same(2))
        .show(ui, |ui| {
            let label = Label::new(RichText::new(text).small().weak()).selectable(false);
            ui.add(label);
        });
}

fn display_visualizer(ui: &mut Ui, gem: &mut GemPlayer) {
    let dt = ui.input(|i| i.stable_dt);

    let targets = gem.player.visualizer.bands_receiver.try_iter().last();

    calculate_bands(&mut gem.player.visualizer.display_bands, targets.as_deref(), dt);

    let display_bands = &gem.player.visualizer.display_bands;

    let desired_height = ui.available_height() * 0.5;
    let bar_width = 10.0;
    let bar_gap = 4.0;
    let bar_radius = 1.0;
    let min_bar_height = 3.0;

    let num_bars = display_bands.len() as f32;
    let total_width = (num_bars * bar_width) + ((num_bars - 1.0) * bar_gap);

    let (rect, _) = ui.allocate_exact_size(vec2(total_width, desired_height), Sense::hover());

    let painter = ui.painter();
    for (i, &value) in display_bands.iter().enumerate() {
        let height = (value * rect.height()).max(min_bar_height);
        let x = rect.left() + i as f32 * (bar_width + bar_gap);
        let y = rect.bottom();

        let bar_rect = Rect::from_min_max(pos2(x, y - height), pos2(x + bar_width, y));
        painter.rect_filled(bar_rect, bar_radius, ui.visuals().text_color());
    }
}
