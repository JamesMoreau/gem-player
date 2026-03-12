use egui::{Context, OpenUrl};
use strum_macros::{Display, EnumString};

use crate::{
    maybe_play_next, maybe_play_previous,
    player::{adjust_volume_by_delta, play_or_pause},
    GemPlayer,
};

#[derive(PartialEq, Debug, Clone, Copy, EnumString, Display)]
pub enum Command {
    // OpenFile,
    PlayPause,
    NextTrack,
    PreviousTrack,
    VolumeUp,
    VolumeDown,
    // Mute / ummute
    ReportIssue,
    Play,
    Pause,
}

pub fn execute(ctx: &Context, gem: &mut GemPlayer, command: Command) {
    match command {
        Command::PlayPause => {
            if let Some(backend) = &mut gem.player.backend {
                play_or_pause(&mut backend.player);
            }
        }
        Command::NextTrack => maybe_play_next(ctx, gem),
        Command::PreviousTrack => maybe_play_previous(ctx, gem),
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
            ctx.open_url(OpenUrl { url, new_tab: true });
        }
        Command::Play => {
            if let Some(backend) = &mut gem.player.backend {
                backend.player.play();
            }
        }
        Command::Pause => {
            if let Some(backend) = &mut gem.player.backend {
                backend.player.pause();
            }
        }
    }
}
