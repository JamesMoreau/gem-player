use anyhow::Result;
use eframe::wgpu::rwh::{RawWindowHandle, WindowHandle};
use fully_pub::fully_pub;
use souvlaki::{MediaControlEvent, MediaControls, MediaMetadata, MediaPlayback, MediaPosition, PlatformConfig, SeekDirection};
use std::{
    ffi::c_void,
    sync::mpsc::{self, Receiver},
};

use crate::{
    APP_NAME, GemPlayer,
    artwork_cache::artwork_uri,
    commands::GemCommand,
    player::{Player, get_position},
};

#[derive(Debug)]
pub enum OSMediaControlsState {
    Pending, // Could be waiting for the window handle.
    Initialized(OSMediaControls),
    Failed,
}

#[derive(Debug)]
#[fully_pub]
struct OSMediaControls {
    controls: MediaControls,
    events_receiver: Receiver<MediaControlEvent>,
}

pub fn update_metadata(controls: &mut MediaControls, player: &Player) -> Result<()> {
    let artwork_uri = artwork_uri();

    let metadata = match &player.playing {
        Some(track) => MediaMetadata {
            title: track.title.as_deref(),
            album: track.album.as_deref(),
            artist: track.artist.as_deref(),
            duration: Some(track.duration),
            cover_url: artwork_uri.as_deref(),
        },
        None => MediaMetadata {
            title: None,
            album: None,
            artist: None,
            duration: None,
            cover_url: None,
        },
    };

    controls.set_metadata(metadata)?;

    Ok(())
}

pub fn update_playback(controls: &mut MediaControls, player: &Player) -> Result<()> {
    let backend = player.backend.as_ref();

    let progress = get_position(player).map(MediaPosition);

    let is_paused = backend.is_some_and(|b| b.player.is_paused());

    let playback = if player.playing.is_some() {
        if is_paused {
            MediaPlayback::Paused { progress }
        } else {
            MediaPlayback::Playing { progress }
        }
    } else {
        MediaPlayback::Stopped
    };

    controls.set_playback(playback)?;

    Ok(())
}

pub fn setup_os_media_controls(window_handle: WindowHandle<'_>) -> Result<OSMediaControls> {
    let hwnd = match window_handle.as_raw() {
        RawWindowHandle::Win32(h) => Some(h.hwnd.get() as *mut c_void),
        _ => None,
    };

    let media_config = PlatformConfig {
        dbus_name: "gem_player",
        display_name: APP_NAME,
        hwnd,
    };

    let mut controls = MediaControls::new(media_config)?;

    let (events_sender, events_receiver) = mpsc::channel();
    controls.attach(move |event| {
        let _ = events_sender.send(event);
    })?;

    Ok(OSMediaControls { controls, events_receiver })
}

pub fn poll_media_events(gem: &mut GemPlayer) {
    let OSMediaControlsState::Initialized(osmc) = &gem.os_media_controls else {
        return;
    };

    while let Ok(event) = osmc.events_receiver.try_recv() {
        match event {
            MediaControlEvent::Play => gem.commands.push(GemCommand::Play),
            MediaControlEvent::Pause => gem.commands.push(GemCommand::Pause),
            MediaControlEvent::Toggle => gem.commands.push(GemCommand::TogglePlayback),
            MediaControlEvent::Next => gem.commands.push(GemCommand::NextTrack),
            MediaControlEvent::Previous => gem.commands.push(GemCommand::PreviousTrack),
            MediaControlEvent::Stop => gem.commands.push(GemCommand::Stop),
            MediaControlEvent::Seek(seek_direction) => match seek_direction {
                SeekDirection::Forward => gem.commands.push(GemCommand::NextTrack),
                SeekDirection::Backward => gem.commands.push(GemCommand::PreviousTrack),
            },
            MediaControlEvent::SeekBy(seek_direction, duration) => match seek_direction {
                SeekDirection::Forward => gem.commands.push(GemCommand::SeekForward(duration)),
                SeekDirection::Backward => gem.commands.push(GemCommand::SeekBackward(duration)),
            },
            MediaControlEvent::SetPosition(MediaPosition(duration)) => gem.commands.push(GemCommand::SeekTo(duration)),
            MediaControlEvent::SetVolume(volume) => gem.commands.push(GemCommand::SetVolume(volume as f32)),
            MediaControlEvent::OpenUri(uri) => gem.commands.push(GemCommand::OpenUri(uri)),
            MediaControlEvent::Raise => gem.commands.push(GemCommand::RaiseWindow),
            MediaControlEvent::Quit => gem.commands.push(GemCommand::Quit),
        }
    }
}
