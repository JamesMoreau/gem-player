use std::{path::PathBuf, time::Duration};

use egui::{Context, OpenUrl, ViewportCommand};
use log::{error, info, warn};
use strum_macros::{Display, EnumString};

use crate::{
    GemPlayer, maybe_play_next, maybe_play_previous,
    os_media_controls::{OSMediaControlsState, update_metadata, update_playback},
    player::{
        enqueue, enqueue_next, get_position, mute_or_unmute, pause, play, replace_queue, seek, set_volume, stop, toggle, toggle_repeat,
        toggle_shuffle,
    },
    playlist::{PlaylistRetrieval, add_to_playlist, remove_from_playlist},
    track::{Track, TrackRetrieval, open_file_location},
    ui::root::format_duration_to_mmss,
};

#[derive(PartialEq, Debug, Clone, EnumString, Display)]
pub enum GemCommand {
    Play,
    Pause,
    TogglePlayback,
    Stop,

    NextTrack,
    PreviousTrack,

    ToggleRepeat,
    ToggleShuffle,

    SeekTo(Duration),
    SeekForward(Duration),
    SeekBackward(Duration),

    SetVolume(f32),
    ToggleMute,

    PlayTrackList {
        track_keys: Vec<PathBuf>,
        start_at: Option<PathBuf>,
    },
    AddTracksToPlaylist {
        playlist_key: PathBuf,
        track_keys: Vec<PathBuf>,
    },
    RemoveTracksFromPlaylist {
        playlist_key: PathBuf,
        track_keys: Vec<PathBuf>
    },
    EnqueueTracks {
        track_keys: Vec<PathBuf>,
    },
    EnqueueTracksNext {
        track_keys: Vec<PathBuf>,
    },
    OpenTrackLocation(PathBuf),

    OpenUri(String),
    ReportIssue,
    RaiseWindow,
    Quit,
}

pub fn execute(ctx: &Context, gem: &mut GemPlayer, command: GemCommand) {
    match command {
        GemCommand::Play => {
            if let Err(e) = play(&mut gem.player) {
                error!("{}", e);
            } else if let OSMediaControlsState::Initialized(osmc) = &mut gem.os_media_controls
                && let Err(e) = update_playback(&mut osmc.controls, &gem.player)
            {
                error!("{}", e);
            }
        }
        GemCommand::Pause => {
            if let Err(e) = pause(&mut gem.player) {
                error!("{}", e);
            } else if let OSMediaControlsState::Initialized(osmc) = &mut gem.os_media_controls
                && let Err(e) = update_playback(&mut osmc.controls, &gem.player)
            {
                error!("{}", e);
            }
        }
        GemCommand::TogglePlayback => {
            if let Err(e) = toggle(&mut gem.player) {
                error!("{}", e);
            } else if let OSMediaControlsState::Initialized(osmc) = &mut gem.os_media_controls
                && let Err(e) = update_playback(&mut osmc.controls, &gem.player)
            {
                error!("{}", e);
            }
        }
        GemCommand::Stop => {
            stop(&mut gem.player);

            if let OSMediaControlsState::Initialized(osmc) = &mut gem.os_media_controls {
                if let Err(e) = update_metadata(&mut osmc.controls, &gem.player) {
                    error!("{}", e);
                }

                if let Err(e) = update_playback(&mut osmc.controls, &gem.player) {
                    error!("{}", e);
                }
            }
        }
        GemCommand::NextTrack => maybe_play_next(ctx, gem),
        GemCommand::PreviousTrack => maybe_play_previous(ctx, gem),
        GemCommand::SeekTo(position) => {
            if let Err(e) = seek(&mut gem.player, position) {
                error!("{}", e);
            } else if let OSMediaControlsState::Initialized(osmc) = &mut gem.os_media_controls
                && let Err(e) = update_playback(&mut osmc.controls, &gem.player)
            {
                error!("{}", e);
            }

            info!("Seeking to {}", format_duration_to_mmss(position));
        }
        GemCommand::ToggleRepeat => toggle_repeat(&mut gem.player),
        GemCommand::ToggleShuffle => toggle_shuffle(&mut gem.player),
        GemCommand::SeekForward(offset) => {
            if let Some(position) = get_position(&gem.player) {
                let new_position = position + offset;
                if let Err(e) = seek(&mut gem.player, new_position) {
                    error!("{}", e);
                } else if let OSMediaControlsState::Initialized(osmc) = &mut gem.os_media_controls
                    && let Err(e) = update_playback(&mut osmc.controls, &gem.player)
                {
                    error!("{}", e);
                }
            } else {
                error!("Unable to retrieve position");
            }
        }
        GemCommand::SeekBackward(offset) => {
            if let Some(position) = get_position(&gem.player) {
                let new_position = position - offset;
                if let Err(e) = seek(&mut gem.player, new_position) {
                    error!("{}", e);
                } else if let OSMediaControlsState::Initialized(osmc) = &mut gem.os_media_controls
                    && let Err(e) = update_playback(&mut osmc.controls, &gem.player)
                {
                    error!("{}", e);
                }
            } else {
                error!("Unable to retrieve position");
            }
        }
        GemCommand::SetVolume(volume) => {
            if let Err(e) = set_volume(&mut gem.player, volume) {
                error!("{}", e);
            }
        }
        GemCommand::ToggleMute => {
            mute_or_unmute(&mut gem.player);
        }
        GemCommand::PlayTrackList { track_keys, start_at } => {
            let tracks: Vec<Track> = track_keys
                .iter()
                .map(|track_key| gem.library.get_by_path(track_key).clone())
                .collect();

            let start_index = start_at
                .as_ref()
                .and_then(|path| track_keys.iter().position(|p| p == path))
                .unwrap_or(0);

            replace_queue(&mut gem.player, &tracks, start_index);

            maybe_play_next(ctx, gem);
        }
        GemCommand::AddTracksToPlaylist { playlist_key, track_keys } => {
            if track_keys.is_empty() {
                warn!("No track(s) were provided for adding to playlist.");
                return;
            }

            let playlist = gem.playlists.get_by_path_mut(&playlist_key);

            let mut added_count = 0;
            for track_key in &track_keys {
                let track = gem.library.get_by_path(track_key);

                if let Err(e) = add_to_playlist(playlist, track.clone()) {
                    error!("Failed to add track to playlist: {}", e);
                } else {
                    added_count += 1;
                }
            }

            gem.ui.playlists.cache_dirty = true;

            if added_count > 0 {
                let message = format!("Added {} track(s) to playlist '{}'.", added_count, playlist.name);
                info!("{}", message);
                gem.ui.toasts.success(message);
            } else {
                gem.ui.toasts.error("No tracks were added.");
            }
        }
        GemCommand::RemoveTracksFromPlaylist { playlist_key, track_keys } => {
            let playlist = gem.playlists.get_by_path_mut(&playlist_key);
            
            if gem.ui.playlists.selected_tracks.is_empty() {
                error!("No track(s) were provided for removing track from playlist.");
                return;
            };

            let mut added_count = 0;
            for track_key in &track_keys {
                if let Err(e) = remove_from_playlist(playlist, track_key) {
                    error!("Failed to remove track from playlist: {}", e);
                } else {
                    added_count += 1;
                }
            }

            gem.ui.playlists.cache_dirty = true;

            if added_count > 0 {
                let message = format!("Removed {} track(s) from playlist '{}'", added_count, playlist.name);
                info!("{}", message);
                gem.ui.toasts.success(message);
            } else {
                gem.ui.toasts.error("No tracks were removed.");
            }
        }
        GemCommand::EnqueueTracks { track_keys } => {
            if track_keys.is_empty() {
                warn!("No track(s) were provided for enqueue.");
                return;
            }

            for track_key in &track_keys {
                let track = gem.library.get_by_path(track_key);
                enqueue(&mut gem.player, track.clone());
            }
        }
        GemCommand::OpenTrackLocation(track_key) => {
            let track = gem.library.get_by_path(&track_key);

            if let Err(e) = open_file_location(track) {
                error!("Failed to open track location: {}", e);
            } else {
                info!("Opening track location: {}", track.path.display());
            }
        }
        GemCommand::EnqueueTracksNext { track_keys } => {
            if track_keys.is_empty() {
                warn!("No track(s) were provided for enqueue next.");
                return;
            }

            for track_key in &track_keys {
                let track = gem.library.get_by_path(track_key);
                enqueue_next(&mut gem.player, track.clone());
            }
        }
        GemCommand::OpenUri(uri) => {
            warn!("OpenUri is not supported: {uri}");
        }
        GemCommand::ReportIssue => {
            let url = format!("{}/issues", env!("CARGO_PKG_REPOSITORY"));
            ctx.open_url(OpenUrl { url, new_tab: true });
        }
        GemCommand::RaiseWindow => ctx.send_viewport_cmd(ViewportCommand::Focus),
        GemCommand::Quit => ctx.send_viewport_cmd(ViewportCommand::Close),
    }
}
