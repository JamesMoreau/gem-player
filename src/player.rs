use crate::track::Track;
use fully_pub::fully_pub;
use rand::seq::SliceRandom;
use rodio::{Decoder, OutputStream, Sink};
use std::io::{self, BufReader, ErrorKind};

#[fully_pub]
pub struct Player {
    playing_track: Option<Track>,

    queue: Vec<Track>,
    history: Vec<Track>,

    repeat: bool,
    muted: bool,
    volume_before_mute: Option<f32>,
    paused_before_scrubbing: Option<bool>, // None if not scrubbing, Some(true) if paused, Some(false) if playing.

    _stream: OutputStream, // Holds the OutputStream to keep it alive
    sink: Sink,            // Controls playback (play, pause, stop, etc.)
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

pub fn shuffle_queue(queue: &mut Vec<Track>) {
    let mut rng = rand::rng();
    queue.shuffle(&mut rng);
}

pub fn move_to_front(queue: &mut Vec<Track>, track: &Track) {
    let index = queue.iter().position(|t| *t == *track).expect("Track not found in queue");
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
