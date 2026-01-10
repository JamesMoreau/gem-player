use crate::{
    apply_theme,
    custom_window::custom_window,
    format_duration_to_hhmmss, format_duration_to_mmss, handle_dropped_file, maybe_play_next, maybe_play_previous, play_library,
    play_playlist,
    player::{
        clear_the_queue, enqueue, enqueue_next, get_audio_output_devices_and_names, move_to_position, mute_or_unmute, play_or_pause,
        remove_from_queue, switch_audio_devices, toggle_shuffle, Player,
    },
    playlist::{add_to_playlist, create, delete, remove_from_playlist, rename, Playlist, PlaylistRetrieval},
    spawn_folder_picker,
    track::{calculate_total_duration, file_type_name, open_file_location, sort, SortBy, SortOrder, TrackRetrieval},
    GemPlayer, Track, KEY_COMMANDS,
};
use eframe::egui::{
    containers::{self},
    include_image, pos2, text, vec2, Align, Button, Color32, ComboBox, Context, Direction, Frame, Id, Image, Label, Layout, Margin, Popup,
    PopupCloseBehavior, Rect, RectAlign, RichText, ScrollArea, Sense, Separator, Slider, TextEdit, TextFormat, TextStyle, TextureFilter,
    TextureOptions, ThemePreference, Ui, Vec2, WidgetText,
};
use egui_extras::{Size, StripBuilder, TableBuilder};
use egui_material_icons::icons;
use egui_notify::Toasts;
use fully_pub::fully_pub;
use log::{error, info};
use rodio::{Device, DeviceTrait};
use std::{
    path::{Path, PathBuf},
    time::Duration,
};
use strum::IntoEnumIterator;
use strum_macros::EnumIter;

#[derive(Debug, Clone, PartialEq, Eq, EnumIter)]
pub enum View {
    Library,
    Playlists,
    Queue,
    Settings,
}

#[fully_pub]
struct UIState {
    current_view: View,
    theme_preference: ThemePreference,
    marquee: MarqueeState,
    search: String,
    cached_artwork_track_path: Option<PathBuf>, // The uri pointing to the cached texture for the artwork of the currently playing track.
    volume_popup_is_open: bool,

    library: LibraryViewState,
    playlists: PlaylistsViewState,
    settings: SettingsViewState,

    library_and_playlists_are_loading: bool,

    toasts: Toasts,
}

const MARQUEE_SPEED: f32 = 5.0; // chars per second
const MARQUEE_PAUSE_DURATION: Duration = Duration::from_secs(2);

#[fully_pub]
struct MarqueeState {
    track_key: Option<PathBuf>, // We need to know when the track changes to reset.
    position: f32,
    pause_timer: Duration,
}

#[fully_pub]
struct LibraryViewState {
    selected_tracks: Vec<PathBuf>,
    cached_library: Option<Vec<Track>>,

    sort_by: SortBy,
    sort_order: SortOrder,
}

#[fully_pub]
struct PlaylistsViewState {
    selected_playlist_key: Option<PathBuf>, // None: no playlist is selected. Some: the path of the selected playlist.
    selected_tracks: Vec<PathBuf>,

    cached_playlist_tracks: Option<Vec<Track>>,

    rename_buffer: Option<String>, // If Some, the playlist pointed to by selected_track's name is being edited and a buffer for the new name.
    delete_modal_open: bool,       // The menu is open for selected_playlist_path.
}

#[fully_pub]
struct SettingsViewState {
    audio_output_devices_cache: Vec<(Device, String)>,
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
                strip.cell(|ui| navigation_bar(ui, gem));
            });
    });
}

fn switch_view(ui: &mut UIState, view: View) {
    info!("Switching to view: {:?}", view);
    ui.current_view = view;
}

fn drop_files_area_ui(ui: &mut Ui, gem: &mut GemPlayer) -> bool {
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

fn unselectable_label(text: impl Into<WidgetText>) -> Label {
    Label::new(text).selectable(false)
}

fn table_label(text: impl Into<String>, color: Option<Color32>) -> Label {
    let mut rich = RichText::new(text.into());
    if let Some(c) = color {
        rich = rich.color(c);
    }
    Label::new(rich).selectable(false).truncate()
}

/// Elide a path string to something like `/Users/user1/…/Music`
/// Keeps both start and end parts if the path is too long.
fn elide_path(path: &Path, max_len: usize) -> String {
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

fn get_count_and_duration_string_from_tracks(tracks: &[Track]) -> String {
    let duration = calculate_total_duration(tracks);
    let duration_string = format_duration_to_hhmmss(duration);
    format!("{} tracks / {}", tracks.len(), duration_string)
}

fn control_panel_ui(ui: &mut Ui, gem: &mut GemPlayer) {
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
                strip.cell(|ui| playback_controls_ui(ui, gem));

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
    ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
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
    });
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

    let placeholder = include_image!("../assets/icon.png");
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

// TODO: split this out into something like "calculate_bands()" and put it in visualizer.rs .
fn display_visualizer(ui: &mut Ui, gem: &mut GemPlayer) {
    let dt = ui.input(|i| i.stable_dt);
    let smoothing = 12.0;
    let step = smoothing * dt;

    let maybe_bands = gem.player.visualizer.bands_receiver.try_iter().last();
    let display_bands = &mut gem.player.visualizer.display_bands;
    let targets = maybe_bands.unwrap_or_else(|| vec![0.0; display_bands.len()]);

    for (bar, &raw_target) in display_bands.iter_mut().zip(&targets) {
        // clamp
        let target = if raw_target < *bar {
            (*bar - step).max(raw_target)
        } else {
            raw_target
        };

        // smoothing
        let alpha = 1.0 - (-step).exp();
        *bar += (target - *bar) * alpha;
    }

    let desired_height = ui.available_height() * 0.5;
    let bar_width = 10.0;
    let bar_gap = 4.0;
    let bar_radius = 1.0;
    let min_bar_height = 3.0;

    let num_bars = display_bands.len() as f32;
    let total_width = (num_bars * bar_width) + ((num_bars - 1.0) * bar_gap);

    let (rect, _response) = ui.allocate_exact_size(vec2(total_width, desired_height), Sense::hover());

    let painter = ui.painter();
    for (i, &value) in display_bands.iter().enumerate() {
        let height = (value * rect.height()).max(min_bar_height);
        let x = rect.left() + i as f32 * (bar_width + bar_gap);
        let y = rect.bottom();

        let bar_rect = Rect::from_min_max(pos2(x, y - height), pos2(x + bar_width, y));
        painter.rect_filled(bar_rect, bar_radius, ui.visuals().text_color());
    }
}

fn playing_indicator(ui: &mut Ui) {
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

fn library_view(ui: &mut Ui, gem: &mut GemPlayer) {
    if gem.ui.library_and_playlists_are_loading {
        Frame::new()
            .outer_margin(Margin::symmetric((ui.available_width() * (1.0 / 4.0)) as i8, 32))
            .show(ui, |ui| {
                ui.vertical_centered(|ui| {
                    ui.spinner();
                });
            });

        return;
    }

    if gem.library.is_empty() {
        Frame::new()
            .outer_margin(Margin::symmetric((ui.available_width() * (1.0 / 4.0)) as i8, 32))
            .show(ui, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add(unselectable_label(
                        "The library is empty. Try adding your music directory in the settings.",
                    ));
                });
            });

        return;
    }

    let cached_library = gem.ui.library.cached_library.get_or_insert_with(|| {
        // Regenerate the cache.

        let mut filtered_and_sorted: Vec<Track> = gem
            .library
            .iter()
            .filter(|track| {
                let search_lower = gem.ui.search.to_lowercase();

                let matches_search = |field: &Option<String>| {
                    field
                        .as_ref()
                        .map(|text| text.to_lowercase().contains(&search_lower))
                        .unwrap_or(false)
                };

                matches_search(&track.title) || matches_search(&track.artist) || matches_search(&track.album)
            })
            .cloned()
            .collect();

        sort(&mut filtered_and_sorted, gem.ui.library.sort_by, gem.ui.library.sort_order);

        filtered_and_sorted
    });

    let header_labels = [icons::ICON_MUSIC_NOTE, icons::ICON_ARTIST, icons::ICON_ALBUM, icons::ICON_HOURGLASS];

    let available_width = ui.available_width();
    let time_width = 64.0;
    let more_width = 48.0;
    let remaining_width = available_width - time_width - more_width;
    let title_width = remaining_width * 0.5;
    let artist_width = remaining_width * 0.25;
    let album_width = remaining_width * 0.25;

    // Since we are setting the widths of the table columns manually by dividing up the available width,
    // if we leave the default item spacing, the width taken up by the table will be greater than the available width,
    // causing the right side of the table to be cut off by the window.
    ui.spacing_mut().item_spacing.x = 0.0;

    // Used to determine if selection should be extended.
    let shift_is_pressed = ui.input(|i| i.modifiers.shift);

    let mut should_play_library = None;
    let mut context_menu_action = None;

    let playing_color = ui.visuals().selection.bg_fill;

    TableBuilder::new(ui)
        .striped(true)
        .sense(Sense::click())
        .cell_layout(Layout::left_to_right(Align::Center))
        .column(egui_extras::Column::exact(title_width))
        .column(egui_extras::Column::exact(artist_width))
        .column(egui_extras::Column::exact(album_width))
        .column(egui_extras::Column::exact(time_width))
        .column(egui_extras::Column::exact(more_width))
        .header(16.0, |mut header| {
            for (i, h) in header_labels.iter().enumerate() {
                header.col(|ui| {
                    if i == 0 {
                        ui.add_space(16.0);
                    }
                    ui.add(unselectable_label(RichText::new(*h).strong()));
                });
            }
        })
        .body(|body| {
            body.rows(26.0, cached_library.len(), |mut row| {
                let track = &cached_library[row.index()];
                let track_is_playing = gem.player.playing.as_ref().is_some_and(|t| t == track);

                let row_is_selected = gem.ui.library.selected_tracks.contains(&track.path);
                row.set_selected(row_is_selected);

                let text_color = if track_is_playing && !row_is_selected {
                    Some(playing_color)
                } else {
                    None
                };

                row.col(|ui| {
                    ui.add_space(16.0);
                    let label = table_label(track.title.as_deref().unwrap_or("-"), text_color);
                    ui.add(label);
                });

                row.col(|ui| {
                    ui.add_space(4.0);
                    let label = table_label(track.artist.as_deref().unwrap_or("-"), text_color);
                    ui.add(label);
                });

                row.col(|ui| {
                    ui.add_space(4.0);
                    let label = table_label(track.album.as_deref().unwrap_or("-"), text_color);
                    ui.add(label);
                });

                row.col(|ui| {
                    ui.add_space(4.0);
                    let duration_string = format_duration_to_mmss(track.duration);
                    let label = table_label(duration_string, text_color);
                    ui.add(label);
                });

                let rest_of_row_is_hovered = row.response().hovered();
                let mut more_cell_contains_pointer = false;
                row.col(|ui| {
                    ui.add_space(8.0);

                    more_cell_contains_pointer = ui.rect_contains_pointer(ui.max_rect());
                    let should_show_more_button: bool = rest_of_row_is_hovered || more_cell_contains_pointer || row_is_selected;

                    if should_show_more_button {
                        let more_button = Button::new(icons::ICON_MORE_HORIZ);
                        let response = ui.add(more_button).on_hover_text("More");

                        if response.clicked() {
                            gem.ui.library.selected_tracks.push(track.path.clone());
                        }

                        Popup::menu(&response).show(|ui| {
                            let selected_tracks_count = gem.ui.library.selected_tracks.len();
                            let maybe_action = library_context_menu_ui(ui, selected_tracks_count, &gem.playlists);
                            if let Some(action) = maybe_action {
                                context_menu_action = Some(action);
                            }
                        });
                    } else if track_is_playing {
                        playing_indicator(ui);
                    }
                });

                let response = row.response();

                let secondary_clicked = response.secondary_clicked();
                let primary_clicked = response.clicked() || response.double_clicked();
                let already_selected = gem.ui.library.selected_tracks.contains(&track.path);

                if primary_clicked || secondary_clicked {
                    let selected_tracks = &mut gem.ui.library.selected_tracks;

                    if secondary_clicked {
                        if selected_tracks.is_empty() || !already_selected {
                            selected_tracks.clear();
                            selected_tracks.push(track.path.clone());
                        }
                    } else if shift_is_pressed && !selected_tracks.is_empty() {
                        let last_selected = selected_tracks.last().unwrap();
                        let last_index = cached_library.iter().position(|t| &t.path == last_selected).unwrap();
                        let clicked_index = cached_library.iter().position(|t| t.path == track.path).unwrap();

                        let start = last_index.min(clicked_index);
                        let end = last_index.max(clicked_index);
                        for t in &cached_library[start..=end] {
                            if !selected_tracks.contains(&t.path) {
                                selected_tracks.push(t.path.clone());
                            }
                        }
                    } else {
                        selected_tracks.clear();
                        selected_tracks.push(track.path.clone());
                    }
                }

                if response.double_clicked() {
                    should_play_library = Some(track.clone());
                }

                Popup::context_menu(&response).show(|ui| {
                    let selected_tracks_count = gem.ui.library.selected_tracks.len();
                    let maybe_action = library_context_menu_ui(ui, selected_tracks_count, &gem.playlists);
                    if let Some(action) = maybe_action {
                        context_menu_action = Some(action);
                    }
                });
            });
        });

    // Perform actions AFTER rendering the table to avoid borrow checker issues that come with mutating state inside closures.

    if let Some(track) = should_play_library {
        if let Err(e) = play_library(gem, Some(&track)) {
            error!("{}", e);
            gem.ui.toasts.error("Error playing from library");
        }
    }

    if let Some(action) = context_menu_action {
        handle_library_context_menu_action(gem, action);
    }
}

#[derive(Debug)]
enum LibraryContextMenuAction {
    AddToPlaylist(PathBuf),
    EnqueueNext,
    Enqueue,
    OpenFileLocation,
}

fn handle_library_context_menu_action(gem: &mut GemPlayer, action: LibraryContextMenuAction) {
    match action {
        LibraryContextMenuAction::AddToPlaylist(playlist_key) => {
            if gem.ui.library.selected_tracks.is_empty() {
                error!("No track(s) selected for adding to playlist.");
                return;
            }

            let playlist = gem.playlists.get_by_path_mut(&playlist_key);

            let mut added_count = 0;
            for track_key in &gem.ui.library.selected_tracks {
                let track = gem.library.get_by_path(track_key);
                if let Err(e) = add_to_playlist(playlist, track.clone()) {
                    error!("Failed to add track to playlist: {}", e);
                } else {
                    added_count += 1;
                }
            }

            gem.ui.playlists.cached_playlist_tracks = None;

            if added_count > 0 {
                let message = format!("Added {} track(s) to playlist '{}'.", added_count, playlist.name);
                info!("{}", message);
                gem.ui.toasts.success(message);
            } else {
                gem.ui.toasts.error("No tracks were added.");
            }
        }
        LibraryContextMenuAction::EnqueueNext => {
            if gem.ui.library.selected_tracks.is_empty() {
                error!("No track(s) selected for enqueue next");
                return;
            }

            for track_key in &gem.ui.library.selected_tracks {
                let track = gem.library.get_by_path(track_key);
                enqueue_next(&mut gem.player, track.clone());
            }
        }
        LibraryContextMenuAction::Enqueue => {
            if gem.ui.library.selected_tracks.is_empty() {
                error!("No track(s) selected for enqueue");
                return;
            }

            for track_key in &gem.ui.library.selected_tracks {
                let track = gem.library.get_by_path(track_key);
                enqueue(&mut gem.player, track.clone());
            }
        }
        LibraryContextMenuAction::OpenFileLocation => {
            // We just grab the first since we cannot reveal multiple file locations.
            let Some(first_track_key) = gem.ui.library.selected_tracks.first() else {
                error!("No track(s) were selected for opening file location.");
                return;
            };

            let first_track = gem.library.get_by_path(first_track_key);
            if let Err(e) = open_file_location(first_track) {
                error!("Failed to open track location: {}", e);
            } else {
                info!("Opening track location: {}", first_track.path.display());
            }
        }
    }
}

fn library_context_menu_ui(ui: &mut Ui, selected_tracks_count: usize, playlists: &[Playlist]) -> Option<LibraryContextMenuAction> {
    let modal_width = 220.0;
    ui.set_width(modal_width);

    ui.add_enabled(false, Label::new(format!("{} track(s) selected", selected_tracks_count)));

    ui.separator();

    let mut action = None;

    let add_to_playlists_enabled = !playlists.is_empty();
    ui.add_enabled_ui(add_to_playlists_enabled, |ui| {
        ui.menu_button("Add to Playlist", |ui| {
            ui.set_min_width(modal_width);

            ScrollArea::vertical().max_height(164.0).show(ui, |ui| {
                for playlist in playlists.iter() {
                    let response = ui.button(&playlist.name);
                    if response.clicked() {
                        action = Some(LibraryContextMenuAction::AddToPlaylist(playlist.m3u_path.clone()));
                    }
                }
            });
        });
    });

    ui.separator();

    let response = ui.button(format!("Play Next {}", icons::ICON_PLAY_ARROW));
    if response.clicked() {
        action = Some(LibraryContextMenuAction::EnqueueNext);
    }

    let response = ui.button(format!("Add to Queue {}", icons::ICON_QUEUE_MUSIC));
    if response.clicked() {
        action = Some(LibraryContextMenuAction::Enqueue);
    }

    ui.separator();

    let response = ui.button(format!("Open File Location {}", icons::ICON_FOLDER));
    if response.clicked() {
        action = Some(LibraryContextMenuAction::OpenFileLocation);
    }

    action
}

fn queue_view(ui: &mut Ui, player: &mut Player) {
    if player.queue.is_empty() {
        Frame::new()
            .outer_margin(Margin::symmetric((ui.available_width() * (1.0 / 4.0)) as i8, 32))
            .show(ui, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add(unselectable_label("The queue is empty."));
                });
            });

        return;
    }

    let header_labels = [
        icons::ICON_TAG,
        icons::ICON_MUSIC_NOTE,
        icons::ICON_ARTIST,
        icons::ICON_ALBUM,
        icons::ICON_HOURGLASS,
        "",
    ];

    let available_width = ui.available_width();
    let position_width = 64.0;
    let time_width = 64.0;
    let actions_width = 80.0;
    let remaining_width = available_width - position_width - time_width - actions_width;
    let title_width = remaining_width * (2.0 / 4.0);
    let artist_width = remaining_width * (1.0 / 4.0);
    let album_width = remaining_width * (1.0 / 4.0);

    ui.spacing_mut().item_spacing.x = 0.0; // See comment in library_view() for why we set item_spacing to 0.

    // We only operate on the queue after we are done iterating over it.
    let mut to_be_removed = None;
    let mut to_be_moved_to_front = None;

    TableBuilder::new(ui)
        .striped(true)
        .sense(Sense::hover())
        .cell_layout(Layout::left_to_right(Align::Center))
        .column(egui_extras::Column::exact(position_width))
        .column(egui_extras::Column::exact(title_width))
        .column(egui_extras::Column::exact(artist_width))
        .column(egui_extras::Column::exact(album_width))
        .column(egui_extras::Column::exact(time_width))
        .column(egui_extras::Column::exact(actions_width))
        .header(16.0, |mut header| {
            for (i, h) in header_labels.iter().enumerate() {
                header.col(|ui| {
                    if i == 0 {
                        ui.add_space(16.0);
                    }
                    ui.add(unselectable_label(RichText::new(*h).strong()));
                });
            }
        })
        .body(|body| {
            body.rows(26.0, player.queue.len(), |mut row| {
                let index = row.index();
                let track = &player.queue[index];

                row.col(|ui| {
                    ui.add_space(16.0);
                    ui.add(unselectable_label(format!("{}", index + 1)));
                });

                row.col(|ui| {
                    ui.add_space(4.0);
                    ui.add(unselectable_label(track.title.as_deref().unwrap_or("-")));
                });

                row.col(|ui| {
                    ui.add_space(4.0);
                    ui.add(unselectable_label(track.artist.as_deref().unwrap_or("-")));
                });

                row.col(|ui| {
                    ui.add_space(4.0);
                    ui.add(unselectable_label(track.album.as_deref().unwrap_or("-")));
                });

                row.col(|ui| {
                    ui.add_space(4.0);
                    let duration_string = format_duration_to_mmss(track.duration);
                    ui.add(unselectable_label(duration_string));
                });

                // We only display the actions column buttons if the row is hovered. There is a chicken and egg problem here.
                // We need to know if the row is hovered before we display the actions column buttons. So, we check if
                // either the row response (of the previous cells) or the actions column cell contains the pointer.
                let row_is_hovered = row.response().hovered();
                let mut actions_cell_contains_pointer = false;
                row.col(|ui| {
                    actions_cell_contains_pointer = ui.rect_contains_pointer(ui.max_rect());
                    let should_show_action_buttons = row_is_hovered || actions_cell_contains_pointer;

                    ui.add_space(8.0);

                    let response = ui.add_visible(should_show_action_buttons, Button::new(icons::ICON_ARROW_UPWARD));
                    if response.clicked() {
                        to_be_moved_to_front = Some(index);
                    }

                    ui.add_space(8.0);

                    let response = ui.add_visible(should_show_action_buttons, Button::new(icons::ICON_CLOSE));
                    if response.clicked() {
                        to_be_removed = Some(index);
                    }
                });
            });
        });

    if let Some(index) = to_be_removed {
        remove_from_queue(player, index);
    }

    if let Some(index) = to_be_moved_to_front {
        move_to_position(player, index, 0);
    }
}

fn playlists_view(ui: &mut Ui, gem: &mut GemPlayer) {
    if gem.ui.library_and_playlists_are_loading {
        Frame::new()
            .outer_margin(Margin::symmetric((ui.available_width() * (1.0 / 4.0)) as i8, 32))
            .show(ui, |ui| {
                ui.vertical_centered(|ui| {
                    ui.spinner();
                });
            });

        return;
    }

    if gem.library_directory.is_none() {
        Frame::new()
            .outer_margin(Margin::symmetric((ui.available_width() * (1.0 / 4.0)) as i8, 32))
            .show(ui, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add(unselectable_label("Try adding your music directory in the settings"));
                });
            });

        return;
    };

    delete_playlist_modal(ui, gem);

    let size = ui.available_size();
    let playlists_width = size.x * (1.0 / 4.0);

    ui.spacing_mut().item_spacing.x = 0.0; // See comment in library_view as to why we do this.

    StripBuilder::new(ui)
        .size(Size::exact(playlists_width))
        .size(Size::exact(6.0))
        .size(Size::remainder())
        .horizontal(|mut strip| {
            strip.cell(|ui| {
                let width = ui.available_width();
                TableBuilder::new(ui)
                    .striped(true)
                    .sense(Sense::click())
                    .cell_layout(Layout::left_to_right(Align::Center))
                    .column(egui_extras::Column::exact(width))
                    .header(36.0, |mut header| {
                        header.col(|ui| {
                            containers::Sides::new().height(ui.available_height()).show(
                                ui,
                                |ui| {
                                    ui.add_space(8.0);
                                    ui.add(unselectable_label(RichText::new("Playlists").heading().strong()));
                                },
                                |ui| {
                                    ui.add_space(8.0);

                                    let add_button = Button::new(icons::ICON_ADD);
                                    let response = ui.add(add_button).on_hover_text("Add playlist");
                                    if response.clicked() {
                                        let directory = gem.library_directory.as_ref().unwrap(); // We checked earlier so this is safe.
                                        let new_playlist_name = format!("Playlist {}", gem.playlists.len() + 1);
                                        let result = create(new_playlist_name, directory);
                                        match result {
                                            Err(e) => {
                                                let error_message = format!("Failed to create: {}.", e);
                                                error!("{}", &error_message);
                                                gem.ui.toasts.error(&error_message);
                                            }
                                            Ok(new_playlist) => {
                                                info!("Created and saved {} to {:?}", &new_playlist.name, &new_playlist.m3u_path);
                                                gem.playlists.push(new_playlist);
                                            }
                                        }
                                    }
                                },
                            );
                        });
                    })
                    .body(|body| {
                        body.rows(36.0, gem.playlists.len(), |mut row| {
                            let playlist = &mut gem.playlists[row.index()];

                            if let Some(playlist_key) = &gem.ui.playlists.selected_playlist_key {
                                let playlist_is_selected = playlist.m3u_path == *playlist_key;
                                row.set_selected(playlist_is_selected);
                            }

                            row.col(|ui| {
                                ui.add_space(8.0);
                                ui.add(unselectable_label(&playlist.name));
                            });

                            let response = row.response();
                            if response.clicked() {
                                info!("Selected playlist: {}", playlist.name);
                                gem.ui.playlists.selected_playlist_key = Some(playlist.m3u_path.clone());

                                gem.ui.playlists.rename_buffer = None; // In case we were currently editing
                                gem.ui.playlists.cached_playlist_tracks = None;
                                gem.ui.playlists.selected_tracks.clear();
                            }
                        });
                    });
            });

            strip.cell(|ui| {
                ui.add(Separator::default().vertical());
            });

            strip.cell(|ui| playlist_ui(ui, gem));
        });
}

fn delete_playlist_modal(ui: &mut Ui, gem: &mut GemPlayer) {
    if !gem.ui.playlists.delete_modal_open {
        return;
    }

    let Some(playlist_key) = gem.ui.playlists.selected_playlist_key.clone() else {
        error!("The delete playlist is open but no playlist is selected.");
        return;
    };

    let mut cancel_clicked = false;
    let mut confirm_clicked = false;

    let modal = containers::Modal::new(Id::new("delete_playlist_modal"))
        .backdrop_color(Color32::TRANSPARENT)
        .show(ui.ctx(), |ui| {
            ui.set_width(200.0);
            Frame::new().outer_margin(Margin::same(4)).show(ui, |ui| {
                let label = unselectable_label(RichText::new("Are you sure you want to delete this playlist?").heading());
                ui.add(label);

                ui.separator();

                containers::Sides::new().show(
                    ui,
                    |ui| {
                        let response = ui.button(format!("\t{}\t", icons::ICON_CLOSE));
                        if response.clicked() {
                            cancel_clicked = true;
                        }
                    },
                    |ui| {
                        let response = ui.button(format!("\t{}\t", icons::ICON_CHECK));
                        if response.clicked() {
                            confirm_clicked = true;

                            let result = delete(&playlist_key, &mut gem.playlists);
                            if let Err(e) = result {
                                error!("{}", e);
                            } else {
                                let message =
                                    "Playlist was deleted successfully. If this was a mistake, the m3u file can be found in the trash.";
                                info!("{}", message);
                                gem.ui.toasts.success(message);
                                gem.ui.playlists.selected_playlist_key = None;
                            }
                        }
                    },
                );
            });
        });

    if confirm_clicked || cancel_clicked || modal.should_close() {
        gem.ui.playlists.delete_modal_open = false;
    }
}

fn playlist_ui(ui: &mut Ui, gem: &mut GemPlayer) {
    let Some(playlist_key) = gem.ui.playlists.selected_playlist_key.clone() else {
        return; // No playlist selected, do nothing
    };

    StripBuilder::new(ui)
        .size(Size::exact(64.0))
        .size(Size::remainder())
        .vertical(|mut strip| {
            strip.cell(|ui| {
                Frame::new().fill(ui.visuals().faint_bg_color).show(ui, |ui| {
                    if let Some(name_buffer) = &mut gem.ui.playlists.rename_buffer {
                        // Editing mode
                        let mut discard_clicked = false;
                        let mut save_clicked = false;

                        containers::Sides::new().height(ui.available_height()).show(
                            ui,
                            |ui| {
                                ui.add_space(16.0);
                                let name_edit = TextEdit::singleline(name_buffer).char_limit(50);
                                ui.add(name_edit);
                            },
                            |ui| {
                                ui.add_space(16.0);

                                let cancel_button = Button::new(icons::ICON_CANCEL);
                                let response = ui.add(cancel_button).on_hover_text("Discard");
                                discard_clicked = response.clicked();

                                ui.add_space(8.0);

                                let confirm_button = Button::new(icons::ICON_SAVE);
                                let response = ui.add(confirm_button).on_hover_text("Save");
                                save_clicked = response.clicked();
                            },
                        );

                        if save_clicked {
                            let name_buffer_clone = name_buffer.to_owned();

                            let playlist = &mut gem.playlists.get_by_path_mut(&playlist_key);
                            let result = rename(playlist, name_buffer_clone);
                            match result {
                                Err(e) => {
                                    let message = format!("Error renaming playlist: {}", e);
                                    error!("{}", message);
                                    gem.ui.toasts.error(message);
                                }
                                Ok(_) => {
                                    // Update the selected playlist with the new path so that we remain selected.
                                    gem.ui.playlists.selected_playlist_key = Some(playlist.m3u_path.clone());
                                }
                            }

                            gem.ui.playlists.rename_buffer = None;
                        }

                        if discard_clicked {
                            gem.ui.playlists.rename_buffer = None;
                        }
                    } else {
                        // Not edit mode
                        let strip_contains_pointer = ui.rect_contains_pointer(ui.max_rect());
                        let mut play_clicked = false;
                        let mut delete_clicked = false;
                        let mut edit_clicked = false;

                        containers::Sides::new().height(ui.available_height()).show(
                            ui,
                            |ui| {
                                ui.add_space(16.0);

                                let name = &gem.playlists.get_by_path(&playlist_key).name;
                                ui.add(unselectable_label(RichText::new(name).heading().strong()));

                                if strip_contains_pointer {
                                    ui.add_space(16.0);

                                    let play = Button::new(icons::ICON_PLAY_ARROW);
                                    let response = ui.add(play);
                                    play_clicked = response.clicked();
                                }
                            },
                            |ui| {
                                if !strip_contains_pointer {
                                    return;
                                }

                                ui.add_space(16.0);

                                let delete_button = Button::new(icons::ICON_DELETE);
                                let response = ui.add(delete_button).on_hover_text("Delete");
                                delete_clicked = response.clicked();

                                ui.add_space(8.0);

                                let edit_name_button = Button::new(icons::ICON_EDIT);
                                let response = ui.add(edit_name_button).on_hover_text("Edit name");
                                edit_clicked = response.clicked();
                            },
                        );

                        // We have to do this pattern since we want to access gem across
                        // the two captures used by containers::Sides.
                        if play_clicked {
                            let path = &gem.playlists.get_by_path(&playlist_key).m3u_path;
                            if let Err(e) = play_playlist(gem, &path.clone(), None) {
                                error!("{}", e);
                                gem.ui.toasts.error("Error playing from playlist");
                            }
                        }

                        if delete_clicked {
                            info!("Opening delete playlist modal");
                            gem.ui.playlists.delete_modal_open = true;
                        }

                        if edit_clicked {
                            let playlist = &mut gem.playlists.get_by_path(&playlist_key);
                            info!("Editing playlist name: {}", playlist.name);
                            gem.ui.playlists.rename_buffer = Some(playlist.name.clone());
                        }
                    }
                });
            });

            strip.cell(|ui| playlist_tracks_ui(ui, gem));
        });
}

fn playlist_tracks_ui(ui: &mut Ui, gem: &mut GemPlayer) {
    let Some(playlist_key) = gem.ui.playlists.selected_playlist_key.clone() else {
        Frame::new()
            .outer_margin(Margin::symmetric((ui.available_width() * (1.0 / 4.0)) as i8, 32))
            .show(ui, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add(unselectable_label("No playlist selected"));
                });
            });

        return;
    };

    let playlist_length = gem.playlists.get_by_path(&playlist_key).tracks.len();
    if playlist_length == 0 {
        Frame::new()
            .outer_margin(Margin::symmetric((ui.available_width() * (1.0 / 4.0)) as i8, 32))
            .show(ui, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add(unselectable_label("The playlist is empty."));
                });
            });

        return;
    }

    let cached_playlist_tracks = gem.ui.playlists.cached_playlist_tracks.get_or_insert_with(|| {
        // Regenerate the cache.

        let filtered: Vec<Track> = gem
            .playlists
            .get_by_path(&playlist_key)
            .tracks
            .iter()
            .filter(|track| {
                let search_lower = gem.ui.search.to_lowercase();

                let matches_search = |field: &Option<String>| {
                    field
                        .as_ref()
                        .map(|text| text.to_lowercase().contains(&search_lower))
                        .unwrap_or(false)
                };

                matches_search(&track.title) || matches_search(&track.artist) || matches_search(&track.album)
            })
            .cloned()
            .collect();

        filtered
    });

    let header_labels = [
        icons::ICON_TAG,
        icons::ICON_MUSIC_NOTE,
        icons::ICON_ARTIST,
        icons::ICON_ALBUM,
        icons::ICON_HOURGLASS,
    ];

    let available_width = ui.available_width();
    let position_width = 64.0;
    let time_width = 64.0;
    let more_width = 48.0;
    let remaining_width = available_width - position_width - time_width - more_width;
    let title_width = remaining_width * (2.0 / 4.0);
    let artist_width = remaining_width * (1.0 / 4.0);
    let album_width = remaining_width * (1.0 / 4.0);

    ui.spacing_mut().item_spacing.x = 0.0; // See comment in library_view() for why we set item_spacing to 0.

    // Used to determine if selection should be extended.
    let shift_is_pressed = ui.input(|i| i.modifiers.shift);

    let mut should_play_playlist = None;
    let mut context_menu_action = None;

    let playing_color = ui.visuals().selection.bg_fill;

    TableBuilder::new(ui)
        .striped(true)
        .sense(Sense::click())
        .cell_layout(Layout::left_to_right(Align::Center))
        .column(egui_extras::Column::exact(position_width))
        .column(egui_extras::Column::exact(title_width))
        .column(egui_extras::Column::exact(artist_width))
        .column(egui_extras::Column::exact(album_width))
        .column(egui_extras::Column::exact(time_width))
        .column(egui_extras::Column::exact(more_width))
        .header(16.0, |mut header| {
            for (i, h) in header_labels.iter().enumerate() {
                header.col(|ui| {
                    if i == 0 {
                        ui.add_space(16.0);
                    }
                    ui.add(unselectable_label(RichText::new(*h).strong()));
                });
            }
        })
        .body(|body| {
            body.rows(26.0, cached_playlist_tracks.len(), |mut row| {
                let index = row.index();
                let track = &cached_playlist_tracks[index];
                let track_is_playing = gem.player.playing.as_ref().is_some_and(|t| t == track);

                let row_is_selected = gem.ui.playlists.selected_tracks.contains(&track.path);
                row.set_selected(row_is_selected);

                let text_color = if track_is_playing && !row_is_selected {
                    Some(playing_color)
                } else {
                    None
                };

                row.col(|ui| {
                    ui.add_space(16.0);
                    let label = table_label(format!("{}", index + 1), text_color);
                    ui.add(label);
                });

                row.col(|ui| {
                    ui.add_space(4.0);
                    let label = table_label(track.title.as_deref().unwrap_or("-"), text_color);
                    ui.add(label);
                });

                row.col(|ui| {
                    ui.add_space(4.0);
                    let label = table_label(track.artist.as_deref().unwrap_or("-"), text_color);
                    ui.add(label);
                });

                row.col(|ui| {
                    ui.add_space(4.0);
                    let label = table_label(track.album.as_deref().unwrap_or("-"), text_color);
                    ui.add(label);
                });

                row.col(|ui| {
                    ui.add_space(4.0);
                    let duration_string = format_duration_to_mmss(track.duration);
                    let label = table_label(duration_string, text_color);
                    ui.add(label);
                });

                let rest_of_row_is_hovered = row.response().hovered();
                let mut more_cell_contains_pointer = false;
                row.col(|ui| {
                    ui.add_space(8.0);

                    more_cell_contains_pointer = ui.rect_contains_pointer(ui.max_rect());
                    let should_show_more_button = rest_of_row_is_hovered || more_cell_contains_pointer || row_is_selected;

                    if should_show_more_button {
                        let more_button = Button::new(icons::ICON_MORE_HORIZ);
                        let response = ui.add(more_button).on_hover_text("More");

                        Popup::menu(&response).show(|ui| {
                            let selected_tracks_count = gem.ui.playlists.selected_tracks.len();
                            let maybe_action = playlist_context_menu_ui(ui, selected_tracks_count);
                            if let Some(action) = maybe_action {
                                context_menu_action = Some(action);
                            }
                        });
                    } else if track_is_playing {
                        playing_indicator(ui);
                    }
                });

                let response = row.response();

                let secondary_clicked = response.secondary_clicked();
                let primary_clicked = response.clicked() || response.double_clicked();
                let already_selected = gem.ui.playlists.selected_tracks.contains(&track.path);

                if primary_clicked || secondary_clicked {
                    let selected_tracks = &mut gem.ui.playlists.selected_tracks;

                    if secondary_clicked {
                        if selected_tracks.is_empty() || !already_selected {
                            selected_tracks.clear();
                            selected_tracks.push(track.path.clone());
                        }
                    } else if shift_is_pressed && !selected_tracks.is_empty() {
                        let last_selected_track = selected_tracks.last().unwrap();
                        let last_index = cached_playlist_tracks.iter().position(|t| &t.path == last_selected_track).unwrap();
                        let clicked_index = cached_playlist_tracks.iter().position(|t| t.path == track.path).unwrap();

                        let start = last_index.min(clicked_index);
                        let end = last_index.max(clicked_index);
                        for t in &cached_playlist_tracks[start..=end] {
                            if !selected_tracks.contains(&t.path) {
                                selected_tracks.push(t.path.clone());
                            }
                        }
                    } else {
                        selected_tracks.clear();
                        selected_tracks.push(track.path.clone());
                    }
                }

                if response.double_clicked() {
                    should_play_playlist = Some((playlist_key.clone(), track.path.clone()));
                }

                Popup::context_menu(&response).show(|ui| {
                    let selected_tracks_count = gem.ui.playlists.selected_tracks.len();
                    let maybe_action = playlist_context_menu_ui(ui, selected_tracks_count);
                    if let Some(action) = maybe_action {
                        context_menu_action = Some(action);
                    }
                });
            });
        });

    if let Some(action) = context_menu_action {
        handle_playlist_context_menu_action(gem, action, &playlist_key);
    }

    if let Some((playlist_key, track_key)) = should_play_playlist {
        if let Err(e) = play_playlist(gem, &playlist_key, Some(&track_key)) {
            error!("{}", e);
            gem.ui.toasts.error("Error playing from playlist");
        }
    }
}

#[derive(Debug)]
enum PlaylistContextMenuAction {
    RemoveFromPlaylist,
    EnqueueNext,
    Enqueue,
    OpenFileLocation,
}

fn handle_playlist_context_menu_action(gem: &mut GemPlayer, action: PlaylistContextMenuAction, playlist_key: &Path) {
    match action {
        PlaylistContextMenuAction::RemoveFromPlaylist => {
            let Some(playlist_key) = &gem.ui.playlists.selected_playlist_key else {
                error!("No playlist selected for removing track from playlist.");
                return;
            };

            if gem.ui.playlists.selected_tracks.is_empty() {
                error!("No track(s) selected for removing track from playlist next.");
                return;
            };

            let playlist = gem.playlists.get_by_path_mut(playlist_key);

            let mut added_count = 0;
            for track_key in &gem.ui.playlists.selected_tracks {
                if let Err(e) = remove_from_playlist(playlist, track_key) {
                    error!("Failed to remove track from playlist: {}", e);
                } else {
                    added_count += 1;
                }
            }

            gem.ui.playlists.cached_playlist_tracks = None;

            if added_count > 0 {
                let message = format!("Removed {} track(s) from playlist '{}'", added_count, playlist.name);
                info!("{}", message);
                gem.ui.toasts.success(message);
            } else {
                gem.ui.toasts.error("No tracks were removed.");
            }
        }
        PlaylistContextMenuAction::EnqueueNext => {
            if gem.ui.playlists.selected_tracks.is_empty() {
                error!("No track(s) selected for enqueue next");
                return;
            };

            let playlist = gem.playlists.get_by_path(playlist_key);
            for track_key in &gem.ui.playlists.selected_tracks {
                let track = playlist.tracks.get_by_path(track_key);
                enqueue_next(&mut gem.player, track.clone());
            }
        }
        PlaylistContextMenuAction::Enqueue => {
            if gem.ui.playlists.selected_tracks.is_empty() {
                error!("No track(s) selected for enqueue");
                return;
            };

            let playlist = gem.playlists.get_by_path(playlist_key);
            for track_key in &gem.ui.playlists.selected_tracks {
                let track = playlist.tracks.get_by_path(track_key);
                enqueue(&mut gem.player, track.clone());
            }
        }
        PlaylistContextMenuAction::OpenFileLocation => {
            // We take the first one since we cannot open / reveal multiple tracks.
            let Some(first_track_key) = gem.ui.playlists.selected_tracks.first() else {
                error!("No track(s) selected for opening file location");
                return;
            };

            let playlist = gem.playlists.get_by_path(playlist_key);
            let first_track = playlist.tracks.get_by_path(first_track_key);
            if let Err(e) = open_file_location(first_track) {
                error!("Failed to open track location: {}", e);
            } else {
                info!("Opening track location: {}", first_track.path.display());
            }
        }
    }
}

fn playlist_context_menu_ui(ui: &mut Ui, selected_tracks_count: usize) -> Option<PlaylistContextMenuAction> {
    let modal_width = 220.0;
    ui.set_width(modal_width);

    ui.add_enabled(false, Label::new(format!("{} track(s) selected", selected_tracks_count)));

    ui.separator();

    let mut action = None;

    let response = ui.button(format!("{} Remove from Playlist", icons::ICON_DELETE));
    if response.clicked() {
        action = Some(PlaylistContextMenuAction::RemoveFromPlaylist);
    }

    ui.separator();

    let response = ui.button(format!("{} Play Next", icons::ICON_PLAY_ARROW));
    if response.clicked() {
        action = Some(PlaylistContextMenuAction::EnqueueNext);
    }

    let response = ui.button(format!("{} Add to Queue", icons::ICON_ADD));
    if response.clicked() {
        action = Some(PlaylistContextMenuAction::Enqueue);
    }

    ui.separator();

    let response = ui.button(format!("{} Open File Location", icons::ICON_FOLDER));
    if response.clicked() {
        action = Some(PlaylistContextMenuAction::OpenFileLocation);
    }

    action
}

fn settings_view(ui: &mut Ui, gem: &mut GemPlayer) {
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
                    ui.add(unselectable_label("If you like this project, consider supporting me:"));
                    ui.hyperlink_to("Ko-fi", "https://ko-fi.com/jamesmoreau");
                });
            });
        });
}

fn navigation_bar(ui: &mut Ui, gem: &mut GemPlayer) {
    Frame::new().inner_margin(Margin::symmetric(16, 0)).show(ui, |ui| {
        ui.columns_const(|[left, center, right]| {
            left.with_layout(Layout::left_to_right(Align::Center), |ui| {
                let get_icon_and_tooltip = |view: &View| match view {
                    View::Library => icons::ICON_LIBRARY_MUSIC,
                    View::Queue => icons::ICON_QUEUE_MUSIC,
                    View::Playlists => icons::ICON_STAR,
                    View::Settings => icons::ICON_SETTINGS,
                };

                for view in View::iter() {
                    let icon = get_icon_and_tooltip(&view);
                    let response = ui
                        .selectable_label(gem.ui.current_view == view, format!("  {icon}  "))
                        .on_hover_text(format!("{:?}", view));
                    if response.clicked() {
                        switch_view(&mut gem.ui, view);
                    }

                    ui.add_space(4.0);
                }
            });

            center.with_layout(Layout::centered_and_justified(Direction::TopDown), |ui| match gem.ui.current_view {
                View::Library => {
                    let tracks_count_and_duration = get_count_and_duration_string_from_tracks(&gem.library);
                    ui.add(unselectable_label(tracks_count_and_duration));
                }
                View::Queue => {
                    let tracks_count_and_duration = get_count_and_duration_string_from_tracks(&gem.player.queue);
                    ui.add(unselectable_label(tracks_count_and_duration));
                }
                View::Playlists => {
                    let Some(playlist_key) = &gem.ui.playlists.selected_playlist_key else {
                        return;
                    };

                    let playlist = gem.playlists.get_by_path(playlist_key);

                    let tracks_count_and_duration = get_count_and_duration_string_from_tracks(&playlist.tracks);
                    ui.add(unselectable_label(tracks_count_and_duration));
                }
                View::Settings => {}
            });

            right.with_layout(Layout::right_to_left(Align::Center), |ui| match gem.ui.current_view {
                View::Library => {
                    let search_was_changed = search_ui(ui, &mut gem.ui.search);
                    if search_was_changed {
                        // We reset both caches since there is only one search text state variable.
                        gem.ui.library.cached_library = None;
                        gem.ui.library.selected_tracks.clear();
                        gem.ui.playlists.cached_playlist_tracks = None;
                        gem.ui.playlists.selected_tracks.clear();
                    }

                    let sort_was_changed = sort_and_order_by_ui(ui, &mut gem.ui.library.sort_by, &mut gem.ui.library.sort_order);
                    if sort_was_changed {
                        gem.ui.library.cached_library = None;
                    }
                }
                View::Queue => {
                    let queue_is_not_empty = !gem.player.queue.is_empty();

                    let clear_button = Button::new(icons::ICON_CLEAR_ALL);
                    let response = ui
                        .add_enabled(queue_is_not_empty, clear_button)
                        .on_hover_text("Clear")
                        .on_disabled_hover_text("Queue is empty");
                    if response.clicked() {
                        clear_the_queue(&mut gem.player);
                    }
                }
                View::Playlists => {
                    let search_changed = search_ui(ui, &mut gem.ui.search);
                    if search_changed {
                        // Same as above.
                        gem.ui.library.cached_library = None;
                        gem.ui.library.selected_tracks.clear();
                        gem.ui.playlists.cached_playlist_tracks = None;
                        gem.ui.playlists.selected_tracks.clear();
                    }
                }
                _ => {}
            });
        });
    });
}

fn sort_and_order_by_ui(ui: &mut Ui, sort_by: &mut SortBy, sort_order: &mut SortOrder) -> bool {
    let response = ui.button(icons::ICON_FILTER_LIST).on_hover_text("Sort by and order");

    let mut sort_by_changed = false;
    let mut sort_order_changed = false;

    Popup::menu(&response)
        .gap(4.0)
        .close_behavior(PopupCloseBehavior::CloseOnClickOutside)
        .show(|ui| {
            for sb in SortBy::iter() {
                sort_by_changed |= ui.radio_value(sort_by, sb, format!("{:?}", sb)).changed();
            }
            ui.separator();
            for so in SortOrder::iter() {
                sort_order_changed |= ui.radio_value(sort_order, so, format!("{:?}", so)).changed();
            }
        });

    sort_by_changed || sort_order_changed
}

fn search_ui(ui: &mut Ui, search_text: &mut String) -> bool {
    let mut changed = false;
    let clear_button_is_visible = !search_text.is_empty();
    let response = ui
        .add_visible(clear_button_is_visible, Button::new(icons::ICON_CLEAR))
        .on_hover_text("Clear search");
    if response.clicked() {
        search_text.clear();
        changed = true;
    }

    let search_bar = TextEdit::singleline(search_text)
        .hint_text(format!("{} Search ...", icons::ICON_SEARCH))
        .desired_width(140.0)
        .char_limit(20);

    let response = ui.add(search_bar);
    if response.changed() {
        changed = true;
    }

    changed
}
