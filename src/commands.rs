use egui::{OpenUrl, Ui};
use log::error;
use strum_macros::{Display, EnumString};

use crate::{
    GemPlayer, maybe_play_next, maybe_play_previous,
    player::{adjust_volume_by_delta, play_or_pause},
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

pub fn execute(ui: &mut Ui, gem: &mut GemPlayer, command: Command) {
    match command {
        Command::PlayPause => {
            if let Err(e) = play_or_pause(&mut gem.player) {
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
