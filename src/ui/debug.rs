use egui::{Context, Ui, ViewportInfo};

use crate::{APP_NAME, GemPlayer};

pub fn debug_window(ctx: &Context, gem: &mut GemPlayer) {
    if !gem.ui.debug_menu_open {
        return;
    }

    let gem_viewport = ctx.input(|i| i.viewport().clone());

    let viewport_id = egui::ViewportId::from_hash_of("debug_window");

    let viewport_builder = egui::ViewportBuilder::default()
        .with_title(format!("{} Debug", APP_NAME))
        .with_inner_size([320.0, 400.0]);

    let close_requested = ctx.show_viewport_immediate(viewport_id, viewport_builder, |ui, _class| {
        egui::CentralPanel::default().show(ui, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                debug_ui(ui, ctx, &gem_viewport);
            });
        });

        ui.ctx().input(|i| i.viewport().close_requested())
    });

    if close_requested {
        gem.ui.debug_menu_open = false;
    }
}

fn debug_ui(ui: &mut Ui, ctx: &Context, gem_viewport: &ViewportInfo) {
    ui.heading("Window Information");

    if let Some(rect) = gem_viewport.inner_rect {
        ui.label(format!("Window size: {:.0} × {:.0} points", rect.width(), rect.height()));
    }

    ui.add_space(16.0);
    ui.separator();

    ui.heading("egui Debug Options");

    let mut debug_options = ctx.global_style().debug;

    debug_options.ui(ui);

    ctx.global_style_mut(|style| {
        style.debug = debug_options;
    });
}
