use crate::{
    format_duration_to_hhmmss, format_duration_to_mmss, maybe_play_next, maybe_play_previous, play_library, play_playlist,
    player::{
        clear_the_queue, enqueue, enqueue_next, move_to_position, mute_or_unmute, play_or_pause, remove_from_queue, toggle_shuffle, Player,
    },
    playlist::{add_to_playlist, create, delete, remove_from_playlist, rename, PlaylistRetrieval},
    start_library_watcher, tickle_watcher,
    track::{calculate_total_duration, open_file_location, sort, SortBy, SortOrder, TrackRetrieval},
    GemPlayer, Track, KEY_COMMANDS,
};
use dark_light::Mode;
use eframe::egui::{
    containers, include_image, os::OperatingSystem, popup, text, AboveOrBelow, Align, Align2, Button, CentralPanel, Color32, Context,
    Direction, FontId, Frame, Id, Image, Label, Layout, Margin, PointerButton, RichText, ScrollArea, Sense, Separator, Slider, TextEdit,
    TextFormat, TextStyle, TextureFilter, TextureOptions, ThemePreference, Ui, UiBuilder, Vec2, ViewportCommand, Visuals,
};
use egui_extras::{Size, StripBuilder, TableBuilder};
use egui_inbox::UiInbox;
use egui_material_icons::icons;
use egui_notify::Toasts;
use fully_pub::fully_pub;
use function_name::named;
use log::{error, info};
use rfd::FileDialog;
use std::{
    path::{Path, PathBuf},
    time::{Duration, Instant},
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
pub struct UIState {
    current_view: View,
    theme_preference: ThemePreference,
    theme_dirty: bool,
    marquee: MarqueeState,
    search: String,
    cached_artwork_uri: Option<String>,

    library: LibraryViewState,
    playlists: PlaylistsViewState,

    toasts: Toasts,
}

const MARQUEE_SPEED: f32 = 5.0; // chars per second
const MARQUEE_PAUSE_DURATION: Duration = Duration::from_secs(2);

#[fully_pub]
pub struct MarqueeState {
    offset: usize,
    track_identifier: Option<PathBuf>,

    last_update: Instant,
    next_update: Instant,
    pause_until: Option<Instant>,
}

#[fully_pub]
struct LibraryViewState {
    selected_track_key: Option<PathBuf>,
    track_menu_is_open: bool, // The menu is open for selected_track

    cached_library: Vec<Track>,
    cache_dirty: bool,

    sort_by: SortBy,
    sort_order: SortOrder,
}

#[fully_pub]
struct PlaylistsViewState {
    selected_playlist_key: Option<PathBuf>, // None: no playlist is selected. Some: the path of the selected playlist.
    selected_track_key: Option<PathBuf>,

    cached_playlist_tracks: Vec<Track>,
    cache_dirty: bool,

    playlist_rename: Option<String>, // If Some, the playlist pointed to by selected_track's name is being edited and a buffer for the new name.
    delete_playlist_modal_is_open: bool, // The menu is open for selected_playlist_path.
    track_menu_is_open: bool,        // The menu is open for selected_playlist_path.
}

pub fn maybe_update_theme(gem_player: &mut GemPlayer, ctx: &Context) {
    if !gem_player.ui_state.theme_dirty {
        return; // We don't need to update the theme if it has not been changed.
    }

    match gem_player.ui_state.theme_preference {
        ThemePreference::Dark => ctx.set_visuals(Visuals::dark()),
        ThemePreference::Light => ctx.set_visuals(Visuals::light()),
        ThemePreference::System => {
            let visuals = match dark_light::detect() {
                Ok(Mode::Light) => Visuals::light(),
                _ => Visuals::dark(), // Covers both Mode::Dark, Mode::Unspecified, and errors
            };
            ctx.set_visuals(visuals);
        }
    }
}

pub fn render_gem_player(gem_player: &mut GemPlayer, ctx: &Context) {
    custom_window_frame(ctx, "", |ui| {
        let control_ui_height = 64.0;
        let navigation_ui_height = 32.0;
        let separator_space = 2.0; // Even numbers seem to work better for getting pixel perfect placements.

        ctx.texture_ui(ui);

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
                strip.cell(|ui| {
                    render_control_panel(ui, gem_player);
                });
                strip.cell(|ui| {
                    ui.add(Separator::default().spacing(separator_space));
                });
                strip.cell(|ui| match gem_player.ui_state.current_view {
                    View::Library => render_library_view(ui, gem_player),
                    View::Queue => render_queue_view(ui, &mut gem_player.player),
                    View::Playlists => render_playlists_view(ui, gem_player),
                    View::Settings => render_settings_view(ui, gem_player),
                });
                strip.cell(|ui| {
                    ui.add(Separator::default().spacing(separator_space));
                });
                strip.cell(|ui| {
                    render_navigation_ui(ui, gem_player);
                });
            });
    });
}

fn custom_window_frame(ctx: &Context, title: &str, add_contents: impl FnOnce(&mut Ui)) {
    let panel_frame = Frame {
        fill: ctx.style().visuals.window_fill(),
        corner_radius: 10.0.into(),
        stroke: ctx.style().visuals.widgets.noninteractive.bg_stroke,
        outer_margin: 0.5.into(), // so the stroke is within the bounds
        ..Default::default()
    };

    CentralPanel::default().frame(panel_frame).show(ctx, |ui| {
        let app_rect = ui.max_rect();

        let title_bar_height = 24.0;
        let title_bar_rect = {
            let mut rect = app_rect;
            rect.max.y = rect.min.y + title_bar_height;
            rect
        };
        render_title_bar(ui, title_bar_rect, title);

        let content_rect = {
            let mut rect = app_rect;
            rect.min.y = title_bar_rect.max.y;
            rect
        }
        .shrink2(Vec2::new(2.0, 0.0));
        let mut content_ui = ui.new_child(UiBuilder::new().max_rect(content_rect));
        add_contents(&mut content_ui);
    });
}

fn render_title_bar(ui: &mut Ui, title_bar_rect: eframe::epaint::Rect, title: &str) {
    let painter = ui.painter();

    let title_bar_response = ui.interact(title_bar_rect, Id::new("title_bar"), Sense::click_and_drag());

    painter.text(
        title_bar_rect.center(),
        Align2::CENTER_CENTER,
        title,
        FontId::proportional(20.0),
        ui.style().visuals.text_color(),
    );

    if title_bar_response.double_clicked() {
        let is_maximized = ui.input(|i| i.viewport().maximized.unwrap_or(false));
        ui.ctx().send_viewport_cmd(ViewportCommand::Maximized(!is_maximized));
    }

    if title_bar_response.drag_started_by(PointerButton::Primary) {
        ui.ctx().send_viewport_cmd(ViewportCommand::StartDrag);
    }

    let is_macos = ui.ctx().os() == OperatingSystem::Mac;
    ui.scope_builder(
        UiBuilder::new().max_rect(title_bar_rect).layout(if is_macos {
            Layout::left_to_right(Align::Center)
        } else {
            Layout::right_to_left(Align::Center)
        }),
        |ui| {
            ui.add_space(8.0);

            ui.visuals_mut().button_frame = false;
            let button_height = 12.0;

            let close_button = |ui: &mut Ui| {
                let response = ui
                    .add(Button::new(RichText::new(icons::ICON_CLOSE).size(button_height)))
                    .on_hover_text("Close the window");
                if response.clicked() {
                    ui.ctx().send_viewport_cmd(ViewportCommand::Close);
                }
            };

            let fullscreen_button = |ui: &mut Ui| {
                let is_fullscreen = ui.input(|i| i.viewport().fullscreen.unwrap_or(false));
                let tooltip = if is_fullscreen { "Restore window" } else { "Maximize window" };
                let response = ui
                    .add(Button::new(RichText::new(icons::ICON_SQUARE).size(button_height)))
                    .on_hover_text(tooltip);
                if response.clicked() {
                    ui.ctx().send_viewport_cmd(ViewportCommand::Fullscreen(!is_fullscreen));
                }
            };

            let minimize_button = |ui: &mut Ui| {
                let response = ui
                    .add(Button::new(RichText::new(icons::ICON_MINIMIZE).size(button_height)))
                    .on_hover_text("Minimize the window");
                if response.clicked() {
                    ui.ctx().send_viewport_cmd(ViewportCommand::Minimized(true));
                }
            };

            if is_macos {
                close_button(ui);
                minimize_button(ui);
                fullscreen_button(ui);
            } else {
                minimize_button(ui);
                fullscreen_button(ui);
                close_button(ui);
            }
        },
    );
}

fn switch_view(ui_state: &mut UIState, view: View) {
    info!("Switching to view: {:?}", view);
    ui_state.current_view = view;
}

fn render_control_panel(ui: &mut Ui, gem_player: &mut GemPlayer) {
    // Specifying the widths of the elements in the track info component before-hand allows us to center them horizontally.
    let button_width = 20.0;
    let gap = 10.0;
    let artwork_width = ui.available_height();
    let slider_width = 420.0;

    Frame::new().inner_margin(Margin::symmetric(16, 0)).show(ui, |ui| {
        StripBuilder::new(ui)
            .size(Size::remainder())
            .size(Size::exact(button_width + gap + artwork_width + gap + slider_width))
            .size(Size::remainder())
            .horizontal(|mut strip| {
                strip.cell(|ui| {
                    render_playback_controls(ui, gem_player);
                });

                strip.cell(|ui| {
                    render_track_info(ui, gem_player, button_width, gap, artwork_width, slider_width);
                });

                strip.cell(|ui| {
                    render_volume_controls(ui, gem_player);
                });
            });
    });
}

fn render_playback_controls(ui: &mut Ui, gem_player: &mut GemPlayer) {
    ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
        let track_is_playing = gem_player.player.playing.is_some();

        let previous_button = Button::new(RichText::new(icons::ICON_SKIP_PREVIOUS).size(18.0));
        let previous_track_exists = !gem_player.player.history.is_empty();
        let is_previous_enabled = track_is_playing || previous_track_exists;

        let response = ui
            .add_enabled(is_previous_enabled, previous_button)
            .on_hover_text("Previous")
            .on_disabled_hover_text("No previous track");
        if response.clicked() {
            maybe_play_previous(gem_player)
        }

        let sink_is_paused = gem_player.player.sink.is_paused();
        let play_pause_icon = if sink_is_paused {
            icons::ICON_PLAY_ARROW
        } else {
            icons::ICON_PAUSE
        };
        let tooltip = if sink_is_paused { "Play" } else { "Pause" };
        let play_pause_button = Button::new(RichText::new(play_pause_icon).size(24.0));
        let response = ui
            .add_enabled(track_is_playing, play_pause_button)
            .on_hover_text(tooltip)
            .on_disabled_hover_text("No current track");
        if response.clicked() {
            play_or_pause(&mut gem_player.player);
        }

        let next_button = Button::new(RichText::new(icons::ICON_SKIP_NEXT).size(18.0));
        let next_track_exists = !gem_player.player.queue.is_empty();
        let response = ui
            .add_enabled(next_track_exists, next_button)
            .on_hover_text("Next")
            .on_disabled_hover_text("No next track");
        if response.clicked() {
            maybe_play_next(gem_player);
        }
    });
}

fn render_track_info(ui: &mut Ui, gem_player: &mut GemPlayer, button_size: f32, gap: f32, artwork_width: f32, slider_width: f32) {
    ui.spacing_mut().item_spacing = Vec2::splat(0.0);
    let available_height = ui.available_height();

    if gem_player.player.playing.is_some() {
        // Necessary to keep UI up-to-date with the current state of the sink/player.
        // We only need to call this if there is a currently playing track.
        ui.ctx().request_repaint_after_secs(1.0);
    }

    StripBuilder::new(ui)
        .size(Size::exact(button_size))
        .size(Size::exact(gap))
        .size(Size::exact(artwork_width))
        .size(Size::exact(gap))
        .size(Size::exact(slider_width))
        .horizontal(|mut strip| {
            strip.cell(|ui| {
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

                let color = get_button_color(ui, gem_player.player.repeat);
                let repeat_button = Button::new(RichText::new(icons::ICON_REPEAT).color(color)).min_size(Vec2::splat(button_size));
                let response = ui.add(repeat_button).on_hover_text("Repeat");
                if response.clicked() {
                    gem_player.player.repeat = !gem_player.player.repeat;
                }

                ui.add_space(4.0);

                let color = get_button_color(ui, gem_player.player.shuffle.is_some());
                let shuffle_button = Button::new(RichText::new(icons::ICON_SHUFFLE).color(color)).min_size(Vec2::splat(button_size));
                let queue_is_not_empty = !gem_player.player.queue.is_empty();
                let response = ui
                    .add_enabled(queue_is_not_empty, shuffle_button)
                    .on_hover_text("Shuffle")
                    .on_disabled_hover_text("Queue is empty");
                if response.clicked() {
                    toggle_shuffle(&mut gem_player.player);
                }
            });
            strip.empty();
            strip.cell(|ui| {
                render_artwork(ui, gem_player, artwork_width);
            });
            strip.empty();
            strip.strip(|builder| {
                let mut position_as_secs = 0.0;
                let mut track_duration_as_secs = 0.1; // We set to 0.1 so that when no track is playing, the slider is at the start.

                if let Some(playing_track) = &gem_player.player.playing {
                    position_as_secs = gem_player.player.sink.get_pos().as_secs_f32();
                    track_duration_as_secs = playing_track.duration.as_secs_f32();
                }

                builder.sizes(Size::exact(available_height / 2.0), 2).vertical(|mut strip| {
                    strip.cell(|ui| {
                        ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                            ui.style_mut().spacing.slider_width = slider_width;
                            let playback_progress_slider = Slider::new(&mut position_as_secs, 0.0..=track_duration_as_secs)
                                .trailing_fill(true)
                                .show_value(false)
                                .step_by(1.0); // Step by 1 second.
                            let track_is_playing = gem_player.player.playing.is_some();
                            let response = ui.add_enabled(track_is_playing, playback_progress_slider);

                            if response.dragged() && gem_player.player.paused_before_scrubbing.is_none() {
                                gem_player.player.paused_before_scrubbing = Some(gem_player.player.sink.is_paused());
                                gem_player.player.sink.pause(); // Pause playback during scrubbing
                            }

                            if response.drag_stopped() {
                                let new_position = Duration::from_secs_f32(position_as_secs);
                                info!("Seeking to {}", format_duration_to_mmss(new_position));
                                if let Err(e) = gem_player.player.sink.try_seek(new_position) {
                                    error!("Error seeking to new position: {:?}", e);
                                }

                                // Resume playback if the player was not paused before scrubbing
                                if gem_player.player.paused_before_scrubbing == Some(false) {
                                    gem_player.player.sink.play();
                                }

                                gem_player.player.paused_before_scrubbing = None;
                            }
                        });
                    });
                    strip.strip(|builder| {
                        // Placing the track info after the slider ensures that the playback position display is accurate. The seek operation is only
                        // executed after the slider thumb is released. If we placed the display before, the current position would not be reflected.
                        builder
                            .size(Size::exact(slider_width * (4.0 / 5.0)))
                            .size(Size::exact(slider_width * (1.0 / 5.0)))
                            .horizontal(|mut hstrip| {
                                hstrip.cell(|ui| {
                                    render_track_marquee(ui, gem_player.player.playing.as_ref(), &mut gem_player.ui_state.marquee);
                                });

                                hstrip.cell(|ui| {
                                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                        let position = Duration::from_secs_f32(position_as_secs);
                                        let track_duration = Duration::from_secs_f32(track_duration_as_secs);
                                        let time_label_text = format!(
                                            "{} / {}",
                                            format_duration_to_mmss(position),
                                            format_duration_to_mmss(track_duration)
                                        );

                                        let time_label = unselectable_label(time_label_text);
                                        ui.add(time_label);
                                    });
                                });
                            });
                    });
                });
            });
        });
}

fn render_track_marquee(ui: &mut Ui, maybe_track: Option<&Track>, marquee: &mut MarqueeState) {
    ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
        let mut title = "None";
        let mut artist = "None";
        let mut album = "None";
        let mut track_identifier = Some(PathBuf::from("none"));

        if let Some(playing_track) = maybe_track {
            title = playing_track.title.as_deref().unwrap_or("Unknown Title");
            artist = playing_track.artist.as_deref().unwrap_or("Unknown Artist");
            album = playing_track.album.as_deref().unwrap_or("Unknown Album");
            track_identifier = Some(playing_track.path.clone());
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

        let galley = ui.fonts(|fonts| fonts.layout_job(format_colored_marquee_text(&text)));

        let text_width = galley.size().x;
        let available_width = ui.available_width();
        let character_count = text.chars().count();
        let average_char_width = text_width / character_count as f32;
        let visible_chars = (available_width / average_char_width).floor() as usize;

        if character_count <= visible_chars {
            ui.add(Label::new(format_colored_marquee_text(&text)).selectable(false).truncate());
            return;
        }

        let seconds_per_char = MARQUEE_SPEED.recip();
        let now = Instant::now();

        // Reset marquee state if track changes.
        if marquee.track_identifier != track_identifier || marquee.track_identifier.is_none() {
            marquee.offset = 0;
            marquee.track_identifier = track_identifier.clone();
            marquee.pause_until = Some(now + MARQUEE_PAUSE_DURATION);
            marquee.last_update = now;
            marquee.next_update = now + MARQUEE_PAUSE_DURATION + Duration::from_secs_f32(seconds_per_char);
        }

        if let Some(paused_until) = marquee.pause_until {
            if now < paused_until {
                ui.ctx().request_repaint_after(paused_until - now);
                let display_text: String = text.chars().take(visible_chars).collect();
                ui.add(Label::new(format_colored_marquee_text(&display_text)).selectable(false).truncate());
                return;
            } else {
                marquee.pause_until = None;
                marquee.last_update = now;
                marquee.next_update = now + Duration::from_secs_f32(seconds_per_char);
            }
        }

        // Advance marquee only if the next expected update time has passed.
        if now >= marquee.next_update {
            marquee.offset += 1;
            marquee.last_update = now;
            marquee.next_update = now + Duration::from_secs_f32(seconds_per_char);

            // Wrap-around and trigger pause at the beginning.
            if marquee.offset >= character_count {
                marquee.offset = 0;
                marquee.pause_until = Some(now + MARQUEE_PAUSE_DURATION);
                marquee.next_update = now + MARQUEE_PAUSE_DURATION + Duration::from_secs_f32(seconds_per_char);
            }
        }

        ui.ctx().request_repaint_after(marquee.next_update - now);

        let display_text: String = text.chars().chain(text.chars()).skip(marquee.offset).take(visible_chars).collect();
        ui.add(Label::new(format_colored_marquee_text(&display_text)).selectable(false).truncate());
    });
}

fn render_artwork(ui: &mut Ui, gem_player: &mut GemPlayer, artwork_width: f32) {
    let artwork_texture_options = TextureOptions::LINEAR.with_mipmap_mode(Some(TextureFilter::Linear));
    let artwork_size = Vec2::splat(artwork_width);

    // Use a default image; if artwork exists for the playing track, load it.
    let mut artwork = Image::new(include_image!("../assets/music_note.svg"));

    if let Some(playing_track) = &gem_player.player.playing {
        if let Some(artwork_bytes) = &playing_track.artwork {
            let uri = format!("bytes://artwork-{}", playing_track.path.to_string_lossy());

            match &gem_player.ui_state.cached_artwork_uri {
                Some(cached_uri) if *cached_uri == uri => {
                    // Already cached.
                    artwork = Image::new(uri);
                }
                Some(cached_uri) => {
                    // Artwork has change. Release the cached uri.
                    ui.ctx().forget_image(cached_uri);
                    artwork = Image::from_bytes(uri.clone(), artwork_bytes.clone());
                    gem_player.ui_state.cached_artwork_uri = Some(uri);
                }
                None => {
                    // No cache, load new artwork.
                    artwork = Image::from_bytes(uri.clone(), artwork_bytes.clone());
                    gem_player.ui_state.cached_artwork_uri = Some(uri);
                }
            }
        }
    } else {
        // No playing track, so release the cache if there is one.
        if let Some(cached_uri) = &gem_player.ui_state.cached_artwork_uri {
            ui.ctx().forget_image(cached_uri);
            gem_player.ui_state.cached_artwork_uri = None;
        }
    }

    ui.add(
        artwork
            .texture_options(artwork_texture_options)
            .fit_to_exact_size(artwork_size)
            .maintain_aspect_ratio(false)
            .corner_radius(2.0),
    );
}

fn render_volume_controls(ui: &mut Ui, gem_player: &mut GemPlayer) {
    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
        let mut volume = gem_player.player.sink.volume();
        let volume_slider = Slider::new(&mut volume, 0.0..=1.0).trailing_fill(true).show_value(false);
        let changed = ui.add(volume_slider).changed();
        if changed {
            gem_player.player.muted = false;
            gem_player.player.volume_before_mute = if volume == 0.0 { None } else { Some(volume) }
        }
        gem_player.player.sink.set_volume(volume);

        let volume_icon = match gem_player.player.sink.volume() {
            0.0 => icons::ICON_VOLUME_OFF,
            v if v <= 0.5 => icons::ICON_VOLUME_DOWN,
            _ => icons::ICON_VOLUME_UP, // v > 0.5 && v <= 1.0
        };
        let tooltip = if gem_player.player.muted { "Unmute" } else { "Mute" };
        let volume_button = Button::new(RichText::new(volume_icon).size(18.0));
        let response = ui.add(volume_button).on_hover_text(tooltip);
        if response.clicked() {
            mute_or_unmute(&mut gem_player.player);
        }
    });
}

fn render_library_view(ui: &mut Ui, gem_player: &mut GemPlayer) {
    if gem_player.library.is_empty() {
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

    render_library_track_menu(ui, gem_player);

    if gem_player.ui_state.library.cache_dirty {
        gem_player.ui_state.library.cached_library = gem_player
            .library
            .iter()
            .filter(|track| {
                let search_lower = gem_player.ui_state.search.to_lowercase();

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

        sort(
            &mut gem_player.ui_state.library.cached_library,
            gem_player.ui_state.library.sort_by,
            gem_player.ui_state.library.sort_order,
        );

        gem_player.ui_state.library.cache_dirty = false;
    }

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

    let mut should_open_track_menu = None;
    let mut should_select_track = None;
    let mut should_play_library = None;

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
            body.rows(26.0, gem_player.ui_state.library.cached_library.len(), |mut row| {
                let track = &gem_player.ui_state.library.cached_library[row.index()];

                let row_is_selected = gem_player
                    .ui_state
                    .library
                    .selected_track_key
                    .as_ref()
                    .is_some_and(|t| *t == track.path);
                row.set_selected(row_is_selected);

                row.col(|ui| {
                    ui.add_space(16.0);
                    ui.add(unselectable_label(track.title.as_deref().unwrap_or("Unknown Title")).truncate());
                });

                row.col(|ui| {
                    ui.add_space(4.0);
                    ui.add(unselectable_label(track.artist.as_deref().unwrap_or("Unknown Artist")).truncate());
                });

                row.col(|ui| {
                    ui.add_space(4.0);
                    ui.add(unselectable_label(track.album.as_deref().unwrap_or("Unknown")));
                });

                row.col(|ui| {
                    ui.add_space(4.0);
                    let duration_string = format_duration_to_mmss(track.duration);
                    ui.add(unselectable_label(duration_string));
                });

                let rest_of_row_is_hovered = row.response().hovered();
                let mut more_cell_contains_pointer = false;
                row.col(|ui| {
                    more_cell_contains_pointer = ui.rect_contains_pointer(ui.max_rect());
                    let should_show_more_button: bool = rest_of_row_is_hovered || more_cell_contains_pointer || row_is_selected;

                    ui.add_space(8.0);

                    ui.scope_builder(
                        {
                            if should_show_more_button {
                                UiBuilder::new()
                            } else {
                                UiBuilder::new().invisible()
                            }
                        },
                        |ui| {
                            let more_button = Button::new(icons::ICON_MORE_HORIZ);
                            let response = ui.add(more_button).on_hover_text("More");
                            if response.clicked() {
                                should_open_track_menu = Some(track.path.clone());
                            }
                        },
                    );
                });

                let response = row.response();

                if response.clicked() {
                    should_select_track = Some(track.path.clone());
                }

                if response.secondary_clicked() {
                    should_open_track_menu = Some(track.path.clone());
                }

                if response.double_clicked() {
                    should_play_library = Some(track.clone());
                }
            });
        });

    if let Some(track_key) = should_open_track_menu {
        gem_player.ui_state.library.selected_track_key = Some(track_key);
        gem_player.ui_state.library.track_menu_is_open = true;
    }

    if let Some(track_key) = should_select_track {
        gem_player.ui_state.library.selected_track_key = Some(track_key);
    }

    if let Some(track) = should_play_library {
        if let Err(e) = play_library(gem_player, Some(&track)) {
            error!("{}", e);
            gem_player.ui_state.toasts.error("Error playing from playlist");
        }
    }
}

#[named]
fn render_library_track_menu(ui: &mut Ui, gem_player: &mut GemPlayer) {
    if !gem_player.ui_state.library.track_menu_is_open {
        return;
    }

    let Some(track_key) = &gem_player.ui_state.library.selected_track_key else {
        error!("{} was called, but there is no selected track.", function_name!());
        return;
    };

    let modal_width = 220.0;

    let modal = containers::Modal::new(Id::new("library_track_menu"))
        .backdrop_color(Color32::TRANSPARENT)
        .show(ui.ctx(), |ui| {
            ui.set_width(modal_width);

            ui.vertical_centered_justified(|ui| {
                let title = gem_player
                    .library
                    .get_by_path(track_key)
                    .title
                    .as_deref()
                    .unwrap_or("Unknown Title");
                ui.label(RichText::new(title).strong());

                ui.add_space(8.0);

                let add_to_playlists_enabled = !gem_player.playlists.is_empty();
                ui.add_enabled_ui(add_to_playlists_enabled, |ui| {
                    ui.menu_button("Add to Playlist", |ui| {
                        ui.set_min_width(modal_width);

                        ScrollArea::vertical().max_height(164.0).show(ui, |ui| {
                            for playlist in gem_player.playlists.iter_mut() {
                                let response = ui.button(&playlist.name);
                                if response.clicked() {
                                    let track = gem_player.library.get_by_path(track_key);
                                    let result = add_to_playlist(playlist, track.clone());
                                    if let Err(e) = result {
                                        error!("{}", e);
                                        gem_player.ui_state.toasts.error(format!("{}", e));
                                    }
                                    ui.close_menu();
                                    gem_player.ui_state.library.track_menu_is_open = false;
                                    gem_player.ui_state.playlists.cache_dirty = true;
                                }
                            }
                        });
                    });
                });

                ui.separator();

                let response = ui.button(format!("{} Play Next", icons::ICON_PLAY_ARROW));
                if response.clicked() {
                    let track = gem_player.library.get_by_path(track_key);
                    enqueue_next(&mut gem_player.player, track.clone());
                    gem_player.ui_state.library.track_menu_is_open = false;
                }

                let response = ui.button(format!("{} Add to Queue", icons::ICON_QUEUE_MUSIC));
                if response.clicked() {
                    let track = gem_player.library.get_by_path(track_key);
                    enqueue(&mut gem_player.player, track.clone());
                    gem_player.ui_state.library.track_menu_is_open = false;
                }

                ui.separator();

                let response = ui.button(format!("{} Open File Location", icons::ICON_FOLDER));
                if response.clicked() {
                    let track = gem_player.library.get_by_path(track_key);
                    let result = open_file_location(track);
                    match result {
                        Err(e) => error!("{}", e),
                        Ok(_) => info!("Opening track location"),
                    }
                    gem_player.ui_state.library.track_menu_is_open = false;
                }
            });
        });

    if modal.should_close() {
        gem_player.ui_state.library.track_menu_is_open = false;
    }
}

fn render_queue_view(ui: &mut Ui, player: &mut Player) {
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

    ui.spacing_mut().item_spacing.x = 0.0; // See comment in render_library_ui for why we set item_spacing to 0.

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
                    ui.add(unselectable_label(track.title.as_deref().unwrap_or("Unknown Title")));
                });

                row.col(|ui| {
                    ui.add_space(4.0);
                    ui.add(unselectable_label(track.artist.as_deref().unwrap_or("Unknown Artist")));
                });

                row.col(|ui| {
                    ui.add_space(4.0);
                    ui.add(unselectable_label(track.album.as_deref().unwrap_or("Unknown")));
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

fn render_playlists_view(ui: &mut Ui, gem_player: &mut GemPlayer) {
    if gem_player.library_directory.is_none() {
        Frame::new()
            .outer_margin(Margin::symmetric((ui.available_width() * (1.0 / 4.0)) as i8, 32))
            .show(ui, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add(unselectable_label("Try adding your music directory in the settings."));
                });
            });

        return;
    };

    render_delete_playlist_modal(ui, gem_player);

    let size = ui.available_size();
    let playlists_width = size.x * (1.0 / 4.0);

    ui.spacing_mut().item_spacing.x = 0.0; // See comment in render_library_ui() as to why we do this.

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
                                        let directory = gem_player.library_directory.as_ref().unwrap(); // We checked earlier so this is safe.
                                        let new_playlist_name = format!("Playlist {}", gem_player.playlists.len() + 1);
                                        let result = create(new_playlist_name, directory);
                                        match result {
                                            Err(e) => {
                                                let error_message = format!("Failed to create: {}.", e);
                                                error!("{}", &error_message);
                                                gem_player.ui_state.toasts.error(&error_message);
                                            }
                                            Ok(new_playlist) => {
                                                info!("Created and saved {} to {:?}.", &new_playlist.name, &new_playlist.m3u_path);
                                                gem_player.playlists.push(new_playlist);
                                            }
                                        }
                                    }
                                },
                            );
                        });
                    })
                    .body(|body| {
                        body.rows(36.0, gem_player.playlists.len(), |mut row| {
                            let playlist = &mut gem_player.playlists[row.index()];

                            if let Some(playlist_key) = &gem_player.ui_state.playlists.selected_playlist_key {
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
                                gem_player.ui_state.playlists.selected_playlist_key = Some(playlist.m3u_path.clone());

                                // Reset in case we were currently editing.
                                gem_player.ui_state.playlists.playlist_rename = None;

                                // Invalidate the playlist ui track cache.
                                gem_player.ui_state.playlists.cache_dirty = true;
                            }
                        });
                    });
            });

            strip.cell(|ui| {
                ui.add(Separator::default().vertical());
            });

            strip.cell(|ui| {
                render_playlist(ui, gem_player);
            });
        });
}

fn render_delete_playlist_modal(ui: &mut Ui, gem_player: &mut GemPlayer) {
    if !gem_player.ui_state.playlists.delete_playlist_modal_is_open {
        return;
    }

    let Some(playlist_key) = gem_player.ui_state.playlists.selected_playlist_key.clone() else {
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

                            let result = delete(&playlist_key, &mut gem_player.playlists);
                            if let Err(e) = result {
                                error!("{}", e);
                            } else {
                                let message =
                                    "Playlist was deleted successfully. If this was a mistake, the m3u file can be found in the trash.";
                                info!("{}", message);
                                gem_player.ui_state.toasts.success(message);
                                gem_player.ui_state.playlists.selected_playlist_key = None;
                            }
                        }
                    },
                );
            });
        });

    if confirm_clicked || cancel_clicked || modal.should_close() {
        // maybe just handle event inside completely or outside completely.
        gem_player.ui_state.playlists.delete_playlist_modal_is_open = false;
    }
}

fn render_playlist(ui: &mut Ui, gem_player: &mut GemPlayer) {
    render_playlist_track_menu(ui, gem_player);

    let Some(playlist_key) = gem_player.ui_state.playlists.selected_playlist_key.clone() else {
        return; // No playlist selected, do nothing
    };

    StripBuilder::new(ui)
        .size(Size::exact(64.0))
        .size(Size::remainder())
        .vertical(|mut strip| {
            strip.cell(|ui| {
                Frame::new().fill(ui.visuals().faint_bg_color).show(ui, |ui| {
                    if let Some(name_buffer) = &mut gem_player.ui_state.playlists.playlist_rename {
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

                            let playlist = &mut gem_player.playlists.get_by_path_mut(&playlist_key);
                            let result = rename(playlist, name_buffer_clone);
                            match result {
                                Err(e) => {
                                    let message = format!("Error renaming playlist: {}", e);
                                    error!("{}", message);
                                    gem_player.ui_state.toasts.error(message);
                                }
                                Ok(_) => {
                                    // Update the selected playlist with the new path so that we remain selected.
                                    gem_player.ui_state.playlists.selected_playlist_key = Some(playlist.m3u_path.clone());
                                }
                            }

                            gem_player.ui_state.playlists.playlist_rename = None;
                        }

                        if discard_clicked {
                            gem_player.ui_state.playlists.playlist_rename = None;
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

                                let name = &gem_player.playlists.get_by_path(&playlist_key).name;
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

                        // We have to do this pattern since we want to access gem_player across
                        // the two captures used by containers::Sides.
                        if play_clicked {
                            let path = &gem_player.playlists.get_by_path(&playlist_key).m3u_path;
                            if let Err(e) = play_playlist(gem_player, &path.clone(), None) {
                                error!("{}", e);
                                gem_player.ui_state.toasts.error("Error playing from playlist");
                            }
                        }

                        if delete_clicked {
                            info!("Opening delete playlist modal");
                            gem_player.ui_state.playlists.delete_playlist_modal_is_open = true;
                        }

                        if edit_clicked {
                            let playlist = &mut gem_player.playlists.get_by_path(&playlist_key);
                            info!("Editing playlist name: {}", playlist.name);
                            gem_player.ui_state.playlists.playlist_rename = Some(playlist.name.clone());
                        }
                    }
                });
            });

            strip.cell(|ui| {
                render_playlist_tracks(ui, gem_player);
            });
        });
}

fn render_playlist_tracks(ui: &mut Ui, gem_player: &mut GemPlayer) {
    let Some(playlist_key) = gem_player.ui_state.playlists.selected_playlist_key.clone() else {
        Frame::new()
            .outer_margin(Margin::symmetric((ui.available_width() * (1.0 / 4.0)) as i8, 32))
            .show(ui, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add(unselectable_label("No playlist selected"));
                });
            });

        return;
    };

    let playlist_length = gem_player.playlists.get_by_path(&playlist_key).tracks.len();
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

    if gem_player.ui_state.playlists.cache_dirty {
        gem_player.ui_state.playlists.cached_playlist_tracks = gem_player
            .playlists
            .get_by_path(&playlist_key)
            .tracks
            .iter()
            .filter(|track| {
                let search_lower = gem_player.ui_state.search.to_lowercase();

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

        gem_player.ui_state.playlists.cache_dirty = false;
    }

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

    ui.spacing_mut().item_spacing.x = 0.0; // See comment in render_library_ui for why we set item_spacing to 0.

    let mut should_select_track = None;
    let mut should_open_track_menu = None;
    let mut should_play_playlist = None;

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
            body.rows(26.0, gem_player.ui_state.playlists.cached_playlist_tracks.len(), |mut row| {
                let index = row.index();
                let track = &gem_player.ui_state.playlists.cached_playlist_tracks[index];

                let row_is_selected = gem_player
                    .ui_state
                    .playlists
                    .selected_track_key
                    .as_ref()
                    .is_some_and(|p| *p == track.path);
                row.set_selected(row_is_selected);

                row.col(|ui| {
                    ui.add_space(16.0);
                    ui.add(unselectable_label(format!("{}", index + 1)));
                });

                row.col(|ui| {
                    ui.add_space(4.0);
                    ui.add(unselectable_label(track.title.as_deref().unwrap_or("Unknown Title")));
                });

                row.col(|ui| {
                    ui.add_space(4.0);
                    ui.add(unselectable_label(track.artist.as_deref().unwrap_or("Unknown Artist")));
                });

                row.col(|ui| {
                    ui.add_space(4.0);
                    ui.add(unselectable_label(track.album.as_deref().unwrap_or("Unknown")));
                });

                row.col(|ui| {
                    ui.add_space(4.0);
                    let duration_string = format_duration_to_mmss(track.duration);
                    ui.add(unselectable_label(duration_string));
                });

                let rest_of_row_is_hovered = row.response().hovered();
                let mut more_cell_contains_pointer = false;
                row.col(|ui| {
                    more_cell_contains_pointer = ui.rect_contains_pointer(ui.max_rect());
                    let should_show_more_button = rest_of_row_is_hovered || more_cell_contains_pointer || row_is_selected;

                    ui.add_space(8.0);

                    ui.scope_builder(
                        {
                            if should_show_more_button {
                                UiBuilder::new()
                            } else {
                                UiBuilder::new().invisible()
                            }
                        },
                        |ui| {
                            let more_button = Button::new(icons::ICON_MORE_HORIZ);
                            let response = ui.add(more_button).on_hover_text("More");
                            if response.clicked() {
                                should_open_track_menu = Some(track.path.clone())
                            }
                        },
                    );
                });

                let response = row.response();

                if response.clicked() {
                    should_select_track = Some(track.path.clone());
                }

                if response.secondary_clicked() {
                    should_open_track_menu = Some(track.path.clone())
                }

                if response.double_clicked() {
                    let playlist_key = gem_player.playlists.get_by_path(&playlist_key).m3u_path.clone();
                    should_play_playlist = Some((playlist_key, track.path.clone()));
                }
            });
        });

    if let Some(track_key) = should_open_track_menu {
        gem_player.ui_state.playlists.selected_track_key = Some(track_key);
        gem_player.ui_state.playlists.track_menu_is_open = true;
    }

    if let Some(track_key) = should_select_track {
        gem_player.ui_state.playlists.selected_track_key = Some(track_key);
    }

    if let Some((playlist_key, track_key)) = should_play_playlist {
        if let Err(e) = play_playlist(gem_player, &playlist_key, Some(&track_key)) {
            error!("{}", e);
            gem_player.ui_state.toasts.error("Error playing from playlist");
        }
    }
}

#[named]
fn render_playlist_track_menu(ui: &mut Ui, gem_player: &mut GemPlayer) {
    if !gem_player.ui_state.playlists.track_menu_is_open {
        return;
    }

    let Some(playlist_key) = gem_player.ui_state.playlists.selected_playlist_key.clone() else {
        error!("{} was called, but there is no selected playlist.", function_name!());
        gem_player.ui_state.playlists.track_menu_is_open = false;
        return;
    };

    let Some(track_key) = gem_player.ui_state.playlists.selected_track_key.clone() else {
        error!("{} was called, but there is no selected track id.", function_name!());
        gem_player.ui_state.playlists.track_menu_is_open = false;
        return;
    };

    let modal_width = 220.0;

    let modal = containers::Modal::new(Id::new("library_track_menu"))
        .backdrop_color(Color32::TRANSPARENT)
        .show(ui.ctx(), |ui| {
            ui.set_width(modal_width);

            ui.vertical_centered_justified(|ui| {
                let title = gem_player
                    .playlists
                    .get_by_path(&playlist_key)
                    .tracks
                    .get_by_path(&track_key)
                    .title
                    .as_deref()
                    .unwrap_or("Unknown Title");
                ui.label(RichText::new(title).strong());

                ui.add_space(8.0);

                let response = ui.button(format!("{} Remove from Playlist", icons::ICON_DELETE));
                if response.clicked() {
                    let playlist = gem_player.playlists.get_by_path_mut(&playlist_key);
                    let track_key = playlist.tracks.get_by_path(&track_key).path.clone();
                    let result = remove_from_playlist(playlist, &track_key);
                    if let Err(e) = result {
                        error!("{}", e);
                        gem_player.ui_state.toasts.error("Error removing track from playlist");
                    }
                    gem_player.ui_state.playlists.track_menu_is_open = false;
                    gem_player.ui_state.playlists.cache_dirty = true;
                }

                ui.separator();

                let response = ui.button(format!("{} Play Next", icons::ICON_PLAY_ARROW));
                if response.clicked() {
                    let track = gem_player.playlists.get_by_path(&playlist_key).tracks.get_by_path(&track_key);
                    enqueue_next(&mut gem_player.player, track.clone());
                    gem_player.ui_state.playlists.track_menu_is_open = false;
                }

                let response = ui.button(format!("{} Add to Queue", icons::ICON_ADD));
                if response.clicked() {
                    let track = gem_player.playlists.get_by_path(&playlist_key).tracks.get_by_path(&track_key);
                    enqueue(&mut gem_player.player, track.clone());
                    gem_player.ui_state.playlists.track_menu_is_open = false;
                }

                ui.separator();

                let response = ui.button(format!("{} Open File Location", icons::ICON_FOLDER));
                if response.clicked() {
                    let track = gem_player.playlists.get_by_path(&playlist_key).tracks.get_by_path(&track_key);
                    let result = open_file_location(track);
                    match result {
                        Err(e) => error!("{}", e),
                        Ok(_) => info!("Opening track location"),
                    }
                    gem_player.ui_state.playlists.track_menu_is_open = false;
                }
            });
        });

    if modal.should_close() {
        gem_player.ui_state.playlists.track_menu_is_open = false;
    }
}

fn render_settings_view(ui: &mut Ui, gem_player: &mut GemPlayer) {
    Frame::new()
        .outer_margin(Margin::symmetric((ui.available_width() * (1.0 / 4.0)) as i8, 32))
        .show(ui, |ui| {
            ScrollArea::vertical().show(ui, |ui| {
                ui.add(unselectable_label(RichText::new("Music Library Path").heading()));
                ui.add_space(8.0);
                ui.add(unselectable_label("Playlists are also stored here as m3u files."));
                ui.horizontal(|ui| {
                    let path = gem_player
                        .library_directory
                        .as_ref()
                        .map_or("No directory selected".to_string(), |p| p.to_string_lossy().to_string());
                    ui.label(path);

                    let response = ui.button(icons::ICON_FOLDER_OPEN).on_hover_text("Change");
                    if response.clicked() {
                        let maybe_directory = FileDialog::new()
                            .set_directory(gem_player.library_directory.as_deref().unwrap_or_else(|| Path::new("/")))
                            .pick_folder();

                        match maybe_directory {
                            None => info!("No folder selected"),
                            Some(directory) => {
                                info!("Selected folder: {:?}", directory);

                                let i = UiInbox::new();
                                let result = start_library_watcher(&directory, i.sender());
                                match result {
                                    Ok(dw) => {
                                        info!("Started watching: {:?}", &directory);
                                        gem_player.library_watcher = Some(dw);
                                        gem_player.library_watcher_inbox = Some(i);

                                        tickle_watcher(&directory);
                                    }
                                    Err(e) => error!("Failed to start watching the library directory: {e}"),
                                }
                            }
                        }
                    }
                });

                ui.add(Separator::default().spacing(32.0));

                ui.add(unselectable_label(RichText::new("Theme").heading()));
                ui.add_space(8.0);

                let before = gem_player.ui_state.theme_preference;
                ThemePreference::radio_buttons(&mut gem_player.ui_state.theme_preference, ui);
                let after = gem_player.ui_state.theme_preference;

                let theme_was_changed = before != after;
                if theme_was_changed {
                    gem_player.ui_state.theme_dirty = true;
                }

                ui.add(Separator::default().spacing(32.0));

                ui.add(unselectable_label(RichText::new("Key Commands").heading()));
                ui.add_space(8.0);
                for (key, binding) in KEY_COMMANDS.iter() {
                    containers::Sides::new().show(
                        ui,
                        |ui| {
                            ui.add(unselectable_label(format!("{:?}", key)));
                        },
                        |ui| {
                            ui.add_space(16.0);
                            ui.label(*binding);
                        },
                    );
                }

                ui.add(Separator::default().spacing(32.0));

                ui.add(unselectable_label(RichText::new("About Gem Player").heading()));
                ui.add_space(8.0);
                let version = env!("CARGO_PKG_VERSION");
                ui.add(unselectable_label(format!("Version: {version}")));
                let description = env!("CARGO_PKG_DESCRIPTION");
                ui.add(unselectable_label(description));

                ui.add(Separator::default().spacing(32.0));

                ui.add(unselectable_label(RichText::new("Author").heading()));
                ui.add_space(8.0);
                ui.add(unselectable_label("James Moreau"));
                ui.hyperlink_to("jamesmoreau.github.io", "https://jamesmoreau.github.io");
            });
        });
}

fn render_navigation_ui(ui: &mut Ui, gem_player: &mut GemPlayer) {
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
                        .selectable_label(gem_player.ui_state.current_view == view, format!("  {icon}  "))
                        .on_hover_text(format!("{:?}", view));
                    if response.clicked() {
                        switch_view(&mut gem_player.ui_state, view);
                    }

                    ui.add_space(4.0);
                }
            });

            center.with_layout(Layout::centered_and_justified(Direction::TopDown), |ui| {
                match gem_player.ui_state.current_view {
                    View::Library => {
                        let tracks_count_and_duration = get_count_and_duration_string_from_tracks(&gem_player.library);
                        ui.add(unselectable_label(tracks_count_and_duration));
                    }
                    View::Queue => {
                        let tracks_count_and_duration = get_count_and_duration_string_from_tracks(&gem_player.player.queue);
                        ui.add(unselectable_label(tracks_count_and_duration));
                    }
                    View::Playlists => {
                        let Some(playlist_key) = &gem_player.ui_state.playlists.selected_playlist_key else {
                            return;
                        };

                        let playlist = gem_player.playlists.get_by_path(playlist_key);

                        let tracks_count_and_duration = get_count_and_duration_string_from_tracks(&playlist.tracks);
                        ui.add(unselectable_label(tracks_count_and_duration));
                    }
                    _ => {}
                }
            });

            right.with_layout(Layout::right_to_left(Align::Center), |ui| match gem_player.ui_state.current_view {
                View::Library => {
                    let search_changed = render_search(ui, &mut gem_player.ui_state.search);
                    if search_changed {
                        gem_player.ui_state.library.cache_dirty = true;
                        gem_player.ui_state.playlists.cache_dirty = true;
                    }

                    let sort_changed = render_sort_and_order_by(
                        ui,
                        &mut gem_player.ui_state.library.sort_by,
                        &mut gem_player.ui_state.library.sort_order,
                    );
                    if sort_changed {
                        gem_player.ui_state.library.cache_dirty = true;
                    }
                }
                View::Queue => {
                    let queue_is_not_empty = !gem_player.player.queue.is_empty();

                    let clear_button = Button::new(icons::ICON_CLEAR_ALL);
                    let response = ui
                        .add_enabled(queue_is_not_empty, clear_button)
                        .on_hover_text("Clear")
                        .on_disabled_hover_text("Queue is empty");
                    if response.clicked() {
                        clear_the_queue(&mut gem_player.player);
                    }
                }
                View::Playlists => {
                    let search_changed = render_search(ui, &mut gem_player.ui_state.search);
                    if search_changed {
                        gem_player.ui_state.library.cache_dirty = true;
                        gem_player.ui_state.playlists.cache_dirty = true;
                    }
                }
                _ => {}
            });
        });
    });
}

fn get_count_and_duration_string_from_tracks(tracks: &[Track]) -> String {
    let duration = calculate_total_duration(tracks);
    let duration_string = format_duration_to_hhmmss(duration);
    format!("{} tracks / {}", tracks.len(), duration_string)
}

fn render_sort_and_order_by(ui: &mut Ui, sort_by: &mut SortBy, sort_order: &mut SortOrder) -> bool {
    let popup_id = ui.make_persistent_id("sort_by_popup");
    let response = ui.button(icons::ICON_FILTER_LIST).on_hover_text("Sort by and order");
    if response.clicked() {
        ui.memory_mut(|mem| mem.toggle_popup(popup_id));
    }

    let mut sort_by_changed = false;
    let mut sort_order_changed = false;

    let close_on_click_outside = popup::PopupCloseBehavior::CloseOnClickOutside;
    popup::popup_above_or_below_widget(ui, popup_id, &response, AboveOrBelow::Above, close_on_click_outside, |ui| {
        ui.set_min_width(100.0);

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

fn render_search(ui: &mut Ui, search_text: &mut String) -> bool {
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

fn unselectable_label(text: impl Into<RichText>) -> Label {
    Label::new(text.into()).selectable(false)
}
