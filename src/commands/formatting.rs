use egui::{Key, Modifiers};

pub fn format_shortcut(mods: Modifiers, key: Key) -> String {
    let mut parts: Vec<&str> = Vec::new();

    if mods.ctrl {
        parts.push("Ctrl");
    }
    if mods.shift {
        parts.push("Shift");
    }
    if mods.alt {
        parts.push("Alt");
    }
    if mods.command {
        parts.push("Cmd");
    }

    let key_str = format_key(key);

    if parts.is_empty() {
        key_str
    } else {
        format!("{} + {}", parts.join(" + "), key_str)
    }
}

fn format_key(key: Key) -> String {
    match key {
        Key::ArrowLeft => "←".into(),
        Key::ArrowRight => "→".into(),
        Key::ArrowUp => "↑".into(),
        Key::ArrowDown => "↓".into(),
        Key::Space => "Space".into(),
        _ => format!("{:?}", key),
    }
}