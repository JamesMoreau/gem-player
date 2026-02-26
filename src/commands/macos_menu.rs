use std::str::FromStr;
use std::sync::mpsc::{channel, Receiver, TryRecvError};

use egui::{Context};
use log::error;
use muda::accelerator::{Accelerator, Code, Modifiers};
use muda::{Menu, MenuEvent, MenuItem, PredefinedMenuItem, Submenu};

use crate::commands::executor::{execute, Command};
use crate::GemPlayer;

pub struct Shortcut {
    pub command: Command,
    pub key: Code,
    pub modifiers: Modifiers,
    pub description: &'static str,
}

pub const SHORTCUTS: &[Shortcut] = &[
    Shortcut {
        command: Command::PlayPause,
        description: "Play / Pause",
        key: Code::Space,
        modifiers: Modifiers::empty(),
    },
    Shortcut {
        command: Command::NextTrack,
        description: "Next track",
        key: Code::ArrowRight,
        modifiers: Modifiers::META,
    },
    Shortcut {
        command: Command::PreviousTrack,
        description: "Previous track",
        key: Code::ArrowLeft,
        modifiers: Modifiers::META,
    },
    Shortcut {
        command: Command::VolumeUp,
        description: "Volume up",
        key: Code::ArrowUp,
        modifiers: Modifiers::META,
    },
    Shortcut {
        command: Command::VolumeDown,
        description: "Volume down",
        key: Code::ArrowDown,
        modifiers: Modifiers::META,
    },
    Shortcut {
        command: Command::GoToLibrary,
        description: "Go to library",
        key: Code::KeyL,
        modifiers: Modifiers::META,
    },
    Shortcut {
        command: Command::GoToPlaylists,
        description: "Go to playlists",
        key: Code::KeyP,
        modifiers: Modifiers::META,
    },
    Shortcut {
        command: Command::GoToSettings,
        description: "Go to settings",
        key: Code::KeyS,
        modifiers: Modifiers::META,
    },
];

pub fn get_shortcut_by_command(command: Command) -> &'static Shortcut {
    SHORTCUTS
        .iter()
        .find(|s| s.command == command)
        .unwrap_or_else(|| panic!("Shortcut must exist for this command {}", command))
}

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

fn menu_item_from_shortcut(shortcut: &Shortcut) -> MenuItem {
    MenuItem::with_id(
        shortcut.command,
        shortcut.description,
        true,
        Some(Accelerator::new(Some(shortcut.modifiers), shortcut.key)),
    )
}

fn item(command: Command) -> MenuItem {
    menu_item_from_shortcut(get_shortcut_by_command(command))
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
                &item(Command::GoToLibrary),
                &item(Command::GoToPlaylists),
                &item(Command::GoToSettings),
            ],
        )
        .unwrap(),
        &Submenu::with_items(
            "Playback",
            true,
            &[
                &item(Command::PlayPause),
                &item(Command::NextTrack),
                &item(Command::PreviousTrack),
                &item(Command::VolumeUp),
                &item(Command::VolumeDown),
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
