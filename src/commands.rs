use std::time::Duration;

use egui::{OpenUrl, Ui};
use log::error;
use strum_macros::{Display, EnumString};

use crate::{
    GemPlayer, maybe_play_next, maybe_play_previous,
    os_media_controls::{OSMediaControlsState, update_playback},
    player::{adjust_volume_by_delta, pause, play, toggle},
};

#[derive(PartialEq, Debug, Clone, Copy, EnumString, Display)]
pub enum Command {
    Play,
    Pause,
    Toggle,
    Stop,

    NextTrack,
    PreviousTrack,

    SeekTo(Duration),

    SetVolume(f32),
    VolumeUp,
    VolumeDown,
    // ToggleMute
    RaiseWindow,
    Quit,
    ReportIssue,
    // OpenFile,
}

pub fn execute(ui: &mut Ui, gem: &mut GemPlayer, command: Command) {
    match command {
        Command::Toggle => {
            if let Err(e) = toggle(&mut gem.player) {
                error!("{}", e);
            } else if let OSMediaControlsState::Initialized(osmc) = &mut gem.os_media_controls
                && let Err(e) = update_playback(&mut osmc.controls, &gem.player)
            {
                error!("{}", e);
            }
        }
        Command::NextTrack => maybe_play_next(ui, gem),
        Command::PreviousTrack => maybe_play_previous(ui, gem),
        Command::VolumeUp => {
            if let Some(backend) = &mut gem.player.backend {
                adjust_volume_by_delta(&mut backend.player, 0.1);
            }
        }
        Command::VolumeDown => {
            if let Some(backend) = &mut gem.player.backend {
                adjust_volume_by_delta(&mut backend.player, -0.1);
            }
        }
        Command::ReportIssue => {
            let url = format!("{}/issues", env!("CARGO_PKG_REPOSITORY"));
            ui.open_url(OpenUrl { url, new_tab: true });
        }
        Command::Play => {
            if let Err(e) = play(&mut gem.player) {
                error!("{}", e);
            } else if let OSMediaControlsState::Initialized(osmc) = &mut gem.os_media_controls
                && let Err(e) = update_playback(&mut osmc.controls, &gem.player)
            {
                error!("{}", e);
            }
        }
        Command::Pause => {
            if let Err(e) = pause(&mut gem.player) {
                error!("{}", e);
            } else if let OSMediaControlsState::Initialized(osmc) = &mut gem.os_media_controls
                && let Err(e) = update_playback(&mut osmc.controls, &gem.player)
            {
                error!("{}", e);
            }
        }
        _ => todo!("Command not yet implemented"),
    }
}
