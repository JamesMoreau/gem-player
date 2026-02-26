use crate::{
    commands::executor::{execute, Command},
    GemPlayer,
};
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

pub fn format_shortcut(mods: Modifiers, key: Key) -> String {
    let mut s = String::new();
    let mut first = true;

    let mut push_part = |part: &str, s: &mut String, first: &mut bool| {
        if !*first {
            s.push_str(" + ");
        }
        s.push_str(part);
        *first = false;
    };

    if mods.ctrl {
        push_part("Ctrl", &mut s, &mut first);
    }
    if mods.shift {
        push_part("Shift", &mut s, &mut first);
    }
    if mods.alt {
        push_part("Alt", &mut s, &mut first);
    }
    if mods.command {
        push_part("Cmd", &mut s, &mut first);
    }

    let key_str = match key {
        Key::ArrowLeft => "←".to_string(),
        Key::ArrowRight => "→".to_string(),
        Key::ArrowUp => "↑".to_string(),
        Key::ArrowDown => "↓".to_string(),
        Key::Space => "Space".to_string(),
        _ => format!("{:?}", key),
    };

    push_part(&key_str, &mut s, &mut first);

    s
}
