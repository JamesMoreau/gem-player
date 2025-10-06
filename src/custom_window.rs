use eframe::egui::{
    os::OperatingSystem, Align, Align2, Button, CentralPanel, Context, FontId, Frame, Id, Layout, PointerButton, Rect, RichText, Sense, Ui,
    UiBuilder, Vec2, ViewportCommand,
};
use egui_material_icons::icons;

pub fn custom_window(ctx: &Context, title: &str, add_contents: impl FnOnce(&mut Ui)) {
    let frame = Frame::new()
        .fill(ctx.style().visuals.window_fill())
        .corner_radius(10.0)
        .stroke(ctx.style().visuals.widgets.noninteractive.bg_stroke)
        .outer_margin(1); // so the stroke is within the bounds

    CentralPanel::default().frame(frame).show(ctx, |ui| {
        let app_rect = ui.max_rect();

        let title_bar_height = 24.0;
        let title_bar_rect = app_rect.with_max_y(app_rect.min.y + title_bar_height);
        title_bar_ui(ui, title_bar_rect, title);

        let content_rect = app_rect.with_min_y(title_bar_rect.max.y).shrink2(Vec2::new(2.0, 0.0));
        let mut content_ui = ui.new_child(UiBuilder::new().max_rect(content_rect));
        add_contents(&mut content_ui);
    });
}

fn title_bar_ui(ui: &mut Ui, title_bar_rect: Rect, title: &str) {
    let response = ui.interact(title_bar_rect, Id::new("title_bar"), Sense::click_and_drag());

    ui.painter().text(
        title_bar_rect.center(),
        Align2::CENTER_CENTER,
        title,
        FontId::proportional(20.0),
        ui.style().visuals.text_color(),
    );

    if response.double_clicked() {
        let is_maximized = ui.input(|i| i.viewport().maximized.unwrap_or(false));
        ui.ctx().send_viewport_cmd(ViewportCommand::Maximized(!is_maximized));
    }

    if response.drag_started_by(PointerButton::Primary) {
        ui.ctx().send_viewport_cmd(ViewportCommand::StartDrag);
    }

    let os = ui.ctx().os();

    let layout = match os {
        OperatingSystem::Mac => Layout::left_to_right(Align::Center),
        _ => Layout::right_to_left(Align::Center),
    };

    ui.scope_builder(UiBuilder::new().max_rect(title_bar_rect).layout(layout), |ui| {
        ui.add_space(8.0);

        ui.visuals_mut().button_frame = false;
        let button_size = 12.0;

        let close_button = |ui: &mut Ui| {
            let button = Button::new(RichText::new(icons::ICON_CLOSE).size(button_size));
            let response = ui.add(button).on_hover_text("Close the window");
            if response.clicked() {
                ui.ctx().send_viewport_cmd(ViewportCommand::Close);
            }
        };

        let fullscreen_button = |ui: &mut Ui| {
            let is_fullscreen = ui.input(|i| i.viewport().fullscreen.unwrap_or(false));
            let tooltip = if is_fullscreen { "Restore window" } else { "Maximize window" };
            let button = Button::new(RichText::new(icons::ICON_SQUARE).size(button_size));
            let response = ui.add(button).on_hover_text(tooltip);
            if response.clicked() {
                ui.ctx().send_viewport_cmd(ViewportCommand::Fullscreen(!is_fullscreen));
            }
        };

        let minimize_button = |ui: &mut Ui| {
            let button = Button::new(RichText::new(icons::ICON_MINIMIZE).size(button_size));
            let response = ui.add(button).on_hover_text("Minimize the window");
            if response.clicked() {
                ui.ctx().send_viewport_cmd(ViewportCommand::Minimized(true));
            }
        };

        match os {
            OperatingSystem::Mac => {
                close_button(ui);
                minimize_button(ui);
                fullscreen_button(ui);
            }
            _ => {
                close_button(ui);
                fullscreen_button(ui);
                minimize_button(ui);
            }
        }
    });
}

// Resize handles
//     let thickness = 8.0;

//     create_resize_handle(
//         ctx,
//         "resize_right",
//         "resize_right_handle",
//         Pos2::new(app_rect.max.x - thickness, app_rect.min.y + thickness),
//         Vec2::new(thickness, app_rect.height() - 2.0 * thickness),
//         CursorIcon::ZoomIn,
//         ResizeDirection::East,
//     );

//     // Bottom edge
//     create_resize_handle(
//         ctx,
//         "resize_bottom",
//         "resize_bottom_handle",
//         Pos2::new(app_rect.min.x + thickness, app_rect.max.y - thickness),
//         Vec2::new(app_rect.width() - 2.0 * thickness, thickness),
//         CursorIcon::ZoomIn,
//         ResizeDirection::South,
//     );

//     // Bottom-right corner
//     create_resize_handle(
//         ctx,
//         "resize_br_area",
//         "resize_br",
//         app_rect.max - Vec2::splat(thickness),
//         Vec2::splat(thickness),
//         CursorIcon::ZoomIn,
//         ResizeDirection::SouthEast,
//     );
// });

// 	fn create_resize_handle(
//     ctx: &Context,
//     area_id: &str,
//     handle_id: &str,
//     position: Pos2,
//     size: Vec2,
//     cursor_icon: CursorIcon,
//     resize_direction: ResizeDirection,
// ) {
//     Area::new(Id::new(area_id)).fixed_pos(position).show(ctx, |ui| {
//         ui.set_min_size(size);
//         let (_id, response) = ui.allocate_space(size);
//         let interaction_response = ui.interact(response, Id::new(handle_id), Sense::click_and_drag());

//         if interaction_response.hovered() {
//             ctx.set_cursor_icon(cursor_icon);
//         }

//         if interaction_response.drag_started_by(PointerButton::Primary) {
//             ctx.send_viewport_cmd(ViewportCommand::BeginResize(resize_direction));
//         }
//     });
// }
