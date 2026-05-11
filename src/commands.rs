use std::time::Duration;

use egui::{Context, OpenUrl, ViewportCommand};
use log::{error, warn};
use strum_macros::{Display, EnumString};

use crate::{
    GemPlayer, maybe_play_next, maybe_play_previous,
    os_media_controls::{OSMediaControlsState, update_metadata, update_playback},
    player::{get_position, pause, play, seek, set_volume, stop, toggle},
};

#[derive(PartialEq, Debug, Clone, EnumString, Display)]
pub enum GemCommand {
    Play,
    Pause,
    TogglePlayback,
    Stop,

    NextTrack,
    PreviousTrack,

    SeekTo(Duration),
    SeekForward(Duration),
    SeekBackward(Duration),

    SetVolume(f32),

    OpenUri(String),
    ReportIssue,
    RaiseWindow,
    Quit,
}

pub fn execute(ctx: &Context, gem: &mut GemPlayer, command: GemCommand) {
    match command {
        GemCommand::Play => {
            if let Err(e) = play(&mut gem.player) {
                error!("{}", e);
            } else if let OSMediaControlsState::Initialized(osmc) = &mut gem.os_media_controls
                && let Err(e) = update_playback(&mut osmc.controls, &gem.player)
            {
                error!("{}", e);
            }
        }
        GemCommand::Pause => {
            if let Err(e) = pause(&mut gem.player) {
                error!("{}", e);
            } else if let OSMediaControlsState::Initialized(osmc) = &mut gem.os_media_controls
                && let Err(e) = update_playback(&mut osmc.controls, &gem.player)
            {
                error!("{}", e);
            }
        }
        GemCommand::TogglePlayback => {
            if let Err(e) = toggle(&mut gem.player) {
                error!("{}", e);
            } else if let OSMediaControlsState::Initialized(osmc) = &mut gem.os_media_controls
                && let Err(e) = update_playback(&mut osmc.controls, &gem.player)
            {
                error!("{}", e);
            }
        }
        GemCommand::Stop => {
            stop(&mut gem.player);

            if let OSMediaControlsState::Initialized(osmc) = &mut gem.os_media_controls {
                if let Err(e) = update_metadata(&mut osmc.controls, &gem.player) {
                    error!("{}", e);
                }

                if let Err(e) = update_playback(&mut osmc.controls, &gem.player) {
                    error!("{}", e);
                }
            }
        }
        GemCommand::NextTrack => maybe_play_next(ctx, gem),
        GemCommand::PreviousTrack => maybe_play_previous(ctx, gem),
        GemCommand::SeekTo(position) => {
            if let Err(e) = seek(&mut gem.player, position) {
                error!("{}", e);
            } else if let OSMediaControlsState::Initialized(osmc) = &mut gem.os_media_controls
                && let Err(e) = update_playback(&mut osmc.controls, &gem.player)
            {
                error!("{}", e);
            }
        }
        GemCommand::SeekForward(offset) => {
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
        GemCommand::SeekBackward(offset) => {
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
        GemCommand::SetVolume(volume) => {
            if let Err(e) = set_volume(&mut gem.player, volume) {
                error!("{}", e);
            }
        }
        GemCommand::OpenUri(uri) => {
            warn!("OpenUri is not supported: {uri}");
        }
        GemCommand::ReportIssue => {
            let url = format!("{}/issues", env!("CARGO_PKG_REPOSITORY"));
            ctx.open_url(OpenUrl { url, new_tab: true });
        }
        GemCommand::RaiseWindow => ctx.send_viewport_cmd(ViewportCommand::Focus),
        GemCommand::Quit => ctx.send_viewport_cmd(ViewportCommand::Close),
    }
}
