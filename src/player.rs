use crate::track::Track;
use fully_pub::fully_pub;
use rand::seq::SliceRandom;
use rodio::{Decoder, OutputStream, Sink};
use std::io::{self, BufReader, ErrorKind};

#[fully_pub]
pub struct Player {
    queue_cursor: Option<usize>, // Points to the currently playing track in the queue.
    queue: Vec<Track>,

    repeat: bool,
    shuffle: Option<Vec<Track>>, // Used to restore the queue after shuffling. The tracks are what was in front of the cursor.
    muted: bool,
    volume_before_mute: Option<f32>,
    paused_before_scrubbing: Option<bool>, // None if not scrubbing, Some(true) if paused, Some(false) if playing.

    stream: OutputStream, // Holds the OutputStream to keep it alive
    sink: Sink,           // Controls playback (play, pause, stop, etc.)
}

pub fn reset_queue(player: &mut Player) {
    player.queue_cursor = None;
    player.queue.clear();
    player.shuffle = None;
    player.repeat = false;
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
        // If repeat is enabled, reload the current track (no need to move the cursor).
        if let Some(current_index) = player.queue_cursor {
            let track = &player.queue[current_index];
            if let Err(e) = load_and_play(&mut player.sink, track) {
                return Err(e.to_string());
            }
        }
        return Ok(());
    }

    if player.queue.is_empty() {
        return Ok(()); // Nothing to play.
    }

    let next_index = if let Some(cursor) = player.queue_cursor {
        if cursor >= player.queue.len() - 1 {
            return Ok(()); // Already at the end of the queue.
        }
        cursor + 1
    } else {
        0 // If no track is currently playing, start with the first track.
    };

    let next_track = &player.queue[next_index];
    if let Err(e) = load_and_play(&mut player.sink, next_track) {
        return Err(e.to_string());
    }

    player.queue_cursor = Some(next_index);
    Ok(())
}

pub fn play_previous(player: &mut Player) -> Result<(), String> {
    let Some(queue_cursor) = player.queue_cursor else {
        return Err("No track is playing".to_string());
    };

    let previous_index = {
        if player.queue.is_empty() {
            return Err("The queue is empty.".to_string());
        }

        if queue_cursor == 0 {
            return Err("Already at the beginning of the queue.".to_string());
        }

        queue_cursor - 1
    };

    let previous_track = &player.queue[previous_index];
    if let Err(e) = load_and_play(&mut player.sink, previous_track) {
        return Err(e.to_string());
    }

    player.queue_cursor = Some(previous_index);
    Ok(())
}

// TODO: Is this ok to call this function from the UI thread since we are doing heavy events like loading a file?
pub fn load_and_play(sink: &mut Sink, track: &Track) -> io::Result<()> {
    sink.stop(); // Stop the current track if any.

    let file = std::fs::File::open(&track.path)?;

    let source_result = Decoder::new(BufReader::new(file));
    let source = match source_result {
        Err(e) => return Err(io::Error::new(ErrorKind::Other, e.to_string())),
        Ok(source) => source,
    };

    sink.append(source);
    sink.play();

    Ok(())
}

pub fn toggle_shuffle(player: &mut Player) {
    let start_index = player.queue_cursor.unwrap() + 1;
    match player.shuffle.take() {
        Some(unshuffled_queue) => {
            // Restore the queue to its original order.
            player.queue.splice(start_index.., unshuffled_queue);
            player.shuffle = None;
        }
        None => {
            player.shuffle = Some(player.queue[start_index..].to_vec());
            shuffle(&mut player.queue[start_index..]);
        }
    }
}

pub fn shuffle(queue: &mut [Track]) {
    let mut rng = rand::rng();
    queue.shuffle(&mut rng);
}

pub fn clear_the_queue(player: &mut Player) {
    player.queue.clear();
    player.queue_cursor = None;
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
