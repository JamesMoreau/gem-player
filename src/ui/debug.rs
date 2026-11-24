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
        .with_inner_size([350.0, 400.0])
        .with_min_inner_size([470.0, 300.0]);

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
    egui::CollapsingHeader::new("Window Information")
        .default_open(false)
        .show(ui, |ui| {
            gem_viewport.ui(ui);
        });

    ui.add_space(8.0);

    egui::CollapsingHeader::new("egui Debug Options").default_open(true).show(ui, |ui| {
        let mut debug_options = ctx.global_style().debug;

        debug_options.ui(ui);

        ctx.global_style_mut(|style| {
            style.debug = debug_options;
        });
    });
}
