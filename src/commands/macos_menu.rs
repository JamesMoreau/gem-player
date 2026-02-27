use std::str::FromStr;
use std::sync::mpsc::{channel, Receiver, TryRecvError};

use egui::Context;
use log::error;
use muda::accelerator::{Accelerator, Code, Modifiers};
use muda::{Menu, MenuEvent, MenuItem, PredefinedMenuItem, Submenu};

use crate::commands::executor::{execute, Command};
use crate::GemPlayer;

pub struct MenuShortcut {
    pub command: Command,
    pub modifiers: Modifiers,
    pub key: Code,
    pub description: &'static str,
}

pub const PLAY_PAUSE: MenuShortcut = MenuShortcut {
    command: Command::PlayPause,
    modifiers: Modifiers::empty(),
    key: Code::Space,
    description: "Play / Pause",
};

pub const NEXT_TRACK: MenuShortcut = MenuShortcut {
    command: Command::NextTrack,
    modifiers: Modifiers::META,
    key: Code::ArrowRight,
    description: "Next track",
};

pub const PREVIOUS_TRACK: MenuShortcut = MenuShortcut {
    command: Command::PreviousTrack,
    modifiers: Modifiers::META,
    key: Code::ArrowLeft,
    description: "Previous track",
};

pub const VOLUME_UP: MenuShortcut = MenuShortcut {
    command: Command::VolumeUp,
    modifiers: Modifiers::META,
    key: Code::ArrowUp,
    description: "Volume up",
};

pub const VOLUME_DOWN: MenuShortcut = MenuShortcut {
    command: Command::VolumeDown,
    modifiers: Modifiers::META,
    key: Code::ArrowDown,
    description: "Volume down",
};

pub const JUMP_TO_PLAYING_TRACK: MenuShortcut = MenuShortcut {
    command: Command::JumpToPlayingTrack,
    modifiers: Modifiers::META,
    key: Code::KeyT,
    description: "Jump to playing track",
};

pub const GO_TO_LIBRARY: MenuShortcut = MenuShortcut {
    command: Command::GoToLibrary,
    modifiers: Modifiers::META,
    key: Code::KeyL,
    description: "Go to library",
};

pub const GO_TO_PLAYLISTS: MenuShortcut = MenuShortcut {
    command: Command::GoToPlaylists,
    modifiers: Modifiers::META,
    key: Code::KeyP,
    description: "Go to playlists",
};

pub const GO_TO_SETTINGS: MenuShortcut = MenuShortcut {
    command: Command::GoToSettings,
    modifiers: Modifiers::META,
    key: Code::KeyS,
    description: "Go to settings",
};

pub const SHORTCUTS: &[MenuShortcut] = &[
    PLAY_PAUSE,
    NEXT_TRACK,
    PREVIOUS_TRACK,
    VOLUME_UP,
    VOLUME_DOWN,
    JUMP_TO_PLAYING_TRACK,
    GO_TO_LIBRARY,
    GO_TO_PLAYLISTS,
    GO_TO_SETTINGS,
];

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

fn menu_item_from_shortcut(shortcut: &MenuShortcut) -> MenuItem {
    MenuItem::with_id(
        shortcut.command,
        shortcut.description,
        true,
        Some(Accelerator::new(Some(shortcut.modifiers), shortcut.key)),
    )
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
                &menu_item_from_shortcut(&JUMP_TO_PLAYING_TRACK),
                &PredefinedMenuItem::separator(),
                &menu_item_from_shortcut(&GO_TO_LIBRARY),
                &menu_item_from_shortcut(&GO_TO_PLAYLISTS),
                &menu_item_from_shortcut(&GO_TO_SETTINGS),
            ],
        )
        .unwrap(),
        &Submenu::with_items(
            "Playback",
            true,
            &[
                &menu_item_from_shortcut(&PLAY_PAUSE),
                &menu_item_from_shortcut(&NEXT_TRACK),
                &menu_item_from_shortcut(&PREVIOUS_TRACK),
                &menu_item_from_shortcut(&VOLUME_UP),
                &menu_item_from_shortcut(&VOLUME_DOWN),
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

pub fn format_shortcut(mods: Modifiers, key: Code) -> String {
    let mut s = String::new();

    if mods.contains(Modifiers::CONTROL) {
        s.push('⌃');
    }
    if mods.contains(Modifiers::SHIFT) {
        s.push('⇧');
    }
    if mods.contains(Modifiers::ALT) {
        s.push('⌥');
    }
    if mods.contains(Modifiers::META) {
        s.push('⌘');
    }

    let key_str = match key {
        Code::ArrowLeft => "←",
        Code::ArrowRight => "→",
        Code::ArrowUp => "↑",
        Code::ArrowDown => "↓",
        Code::Space => "Space",
        _ => return format!("{} {}", s, key),
    };

    if !s.is_empty() {
        s.push(' ');
    }

    s.push_str(key_str);

    s
}
