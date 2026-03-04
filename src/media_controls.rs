use anyhow::{Context, Result};
use fully_pub::fully_pub;
use souvlaki::{MediaControlEvent, MediaControls, MediaMetadata, PlatformConfig};
use std::sync::mpsc::{channel, Receiver};

use crate::{commands::Command, track::Track};

#[fully_pub]
struct OSMediaControls {
    receiver: Receiver<Command>,
    controls: MediaControls,
}

pub fn setup_media_controls() -> Result<OSMediaControls> {
    let (sender, receiver) = channel();

    let config = PlatformConfig {
        dbus_name: "com.jamesmoreau.gemplayer", // TODO change this.
        display_name: "Gem Player",
        hwnd: None,
    };

    let mut controls = MediaControls::new(config).context("Failed to initialize media controls")?;

    controls
        .attach(move |event| {
            let command = match event {
                MediaControlEvent::Play => Some(Command::Play),
                MediaControlEvent::Pause => Some(Command::Pause),
                MediaControlEvent::Toggle => Some(Command::PlayPause),
                MediaControlEvent::Next => Some(Command::NextTrack),
                MediaControlEvent::Previous => Some(Command::PreviousTrack),
                _ => None,
            };

            if let Some(cmd) = command {
                let _ = sender.send(cmd);
            }
        })
        .context("Failed to attach media control handler")?;

    Ok(OSMediaControls { controls, receiver })
}

pub fn set_metadata(os_media_controls: &mut OSMediaControls, track: &Track) {
    let _ = os_media_controls.controls.set_metadata(MediaMetadata {
        title: track.title.as_deref(),
        artist: track.artist.as_deref(),
        album: track.album.as_deref(),
        duration: Some(track.duration),
        // cover_url: TODO
        ..Default::default()
    });
}
