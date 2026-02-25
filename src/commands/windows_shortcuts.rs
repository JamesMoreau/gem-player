use crate::{GemPlayer, commands::executor::{Command, execute}};
use egui::{Context, Key, Modifiers};

pub fn handle_shortcuts(ctx: &Context, gem: &mut GemPlayer) {
    if ctx.wants_keyboard_input() {
        return;
    }

    ctx.input_mut(|i| {
        if i.consume_key(Modifiers::NONE, Key::Space) {
            execute(ctx, gem, Command::PlayPause);
        }

        if i.consume_key(Modifiers::NONE, Key::ArrowLeft) {
            execute(ctx, gem, Command::PreviousTrack);
        }

        if i.consume_key(Modifiers::NONE, Key::ArrowRight) {
            execute(ctx, gem, Command::NextTrack);
        }

        if i.consume_key(Modifiers::NONE, Key::ArrowUp) {
            execute(ctx, gem, Command::VolumeUp);
        }

        if i.consume_key(Modifiers::NONE, Key::ArrowDown) {
            execute(ctx, gem, Command::VolumeDown);
        }
    });
}
