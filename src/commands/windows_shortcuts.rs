use crate::{
    commands::executor::{execute, Command},
    GemPlayer,
};
use egui::{Context, Key, Modifiers};

pub struct Shortcut {
    pub command: Command,
    pub modifiers: Modifiers,
    pub key: Key,
    pub description: &'static str,
}

pub const SHORTCUTS: &[Shortcut] = &[
    Shortcut {
        command: Command::PlayPause,
        modifiers: Modifiers::NONE,
        key: Key::Space,
        description: "Play / Pause",
    },
    Shortcut {
        command: Command::PreviousTrack,
        modifiers: Modifiers::CTRL,
        key: Key::ArrowLeft,
        description: "Previous track",
    },
    Shortcut {
        command: Command::NextTrack,
        modifiers: Modifiers::CTRL,
        key: Key::ArrowRight,
        description: "Next track",
    },
    Shortcut {
        command: Command::VolumeUp,
        modifiers: Modifiers::CTRL,
        key: Key::ArrowUp,
        description: "Volume up",
    },
    Shortcut {
        command: Command::VolumeDown,
        modifiers: Modifiers::CTRL,
        key: Key::ArrowDown,
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

    let push_part = |part: &str, s: &mut String, first: &mut bool| {
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
