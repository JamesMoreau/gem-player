use std::str::FromStr;
use std::sync::mpsc::{Receiver, TryRecvError, channel};

use egui::Context;
use log::error;
use muda::accelerator::{Accelerator, Code, Modifiers};
use muda::{Menu, MenuEvent, MenuItem, PredefinedMenuItem, Submenu};

use crate::commands::executor::{Command, execute};
use crate::GemPlayer;

pub fn poll_menu_events(ctx: &Context, gem: &mut GemPlayer) {
    match gem.menu_receiver.try_recv() {
        Ok(event) => handle_menu_event(ctx, gem, event),
        Err(TryRecvError::Empty) => {} // no menu event this frame
        Err(TryRecvError::Disconnected) => {
            error!("Menu events has been disconnected.");
            gem.ui.library_and_playlists_are_loading = false;
        }
    }
}

fn handle_menu_event(ctx: &Context, gem: &mut GemPlayer, event: MenuEvent) {
    let result = Command::from_str(&event.id.0);
    if let Ok(command) = result {
        execute(ctx, gem, command);
    } else {
        error!("Unable to process menu event: {:?}", event);
    }
}

// Create a native macos menu using the Muda crate. Menu items and events are identified using
// the specific command as an Id. We also return a channel receiver to process these events.
pub fn create_menu() -> (Menu, Receiver<MenuEvent>) {
    let (sender, receiver) = channel();
    MenuEvent::set_event_handler(Some(move |event| {
        let _ = sender.send(event);
    }));

    let menu = Menu::with_items(&[
        &Submenu::with_items(
            "App",
            true,
            &[
                &PredefinedMenuItem::about(None, None),
                &PredefinedMenuItem::separator(),
                &PredefinedMenuItem::services(None),
                &PredefinedMenuItem::separator(),
                &PredefinedMenuItem::hide(None),
                &PredefinedMenuItem::hide_others(None),
                &PredefinedMenuItem::show_all(None),
                &PredefinedMenuItem::separator(),
                &PredefinedMenuItem::quit(None),
            ],
        )
        .unwrap(),
        &Submenu::with_items("File", true, &[&MenuItem::with_id(Command::OpenFile, "Open with", true, None)]).unwrap(),
        // The following do not do anything right now but we leave them for convention.
        &Submenu::with_items(
            "Edit",
            true,
            &[
                &PredefinedMenuItem::undo(None),
                &PredefinedMenuItem::redo(None),
                &PredefinedMenuItem::separator(),
                &PredefinedMenuItem::cut(None),
                &PredefinedMenuItem::copy(None),
                &PredefinedMenuItem::paste(None),
                &PredefinedMenuItem::select_all(None),
            ],
        )
        .unwrap(),
        &Submenu::with_items(
            "View",
            true,
            &[
                &MenuItem::with_id(Command::JumpToPlayingTrack, "Jump to playing track", true, None),
                &PredefinedMenuItem::separator(),
                &MenuItem::with_id(
                    Command::GoToLibrary,
                    "Go to library",
                    true,
                    Some(Accelerator::new(Some(Modifiers::META), Code::KeyL)),
                ),
                &MenuItem::with_id(
                    Command::GoToPlaylists,
                    "Go to playlists",
                    true,
                    Some(Accelerator::new(Some(Modifiers::META), Code::KeyP)),
                ),
                &MenuItem::with_id(
                    Command::GoToSettings,
                    "Go to settings",
                    true,
                    Some(Accelerator::new(Some(Modifiers::META), Code::Comma)),
                ),
            ],
        )
        .unwrap(),
        &Submenu::with_items(
            "Playback",
            true,
            &[
                &MenuItem::with_id(Command::PlayPause, "Play / Pause", true, Some(Accelerator::new(None, Code::Space))),
                &MenuItem::with_id(
                    Command::NextTrack,
                    "Next track",
                    true,
                    Some(Accelerator::new(Some(Modifiers::META), Code::ArrowRight)),
                ),
                &MenuItem::with_id(
                    Command::PreviousTrack,
                    "Previous track",
                    true,
                    Some(Accelerator::new(Some(Modifiers::META), Code::ArrowLeft)),
                ),
                &MenuItem::with_id(
                    Command::VolumeUp,
                    "Volume up",
                    true,
                    Some(Accelerator::new(Some(Modifiers::META), Code::ArrowUp)),
                ),
                &MenuItem::with_id(
                    Command::VolumeDown,
                    "Volume down",
                    true,
                    Some(Accelerator::new(Some(Modifiers::META), Code::ArrowDown)),
                ),
            ],
        )
        .unwrap(),
        &Submenu::with_items(
            "Window",
            true,
            &[
                &PredefinedMenuItem::minimize(None),
                &PredefinedMenuItem::maximize(None),
                &PredefinedMenuItem::separator(),
                &PredefinedMenuItem::fullscreen(None),
            ],
        )
        .unwrap(),
        &Submenu::with_items(
            "Help",
            true,
            &[&MenuItem::with_id(Command::ReportIssue, "Report an issue", true, None)],
        )
        .unwrap(),
    ])
    .unwrap();

    (menu, receiver)
}
