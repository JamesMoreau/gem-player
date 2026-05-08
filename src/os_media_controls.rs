use anyhow::Result;
use eframe::wgpu::rwh::{RawWindowHandle, WindowHandle};
use fully_pub::fully_pub;
use souvlaki::{MediaControlEvent, MediaControls, MediaMetadata, MediaPlayback, MediaPosition, PlatformConfig};
use std::{
    ffi::c_void,
    sync::mpsc::{self, Receiver},
};

use crate::{
    APP_NAME,
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
    let metadata = match &player.playing {
        Some(track) => MediaMetadata {
            title: track.title.as_deref(),
            album: track.album.as_deref(),
            artist: track.artist.as_deref(),
            duration: Some(track.duration),
            cover_url: Some("https://c.pxhere.com/photos/34/c1/souvlaki_authentic_greek_greek_food_mezes-497780.jpg!d"),
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

pub fn handle_media_events(os_media_controls: &mut OSMediaControls, player: &mut Player) {
    while let Ok(event) = os_media_controls.events_receiver.try_recv() {
        match event {
            MediaControlEvent::Play => todo!(),
            MediaControlEvent::Pause => todo!(),
            MediaControlEvent::Toggle => todo!(),
            MediaControlEvent::Next => todo!(),
            MediaControlEvent::Previous => todo!(),
            MediaControlEvent::Stop => todo!(),
            MediaControlEvent::Seek(seek_direction) => todo!(),
            MediaControlEvent::SeekBy(seek_direction, duration) => todo!(),
            MediaControlEvent::SetPosition(media_position) => todo!(),
            MediaControlEvent::SetVolume(_) => todo!(),
            MediaControlEvent::OpenUri(_) => todo!(),
            MediaControlEvent::Raise => todo!(),
            MediaControlEvent::Quit => todo!(),
        }
    }
}
