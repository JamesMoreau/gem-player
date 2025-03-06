use crate::track::Track;
use fully_pub::fully_pub;
use rand::seq::SliceRandom;
use rodio::{Decoder, OutputStream, Sink};
use std::io::{self, BufReader, ErrorKind};

#[fully_pub]
pub struct Player {
    history: Vec<Track>,
    playing: Option<Track>,
    queue: Vec<Track>,

    repeat: bool,
    shuffle: Option<Vec<Track>>, // Used to restore the queue after shuffling. The tracks are what was in front of the cursor.
    muted: bool,
    volume_before_mute: Option<f32>,
    paused_before_scrubbing: Option<bool>, // None if not scrubbing, Some(true) if paused, Some(false) if playing.

    stream: OutputStream, // Holds the OutputStream to keep it alive
    sink: Sink,           // Controls playback (play, pause, stop, etc.)
}

pub fn clear_the_queue(player: &mut Player) {
    player.history.clear();
    player.queue.clear();
    player.shuffle = None;
    player.repeat = false;
}

pub fn play_or_pause(player: &mut Player) {
    if player.sink.is_paused() {
        player.sink.play()
    } else {
        player.sink.pause()
    }
}

pub fn play_next(player: &mut Player) -> Result<(), String> { // TODO: should this error if ther is nothing left to play?
    if player.repeat {
        if let Some(ref playing) = player.playing {
            // If repeat is enabled, just restart the current track.
            if let Err(e) = load_and_play(&mut player.sink, playing) {
                return Err(e.to_string())
            };
            return Ok(());
        }
    }
    
    if let Some(current) = player.playing.take() {
        player.history.push(current);
    }
    
    if let Some(next_track) = player.queue.first().cloned() {
        player.queue.remove(0);
        if let Err(e) = load_and_play(&mut player.sink, &next_track) {
            return Err(e.to_string());
        }
        player.playing = Some(next_track);
    }
    
    Ok(())
}

pub fn play_previous(player: &mut Player) -> Result<(), String> {
    let Some(previous) = player.history.pop() else { // TODO: should history start from the beginning or end?
        return Err("There is no previous track to play.".to_owned());
    };

    if let Err(e) = load_and_play(&mut player.sink, &previous) {
        return Err(e.to_string());
    }

    player.playing = Some(previous);
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
    match player.shuffle.take() {
        Some(unshuffled_queue) => {
            player.queue = unshuffled_queue; // Restore the queue to its original order.
        }
        None => {
            player.shuffle = Some(player.queue.clone()); // Save the original queue.
            shuffle(&mut player.queue);
        }
    }
}

pub fn remove_from_queue(player: &mut Player, index: usize) {
    player.queue.remove(index);
}

pub fn move_to_position(player: &mut Player, from: usize, to: usize) {
    let track = player.queue.remove(from);
    player.queue.insert(to, track);
}

pub fn enqueue_next(player: &mut Player, track: Track) {
    player.queue.insert(0, track);
}

pub fn enqueue(player: &mut Player, track: Track) {
    player.queue.push(track);
}

pub fn shuffle(queue: &mut [Track]) {
    let mut rng = rand::rng();
    queue.shuffle(&mut rng);
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
