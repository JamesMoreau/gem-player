use crate::{
    track::{extract_artwork_from_file, Track},
    visualizer::{visualizer_source, VisualizerCommand},
};
use anyhow::{bail, Context, Result};
use fully_pub::fully_pub;
use log::error;
use rand::seq::SliceRandom;
use rodio::{Decoder, Device, DeviceSinkBuilder, MixerDeviceSink, Source};
use std::{
    fs::File,
    io::{Seek, SeekFrom},
    path::Path,
    sync::mpsc::{Receiver, Sender},
};

#[fully_pub]
struct Player {
    history: Vec<Track>, // In chronological order. The most recently played track is at the end.
    playing: Option<Track>,
    queue: Vec<Track>, // In the order the tracks will be played.

    repeat: bool,
    shuffle: Option<Vec<Track>>, // Used to restore the queue after shuffling. The tracks are what was in front of the cursor.
    paused_before_scrubbing: Option<bool>, // None if not scrubbing, Some(true) if paused, Some(false) if playing.

    backend: Option<AudioBackend>,
    muted: bool,
    volume_before_mute: Option<f32>,

    raw_artwork: Option<Vec<u8>>,
    visualizer: VisualizerState,
}

#[fully_pub]
struct AudioBackend {
    device: Device,
    stream: MixerDeviceSink, // Holds the MixerDeviceSink to keep it alive
    player: rodio::Player,   // Controls playback (play, pause, stop, etc.)
}

#[fully_pub]
struct VisualizerState {
    command_sender: Sender<VisualizerCommand>,
    bands_receiver: Receiver<Vec<f32>>,
    display_bands: Vec<f32>,
}

pub enum TrackTransition {
    Unchanged,
    Changed,
}

pub fn play_next(player: &mut Player) -> Result<TrackTransition> {
    if player.repeat {
        if let Some(playing) = player.playing.clone() {
            play_track(player, playing)?;
            return Ok(TrackTransition::Unchanged);
        }
    }

    if player.queue.is_empty() {
        player.playing = None;
        return Ok(TrackTransition::Changed); // Nothing to play
    }

    if let Some(current) = player.playing.take() {
        player.history.push(current);
    }

    let next_track = player.queue.remove(0);

    play_track(player, next_track)?;

    Ok(TrackTransition::Changed)
}

pub fn play_previous(player: &mut Player) -> Result<TrackTransition> {
    let Some(previous) = player.history.pop() else {
        bail!("There is no previous track to play.");
    };

    if let Some(playing) = player.playing.take() {
        enqueue_next(player, playing);
    }

    play_track(player, previous)?;

    Ok(TrackTransition::Changed)
}

fn play_track(player: &mut Player, track: Track) -> Result<()> {
    let Some(backend) = &player.backend else {
        bail!("No audio backend available");
    };

    backend.player.stop(); // Stop the current track if any.

    let mut file = File::open(&track.path).with_context(|| format!("Failed to open audio file at {:?}", track.path))?;

    let artwork = extract_artwork_from_file(&mut file);
    file.seek(SeekFrom::Start(0))
        .context("Failed to reset file cursor after extracting artwork")?; // Reset the file cursor since accessing artwork moves it forward.

    let decoder = Decoder::try_from(file).with_context(|| format!("Failed to decode audio file {:?}", track.path))?;

    let sample_rate = decoder.sample_rate();
    if let Err(e) = player.visualizer.command_sender.send(VisualizerCommand::SampleRate(sample_rate)) {
        error!("Visualizer channel error: {e}. Continuing playback anyway.");
    }

    let visualizer_source = visualizer_source(decoder, player.visualizer.command_sender.clone());
    backend.player.append(visualizer_source);
    backend.player.play();

    player.raw_artwork = artwork;
    player.playing = Some(track);

    Ok(())
}

pub fn play_or_pause(player: &mut rodio::Player) {
    if player.is_paused() {
        player.play()
    } else {
        player.pause()
    }
}

pub fn add_to_queue_in_order(player: &mut Player, tracks: &[Track], starting_track: Option<&Path>) {
    clear_the_queue(player);

    let start_index = starting_track
        .and_then(|path| tracks.iter().position(|track| track.path == path))
        .unwrap_or(0);

    // Queue tracks from the starting track, wrapping around to the beginning.
    let ordered = tracks[start_index..].iter().chain(&tracks[..start_index]);

    for track in ordered {
        player.queue.push(track.clone());
    }
}


pub fn clear_the_queue(player: &mut Player) {
    player.history.clear();
    player.queue.clear();
    player.shuffle = None;
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

pub fn toggle_shuffle(player: &mut Player) {
    match player.shuffle.take() {
        Some(unshuffled_queue) => {
            player.queue = unshuffled_queue; // Restore the queue to its original order.
        }
        None => {
            let original_queue = player.queue.clone();
            player.shuffle = Some(original_queue);

            let mut rng = rand::rng();
            player.queue.shuffle(&mut rng);
        }
    }
}

pub fn build_audio_backend_from_device(device: Device) -> Result<AudioBackend> {
    let builder = DeviceSinkBuilder::from_device(device.clone())
        .context("Failed to create DeviceSinkBuilder from device")?
        .with_error_callback(|e| {
            error!("Stream error: {}", e);
        });

    let stream = builder.open_sink_or_fallback().context("Failed to open audio sink or fallback")?;

    let player = rodio::Player::connect_new(stream.mixer());
    player.pause();

    Ok(AudioBackend { device, player, stream })
}

pub fn mute_or_unmute(player: &mut Player) {
    player.muted = !player.muted;

    let mut target_volume = 0.0;

    if player.muted {
        if let Some(backend) = &player.backend {
            player.volume_before_mute = Some(backend.player.volume());
        }
        target_volume = 0.0;
    } else if let Some(v) = player.volume_before_mute {
        target_volume = v;
    }

    if let Some(backend) = &player.backend {
        backend.player.set_volume(target_volume);
    }
}

pub fn adjust_volume_by_percentage(player: &mut rodio::Player, percentage: f32) {
    let current_volume = player.volume();

    let min_volume = 0.0;
    let max_volume = 1.0;

    let new_volume = (current_volume + percentage).clamp(min_volume, max_volume);
    player.set_volume(new_volume);
}
