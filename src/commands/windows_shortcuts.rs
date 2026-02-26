use crate::{GemPlayer, commands::executor::{Command, execute}};
use egui::{Context, Key, Modifiers};

pub struct Shortcut {
    pub command: Command,
    pub key: Key,
    pub modifiers: Modifiers,
    pub description: &'static str,
}

pub const SHORTCUTS: &[Shortcut] = &[
    Shortcut {
        command: Command::PlayPause,
        key: Key::Space,
        modifiers: Modifiers::NONE,
        description: "Play / Pause",
    },
    Shortcut {
        command: Command::PreviousTrack,
        key: Key::ArrowLeft,
        modifiers: Modifiers::CTRL,
        description: "Previous track",
    },
    Shortcut {
        command: Command::NextTrack,
        key: Key::ArrowRight,
        modifiers: Modifiers::CTRL,
        description: "Next track",
    },
    Shortcut {
        command: Command::VolumeUp,
        key: Key::ArrowUp,
        modifiers: Modifiers::CTRL,
        description: "Volume up",
    },
    Shortcut {
        command: Command::VolumeDown,
        key: Key::ArrowDown,
        modifiers: Modifiers::CTRL,
        description: "Volume down",
    },
];

pub fn handle_shortcuts(ctx: &Context, gem: &mut GemPlayer) {
    if ctx.wants_keyboard_input() {
        return;
    }

    ctx.input_mut(|i| {
        for shortcut in SHORTCUTS {
            if i.consume_key(shortcut.modifiers, shortcut.key) {
                execute(ctx, gem, shortcut.command);
            }
        }
    });
}
