use crate::{play_library, play_playlist, playlist::remove_track, track::Track, GemPlayer};
use fully_pub::fully_pub;
use log::error;
use rand::seq::SliceRandom;
use rodio::{Decoder, OutputStream, Sink};
use std::{
    io::{self, BufReader, ErrorKind},
    path::PathBuf,
    time::Duration,
};

pub enum PlayerAction {
    PlayFromPlaylist { playlist_identifier: PathBuf, track: Track },
    PlayFromLibrary { track: Track },
    AddTrackToQueue { track: Track },
    PlayPrevious,
    PlayNext,
    RemoveTrackFromPlaylist { track: Track, playlist_identifier: PathBuf }, // TODO: change to track identifier
                                                                            // TODO: Potential Actions
                                                                            // PlayNextFromLibraryd
                                                                            // PlayNextFromPlaylist
                                                                            // RemoveTrackFromQueue { track_id: Uuid }
}

#[fully_pub]
pub struct Player {
    playing_track: Option<Track>,
    actions: Vec<PlayerAction>, // Actions get immedietly processed every frame.

    queue: Vec<Track>,
    history: Vec<Track>,

    repeat: bool,
    muted: bool,
    volume_before_mute: Option<f32>,
    paused_before_scrubbing: Option<bool>, // None if not scrubbing, Some(true) if paused, Some(false) if playing.

    _stream: OutputStream, // Holds the OutputStream to keep it alive
    sink: Sink,            // Controls playback (play, pause, stop, etc.)
}

pub fn check_for_next_track(gem_player: &mut GemPlayer) {
    if !gem_player.player.sink.empty() {
        return; // If a track is still playing, do nothing
    }

    let result = play_next(&mut gem_player.player);
    if let Err(e) = result {
        error!("{}", e);
        gem_player.ui_state.toasts.error("Error playing the next track");
    }

    let nothing_left_to_play = gem_player.player.sink.empty() && gem_player.player.queue.is_empty();
    if nothing_left_to_play {
        gem_player.player.playing_track = None;
    }
}

pub fn process_actions(gem_player: &mut GemPlayer) {
    while let Some(action) = gem_player.player.actions.pop() {
        match action {
            PlayerAction::PlayFromPlaylist {
                playlist_identifier,
                track,
            } => play_playlist(gem_player, &playlist_identifier, Some(&track)),
            PlayerAction::PlayFromLibrary { track } => play_library(gem_player, Some(&track)),
            PlayerAction::AddTrackToQueue { track } => add_to_queue(&mut gem_player.player.queue, track),
            PlayerAction::PlayPrevious => maybe_play_previous(gem_player),
            PlayerAction::PlayNext => {
                let result = play_next(&mut gem_player.player);
                if let Err(e) = result {
                    error!("{}", e);
                    gem_player.ui_state.toasts.error("Error playing the next track");
                }
            }
            PlayerAction::RemoveTrackFromPlaylist {
                playlist_identifier,
                track,
            } => {
                let Some(playlist) = gem_player.playlists.iter_mut().find(|p| p.m3u_path == playlist_identifier) else {
                    error!("Unable to find playlist for RemoveTrackFromPlaylist action.");
                    continue;
                };

                let result = remove_track(playlist, &track);
                if let Err(e) = result {
                    error!("{}", e);
                    gem_player.ui_state.toasts.error("Error removing track from playlist");
                }
            }
        }
    }
}

pub fn is_playing(player: &mut Player) -> bool {
    !player.sink.is_paused()
}

pub fn play_or_pause(player: &mut Player) {
    if player.sink.is_paused() {
        player.sink.play()
    } else {
        player.sink.pause()
    }
}

pub fn play_next(player: &mut Player) -> Result<(), String> {
    if player.repeat {
        if let Some(playing_track) = player.playing_track.clone() {
            if let Err(e) = load_and_play(player, playing_track) {
                return Err(e.to_string());
            }
        }
        return Ok(()); // If we are in repeat mode but there is no current track, do nothing!
    }

    let next_track = if player.queue.is_empty() {
        return Ok(()); // Queue is empty, nothing to play
    } else {
        player.queue.remove(0)
    };

    if let Some(playing_track) = player.playing_track.take() {
        player.history.push(playing_track);
    }

    if let Err(e) = load_and_play(player, next_track) {
        return Err(e.to_string());
    }

    Ok(())
}

// If we are near the beginning of the track, we go to the previously played track.
// Otherwise, we seek to the beginning.
// This is what actually gets called by the UI and key command.
pub fn maybe_play_previous(gem_player: &mut GemPlayer) {
    let playback_position = gem_player.player.sink.get_pos().as_secs_f32();
    let rewind_threshold = 5.0;

    if playback_position < rewind_threshold {
        if gem_player.player.history.is_empty() {
            // No previous track to play, just restart the current track
            if let Err(e) = gem_player.player.sink.try_seek(Duration::ZERO) {
                error!("Error rewinding track: {:?}", e);
            }
            gem_player.player.sink.play();
        } else if let Err(e) = play_previous(&mut gem_player.player) {
            error!("{}", e);
            gem_player.ui_state.toasts.error("Error playing the previous track");
        }
    } else {
        if let Err(e) = gem_player.player.sink.try_seek(Duration::ZERO) {
            error!("Error rewinding track: {:?}", e);
        }
        gem_player.player.sink.play();
    }
}

pub fn play_previous(player: &mut Player) -> Result<(), String> {
    let Some(previous_track) = player.history.pop() else {
        return Ok(()); // No previous track? Do nothing.
    };

    if let Some(playing_track) = player.playing_track.take() {
        player.queue.insert(0, playing_track);
    }

    if let Err(e) = load_and_play(player, previous_track) {
        return Err(e.to_string());
    }

    Ok(())
}

// TODO: Is this ok to call this function from the UI thread since we are doing heavy events like loading a file?
pub fn load_and_play(player: &mut Player, track: Track) -> io::Result<()> {
    player.sink.stop(); // Stop the current track if any.
    player.playing_track = None;

    let file = std::fs::File::open(&track.path)?;

    let source_result = Decoder::new(BufReader::new(file));
    let source = match source_result {
        Ok(source) => source,
        Err(e) => return Err(io::Error::new(ErrorKind::Other, e.to_string())),
    };

    player.playing_track = Some(track);
    player.sink.append(source);
    player.sink.play();

    Ok(())
}

pub fn add_to_queue(queue: &mut Vec<Track>, track: Track) {
    queue.push(track);
}

pub fn add_next_to_queue(queue: &mut Vec<Track>, track: Track) {
    queue.insert(0, track);
}

pub fn remove_from_queue(queue: &mut Vec<Track>, index: usize) {
    queue.remove(index);
}

pub fn shuffle_queue(queue: &mut Vec<Track>) {
    let mut rng = rand::rng();
    queue.shuffle(&mut rng);
}

pub fn move_to_front(queue: &mut Vec<Track>, index: usize) {
    if index == 0 || index >= queue.len() {
        return;
    }

    let track = queue.remove(index);
    queue.insert(0, track);
}

pub fn mute_or_unmute(player: &mut Player) {
    let mut volume = player.sink.volume();

    player.muted = !player.muted;

    if player.muted {
        player.volume_before_mute = Some(volume);
        volume = 0.0;
    } else if let Some(v) = player.volume_before_mute {
        volume = v;
    }

    player.sink.set_volume(volume);
}

pub fn adjust_volume_by_percentage(player: &mut Player, percentage: f32) {
    let current_volume = player.sink.volume();

    let min_volume = 0.0;
    let max_volume = 1.0;

    let new_volume = (current_volume + percentage).clamp(min_volume, max_volume);
    player.sink.set_volume(new_volume);
}
