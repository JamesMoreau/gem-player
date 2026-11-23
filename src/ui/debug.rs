use egui::{Context, Ui};

use crate::{APP_NAME, GemPlayer};

pub fn debug_window(ctx: &Context, gem: &mut GemPlayer) {
    if !gem.ui.debug_menu_open {
        return;
    }

    let viewport_id = egui::ViewportId::from_hash_of("debug_window");

    let viewport_builder = egui::ViewportBuilder::default()
        .with_title(format!("{} Debug", APP_NAME))
        .with_inner_size([320.0, 400.0]);

    let close_requested = ctx.show_viewport_immediate(viewport_id, viewport_builder, |ui, _class| {
        egui::CentralPanel::default().show(ui, |ui| {
            debug_ui(ui, ctx);
        });

        ui.ctx().input(|i| i.viewport().close_requested())
    });

    if close_requested {
        gem.ui.debug_menu_open = false;
    }
}

fn debug_ui(ui: &mut Ui, ctx: &Context) {
    ui.heading("egui Debug Options");
    ui.add_space(8.0);

    let original = ctx.global_style().debug;
    let mut debug_options = original;

    if ui.button("Reset").clicked() {
        debug_options = Default::default();
    }

    ui.separator();

    debug_options.ui(ui);

    if debug_options != original {
        let mut style = (*ctx.global_style()).clone();
        style.debug = debug_options;
        ctx.set_global_style(style);
    }
}
