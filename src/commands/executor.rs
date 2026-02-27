use egui::{Context, OpenUrl, ViewportCommand};
use log::warn;
use strum_macros::{Display, EnumString};

use crate::{
    maybe_play_next, maybe_play_previous,
    player::{adjust_volume_by_percentage, play_or_pause},
    ui::root::View,
    GemPlayer,
};

#[derive(PartialEq, Debug, Clone, Copy, EnumString, Display)]
pub enum Command {
    OpenFile,
    JumpToPlayingTrack,
    GoToLibrary,
    GoToPlaylists,
    GoToSettings,
    PlayPause,
    NextTrack,
    PreviousTrack,
    VolumeUp,
    VolumeDown,
    Minimize,
    Maximize,
    Fullscreen,
    ReportIssue,
    // Mute / ummute
}

pub fn execute(ctx: &Context, gem: &mut GemPlayer, command: Command) {
    match command {
        Command::OpenFile => todo!(),
        Command::GoToLibrary => {
            gem.ui.current_view = View::Library;
        }
        Command::GoToPlaylists => {
            gem.ui.current_view = View::Playlists;
        }
        Command::GoToSettings => {
            gem.ui.current_view = View::Settings;
        }
        Command::PlayPause => {
            if let Some(backend) = &mut gem.player.backend {
                play_or_pause(&mut backend.player);
            }
        }
        Command::NextTrack => maybe_play_next(gem),
        Command::PreviousTrack => maybe_play_previous(gem),
        Command::VolumeUp => {
            if let Some(backend) = &mut gem.player.backend {
                adjust_volume_by_percentage(&mut backend.player, 0.1);
            }
        }
        Command::VolumeDown => {
            if let Some(backend) = &mut gem.player.backend {
                adjust_volume_by_percentage(&mut backend.player, -0.1);
            }
        }
        Command::Minimize => {
            ctx.send_viewport_cmd(ViewportCommand::Minimized(true));
        }
        Command::Maximize => {
            let is_maximized = ctx.input(|i| i.viewport().maximized.unwrap_or(false));
            ctx.send_viewport_cmd(ViewportCommand::Maximized(!is_maximized));
        }
        Command::Fullscreen => {
            let is_fullscreen = ctx.input(|i| i.viewport().fullscreen.unwrap_or(false));
            ctx.send_viewport_cmd(ViewportCommand::Fullscreen(!is_fullscreen))
        }
        Command::ReportIssue => {
            let url = format!("{}/issues", env!("CARGO_PKG_REPOSITORY"));
            ctx.open_url(OpenUrl { url, new_tab: true });
        }
        Command::JumpToPlayingTrack => {
            let Some(playing) = &gem.player.playing else {
                warn!("No currently playing track to jump to.");
                gem.ui.toasts.info("No currently playing track.");
                return;
            };

            gem.ui.current_view = View::Library;
            gem.ui.library.pending_jump_to_track = Some(playing.path.clone());
        }
    }
}
