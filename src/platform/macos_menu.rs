use std::sync::mpsc::{channel, Receiver};

use fully_pub::fully_pub;
use muda::accelerator::{Accelerator, Code, Modifiers};
use muda::{Menu, MenuEvent, MenuItem, PredefinedMenuItem, Submenu};

use crate::commands::Command;

#[fully_pub]
struct MenuBar {
    menu: Menu,
    menu_receiver: Receiver<MenuEvent>,
}

#[fully_pub]
struct MenuShortcut {
    command: Command,
    modifiers: Modifiers,
    key: Code,
    description: &'static str,
}

pub const PLAY_PAUSE: MenuShortcut = MenuShortcut {
    command: Command::PlayPause,
    modifiers: Modifiers::META,
    key: Code::KeyP,
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

pub const SHORTCUTS: &[MenuShortcut] = &[
    PLAY_PAUSE,
    NEXT_TRACK,
    PREVIOUS_TRACK,
    VOLUME_UP,
    VOLUME_DOWN,
    JUMP_TO_PLAYING_TRACK,
];

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
        // &Submenu::with_items("File", true, &[&MenuItem::with_id(Command::OpenFile, "Open with", true, None)]).unwrap(),
        &Submenu::with_items("View", true, &[&menu_item_from_shortcut(&JUMP_TO_PLAYING_TRACK)]).unwrap(),
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
