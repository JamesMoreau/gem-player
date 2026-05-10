use std::time::Duration;

use egui::{OpenUrl, Ui};
use log::error;
use strum_macros::{Display, EnumString};

use crate::{
    GemPlayer, maybe_play_next, maybe_play_previous,
    os_media_controls::{OSMediaControlsState, update_playback},
    player::{adjust_volume_by_delta, pause, play, seek, set_volume, toggle},
};

#[derive(PartialEq, Debug, Clone, Copy, EnumString, Display)]
pub enum Command {
    Play,
    Pause,
    Toggle,
    Stop,

    Next,
    Previous,
    SeekTo(Duration),

    SetVolume(f32),
    VolumeUp,
    VolumeDown,
    // ToggleMute,
    RaiseWindow,
    Quit,
    ReportIssue,
    // OpenFile,
}

pub fn execute(ui: &mut Ui, gem: &mut GemPlayer, command: Command) {
    match command {
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
        Command::Toggle => {
            if let Err(e) = toggle(&mut gem.player) {
                error!("{}", e);
            } else if let OSMediaControlsState::Initialized(osmc) = &mut gem.os_media_controls
                && let Err(e) = update_playback(&mut osmc.controls, &gem.player)
            {
                error!("{}", e);
            }
        }
        Command::Next => maybe_play_next(ui, gem),
        Command::Previous => maybe_play_previous(ui, gem),
        Command::SeekTo(position) => {
            if let Err(e) = seek(&mut gem.player, position) {
                error!("{}", e);
            } else if let OSMediaControlsState::Initialized(osmc) = &mut gem.os_media_controls
                && let Err(e) = update_playback(&mut osmc.controls, &gem.player)
            {
                error!("{}", e);
            }
        }
        Command::SetVolume(volume) => {
            if let Err(e) = set_volume(&mut gem.player, volume) {
                error!("{}", e);
            }
        }
        Command::VolumeUp => {
            if let Err(e) = adjust_volume_by_delta(&mut gem.player, 0.1) {
                error!("{}", e);
            }
        }
        Command::VolumeDown => {
            if let Err(e) = adjust_volume_by_delta(&mut gem.player, -0.1) {
                error!("{}", e);
            }
        }
        Command::ReportIssue => {
            let url = format!("{}/issues", env!("CARGO_PKG_REPOSITORY"));
            ui.open_url(OpenUrl { url, new_tab: true });
        }
        _ => todo!("Command not yet implemented"),
    }
}
