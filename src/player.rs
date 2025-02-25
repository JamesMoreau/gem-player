use std::{
    io::{self, BufReader, ErrorKind},
    path::{Path, PathBuf},
    time::Duration,
};

use eframe::egui::{Context, Event, Key};
use fully_pub::fully_pub;
use indexmap::IndexMap;
use lazy_static::lazy_static;
use log::{error, info};
use rand::seq::SliceRandom;
use rodio::{Decoder, OutputStream, Sink};
use uuid::Uuid;

use crate::{
    playlist::{find, find_mut, read_all_from_a_directory, remove_track, Playlist},
    track::{read_music_from_a_directory, Track},
    ui::UIState,
};

pub const LIBRARY_DIRECTORY_STORAGE_KEY: &str = "library_directory";
pub const THEME_STORAGE_KEY: &str = "theme";

pub const SUPPORTED_AUDIO_FILE_TYPES: [&str; 6] = ["mp3", "m4a", "wav", "flac", "ogg", "opus"];

#[fully_pub]
pub struct GemPlayer {
    ui_state: UIState,

    library: Vec<Track>,                // All the tracks stored in the user's music directory.
    library_directory: Option<PathBuf>, // The directory where music is stored.
    playlists: Vec<Playlist>,

    player: Player,
}

pub enum PlayerAction {
    PlayFromPlaylist { playlist_id: Uuid, track: Track },
    PlayFromLibrary { track: Track },
    AddTrackToQueue { track: Track },
    PlayPrevious,
    PlayNext,
    RemoveTrackFromPlaylist { track: Track, playlist_id: Uuid },
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

pub fn process_player_actions(gem_player: &mut GemPlayer) {
    while let Some(action) = gem_player.player.actions.pop() {
        match action {
            PlayerAction::PlayFromPlaylist { playlist_id, track } => play_playlist_from_track(gem_player, playlist_id, &track),
            PlayerAction::PlayFromLibrary { track } => play_library_from_track(gem_player, &track),
            PlayerAction::AddTrackToQueue { track } => add_to_queue(&mut gem_player.player.queue, track),
            PlayerAction::PlayPrevious => maybe_play_previous(gem_player),
            PlayerAction::PlayNext => {
                let result = play_next(&mut gem_player.player);
                if let Err(e) = result {
                    error!("{}", e);
                    gem_player.ui_state.toasts.error("Error playing the next track");
                }
            }
            PlayerAction::RemoveTrackFromPlaylist { playlist_id, track } => {
                let Some(playlist) = find_mut(playlist_id, &mut gem_player.playlists) else {
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

pub fn read_music_and_playlists_from_directory(directory: &Path) -> (Vec<Track>, Vec<Playlist>) {
    let mut library = Vec::new();
    let mut playlists = Vec::new();

    match read_music_from_a_directory(directory) {
        Ok(found_tracks) => {
            library = found_tracks;
        }
        Err(e) => {
            error!("{}", e);
        }
    }

    match read_all_from_a_directory(directory) {
        Ok(found_playlists) => {
            playlists = found_playlists;
        }
        Err(e) => {
            error!("{}", e);
        }
    }

    info!(
        "Loaded library from {:?}: {} tracks, {} playlists.",
        directory,
        library.len(),
        playlists.len()
    );

    (library, playlists)
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
            if let Err(e) = load_and_play_track(player, playing_track) {
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

    if let Err(e) = load_and_play_track(player, next_track) {
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

    if let Err(e) = load_and_play_track(player, previous_track) {
        return Err(e.to_string());
    }

    Ok(())
}

// TODO: Is this ok to call this function from the UI thread since we are doing heavy events like loading a file?
pub fn load_and_play_track(player: &mut Player, track: Track) -> io::Result<()> {
    player.sink.stop(); // Stop the current track if any.
    player.playing_track = None;

    let file = std::fs::File::open(&track.file_path)?;

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

pub fn move_track_to_front(queue: &mut Vec<Track>, index: usize) {
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

pub fn play_library_from_track(gem_player: &mut GemPlayer, track: &Track) {
    gem_player.player.history.clear();
    gem_player.player.queue.clear();

    // Add all the other tracks to the queue.
    for t in &gem_player.library {
        if track == t {
            continue;
        }

        gem_player.player.queue.push(t.clone());
    }

    let result = load_and_play_track(&mut gem_player.player, track.clone());
    if let Err(e) = result {
        error!("{}", e);
        gem_player
            .ui_state
            .toasts
            .error(format!("Error playing {}", track.title.as_deref().unwrap_or("Unknown")));
    }
}

pub fn play_playlist_from_track(gem_player: &mut GemPlayer, playlist_id: Uuid, track: &Track) {
    gem_player.player.history.clear();
    gem_player.player.queue.clear();

    let Some(playlist) = find(playlist_id, &gem_player.playlists) else {
        error!("Playlist not found.");
        return;
    };

    // Add all the other tracks to the queue.
    for t in &playlist.tracks {
        if track == t {
            continue;
        }

        gem_player.player.queue.push(t.clone());
    }

    let result = load_and_play_track(&mut gem_player.player, track.clone());
    if let Err(e) = result {
        error!("{}", e);
        gem_player
            .ui_state
            .toasts
            .error(format!("Error playing {}", track.title.as_deref().unwrap_or("Unknown")));
    }
}

lazy_static! {
    pub static ref KEY_COMMANDS: IndexMap<Key, &'static str> = {
        let mut map = IndexMap::new();

        map.insert(Key::Space, "Play/Pause");
        map.insert(Key::ArrowLeft, "Previous");
        map.insert(Key::ArrowRight, "Next");
        map.insert(Key::ArrowUp, "Volume Up");
        map.insert(Key::ArrowDown, "Volume Down");
        map.insert(Key::M, "Mute/Unmute");

        map
    };
}

pub fn handle_key_commands(ctx: &Context, player: &mut Player) {
    if ctx.wants_keyboard_input() {
        return;
    }

    ctx.input(|i| {
        for event in &i.events {
            if let Event::Key {
                key,
                pressed: true,
                physical_key: _,
                repeat: _,
                modifiers: _,
            } = event
            {
                let Some(binding) = KEY_COMMANDS.get(key) else {
                    continue;
                };

                info!("Key pressed: {}", binding);

                match key {
                    Key::Space => play_or_pause(player),
                    Key::ArrowLeft => player.actions.push(PlayerAction::PlayPrevious),
                    Key::ArrowRight => player.actions.push(PlayerAction::PlayNext),
                    Key::ArrowUp => adjust_volume_by_percentage(player, 0.1),
                    Key::ArrowDown => adjust_volume_by_percentage(player, -0.1),
                    Key::M => mute_or_unmute(player),
                    _ => {}
                }
            }
        }
    });
}
