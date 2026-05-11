use std::time::Duration;

use egui::{OpenUrl, Ui};
use log::error;
use strum_macros::{Display, EnumString};

use crate::{
    GemPlayer, maybe_play_next, maybe_play_previous,
    os_media_controls::{OSMediaControlsState, update_playback},
    player::{get_position, pause, play, seek, set_volume, toggle},
};

#[derive(PartialEq, Debug, Clone, Copy, EnumString, Display)]
pub enum Command {
    Play,
    Pause,
    TogglePlayback,

    NextTrack,
    PreviousTrack,

    SeekTo(Duration),
    SeekForward(Duration),
    SeekBackward(Duration),

    SetVolume(f32),

    ReportIssue,
    RaiseWindow,
    Quit,
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
        Command::TogglePlayback => {
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
        Command::SeekTo(position) => {
            if let Err(e) = seek(&mut gem.player, position) {
                error!("{}", e);
            } else if let OSMediaControlsState::Initialized(osmc) = &mut gem.os_media_controls
                && let Err(e) = update_playback(&mut osmc.controls, &gem.player)
            {
                error!("{}", e);
            }
        }
        Command::SeekForward(offset) => {
            if let Some(position) = get_position(&gem.player) {
                let new_position = position + offset;
                if let Err(e) = seek(&mut gem.player, new_position) {
                    error!("{}", e);
                } else if let OSMediaControlsState::Initialized(osmc) = &mut gem.os_media_controls
                    && let Err(e) = update_playback(&mut osmc.controls, &gem.player)
                {
                    error!("{}", e);
                }
            } else {
                error!("Unable to retrieve position");
            }
        }
        Command::SeekBackward(offset) => {
            if let Some(position) = get_position(&gem.player) {
                let new_position = position - offset;
                if let Err(e) = seek(&mut gem.player, new_position) {
                    error!("{}", e);
                } else if let OSMediaControlsState::Initialized(osmc) = &mut gem.os_media_controls
                    && let Err(e) = update_playback(&mut osmc.controls, &gem.player)
                {
                    error!("{}", e);
                }
            } else {
                error!("Unable to retrieve position");
            }
        }
        Command::SetVolume(volume) => {
            if let Err(e) = set_volume(&mut gem.player, volume) {
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
