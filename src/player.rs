use crate::{
    track::{extract_artwork_from_file, Track},
    visualizer::{visualizer_source, VisualizerCommand},
};
use fully_pub::fully_pub;
use log::error;
use rand::seq::SliceRandom;
use rodio::{
    cpal::{default_host, traits::HostTrait},
    Decoder, Device, DeviceTrait, MixerDeviceSink, DeviceSinkBuilder, Source,
};
use std::{
    fs::File,
    io::{self, Seek, SeekFrom},
    path::Path,
    sync::mpsc::{Receiver, Sender},
    time::Duration,
};

#[fully_pub]
struct Player {
    history: Vec<Track>, // In chronological order. The most recently played track is at the end.
    playing: Option<Track>,
    queue: Vec<Track>, // In the order the tracks will be played.

    repeat: bool,
    shuffle: Option<Vec<Track>>, // Used to restore the queue after shuffling. The tracks are what was in front of the cursor.
    muted: bool,
    volume_before_mute: Option<f32>,
    paused_before_scrubbing: Option<bool>, // None if not scrubbing, Some(true) if paused, Some(false) if playing.

    backend: Option<AudioBackend>,

    raw_artwork: Option<Vec<u8>>,
    visualizer: VisualizerState,
}

#[fully_pub]
struct AudioBackend {
    device: Device,
    stream: MixerDeviceSink, // Holds the MixerDeviceSink to keep it alive
    player: rodio::Player,           // Controls playback (play, pause, stop, etc.)
}

#[fully_pub]
struct VisualizerState {
    command_sender: Sender<VisualizerCommand>,
    bands_receiver: Receiver<Vec<f32>>,
    display_bands: Vec<f32>,
}

pub fn build_audio_backend_from_device(device: Device) -> Result<AudioBackend, String> {
    let builder = DeviceSinkBuilder::from_device(device.clone())
        .map_err(|e| e.to_string())?
        .with_error_callback(|e| {
            error!("Stream error: {}", e);
        });

    let stream = builder.open_sink_or_fallback().map_err(|e| e.to_string())?;

    let player = rodio::Player::connect_new(stream.mixer());
    player.pause();

    Ok(AudioBackend { device, player, stream })
}

pub fn get_audio_output_devices_and_names() -> Vec<(Device, String)> {
    let mut output_devices_and_names = Vec::new();

    let host = default_host();
    let devices_result = host.output_devices();
    if let Ok(devices) = devices_result {
        for device in devices {
            if let Ok(description) = device.description() {
                output_devices_and_names.push((device, description.name().to_owned()));
            }
        }
    }

    output_devices_and_names
}

/// Switches the audio backend to a new device while preserving playback state.
pub fn switch_audio_devices(player: &mut Player, new_device: Device) -> Result<(), String> {
    let maybe_backend = player.backend.as_ref();
    let maybe_playing_track = player.playing.clone();

    // In order to make the transition smooth, we need to reload the previous playback state onto the new backend.
    let (was_paused, previous_volume, previous_playback_position) = maybe_backend
        .map(|b| (b.player.is_paused(), b.player.volume(), b.player.get_pos()))
        .unwrap_or((true, 0.5, Duration::ZERO));

    match build_audio_backend_from_device(new_device.clone()) {
        Ok(new_backend) => {
            player.backend = Some(new_backend);

            if let Some(playing) = maybe_playing_track {
                load_and_play(player, &playing).map_err(|e| format!("Unable to play previous player's source: {}", e))?;

                if let Some(backend) = &player.backend {
                    if was_paused {
                        backend.player.pause();
                    }

                    backend.player.set_volume(previous_volume);

                    backend
                        .player
                        .try_seek(previous_playback_position)
                        .map_err(|_| "Unable to seek to previous player's position.".to_string())?;
                }
            }

            Ok(())
        }
        Err(e) => Err(e.to_string()),
    }
}

pub fn clear_the_queue(player: &mut Player) {
    player.history.clear();
    player.queue.clear();
    player.shuffle = None;
}

pub fn play_or_pause(player: &mut rodio::Player) {
    if player.is_paused() {
        player.play()
    } else {
        player.pause()
    }
}

pub fn play_next(player: &mut Player) -> Result<(), String> {
    if player.repeat {
        if let Some(playing) = player.playing.clone() {
            return load_and_play(player, &playing).map_err(|e| e.to_string());
        }
    }

    if player.queue.is_empty() {
        player.playing = None;
        return Ok(()); // Nothing to play
    }

    if let Some(current) = player.playing.take() {
        player.history.push(current);
    }

    if let Some(next_track) = player.queue.first().cloned() {
        player.queue.remove(0);
        load_and_play(player, &next_track).map_err(|e| e.to_string())?;
        player.playing = Some(next_track);
    }

    Ok(())
}

pub fn play_previous(player: &mut Player) -> Result<(), String> {
    let Some(previous) = player.history.pop() else {
        return Err("There is no previous track to play.".to_owned());
    };

    if let Err(e) = load_and_play(player, &previous) {
        return Err(e.to_string());
    }

    if let Some(playing) = player.playing.take() {
        enqueue_next(player, playing);
    }

    player.playing = Some(previous);
    Ok(())
}

pub fn load_and_play(player: &mut Player, track: &Track) -> io::Result<()> {
    let Some(backend) = &player.backend else {
        return Err(io::Error::other("No audio backend available"));
    };

    backend.player.stop(); // Stop the current track if any.

    let mut file = File::open(&track.path)?;

    let maybe_artwork = extract_artwork_from_file(&mut file)?;
    player.raw_artwork = maybe_artwork;

    file.seek(SeekFrom::Start(0))?; // Reset the file cursor since accessing artwork moves it forward.

    let decoder_result = Decoder::try_from(file);
    let decoder = match decoder_result {
        Err(e) => return Err(io::Error::other(e.to_string())),
        Ok(d) => d,
    };

    let sample_rate = decoder.sample_rate();
    let result = player.visualizer.command_sender.send(VisualizerCommand::SampleRate(sample_rate));
    if let Err(e) = result {
        error!("Visualizer channel error: {e}. Continuing playback anyway.");
    }

    let visualizer_source = visualizer_source(decoder, player.visualizer.command_sender.clone());
    backend.player.append(visualizer_source);
    backend.player.play();

    Ok(())
}

pub fn play_in_order(player: &mut Player, tracks: &[Track], starting_track: Option<&Path>) -> Result<(), String> {
    clear_the_queue(player);

    let start_index = starting_track
        .and_then(|path| tracks.iter().position(|track| track.path == path))
        .unwrap_or(0);

    // Queue tracks from the starting track, wrapping around to the beginning.
    let ordered = tracks[start_index..].iter().chain(&tracks[..start_index]);

    for track in ordered {
        player.queue.push(track.clone());
    }

    play_next(player)
}

pub fn toggle_shuffle(player: &mut Player) {
    match player.shuffle.take() {
        Some(unshuffled_queue) => {
            player.queue = unshuffled_queue; // Restore the queue to its original order.
        }
        None => {
            let original_queue = player.queue.clone();
            player.shuffle = Some(original_queue);
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
