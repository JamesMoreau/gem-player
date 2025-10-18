use eframe::egui::{
    Align, Align2, Button, CentralPanel, Context, FontId, Frame, Id, Layout, PointerButton, Rect, RichText, Sense, Ui, UiBuilder, Vec2,
    ViewportCommand,
};
#[cfg(target_os = "windows")]
use eframe::egui::{Area, CursorIcon, Pos2, ResizeDirection};

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
        title_bar_ui(ui, title, title_bar_rect);

        let content_rect = app_rect.with_min_y(title_bar_rect.max.y).shrink2(Vec2::new(2.0, 0.0));
        let mut content_ui = ui.new_child(UiBuilder::new().max_rect(content_rect));
        add_contents(&mut content_ui);
    });

    #[cfg(target_os = "windows")]
    {
        let window_rect = ctx.viewport_rect();
        add_resize_handles(ctx, window_rect);
    }
}

fn title_bar_ui(ui: &mut Ui, title: &str, title_bar_rect: Rect) {
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

    let close_button = |ui: &mut Ui| {
        let button = Button::new(RichText::new(icons::ICON_CLOSE).size(12.0));
        let response = ui.add(button).on_hover_text("Close the window");
        if response.clicked() {
            ui.ctx().send_viewport_cmd(ViewportCommand::Close);
        }
    };

    let fullscreen_button = |ui: &mut Ui| {
        let is_fullscreen = ui.input(|i| i.viewport().fullscreen.unwrap_or(false));
        let tooltip = if is_fullscreen { "Restore window" } else { "Maximize window" };
        let button = Button::new(RichText::new(icons::ICON_SQUARE).size(12.0));
        let response = ui.add(button).on_hover_text(tooltip);
        if response.clicked() {
            ui.ctx().send_viewport_cmd(ViewportCommand::Fullscreen(!is_fullscreen));
        }
    };

    let minimize_button = |ui: &mut Ui| {
        let button = Button::new(RichText::new(icons::ICON_MINIMIZE).size(12.0));
        let response = ui.add(button).on_hover_text("Minimize the window");
        if response.clicked() {
            ui.ctx().send_viewport_cmd(ViewportCommand::Minimized(true));
        }
    };

    #[cfg(target_os = "macos")]
    let (layout, button_order): (Layout, &[fn(&mut Ui)]) = (
        Layout::left_to_right(Align::Center),
        &[close_button, minimize_button, fullscreen_button],
    );

    #[cfg(target_os = "windows")]
    let (layout, button_order): (Layout, &[fn(&mut Ui)]) = (
        Layout::right_to_left(Align::Center),
        &[close_button, fullscreen_button, minimize_button],
    );

    ui.scope_builder(UiBuilder::new().max_rect(title_bar_rect).layout(layout), |ui| {
        ui.add_space(8.0);
        ui.visuals_mut().button_frame = false;

        for button in button_order {
            button(ui);
        }
    });
}

#[cfg(target_os = "windows")]
fn add_resize_handles(ctx: &Context, window_rect: Rect) {
    let left = window_rect.left();
    let right = window_rect.right();
    let top = window_rect.top();
    let bottom = window_rect.bottom();
    let width = window_rect.width();
    let height = window_rect.height();

    // Corners
    let corner_size = 6.0;

    create_resize_handle(
        ctx,
        "top_left_corner_area",
        "top_left_corner_handle",
        Pos2::new(left, top),
        Vec2::splat(corner_size),
        CursorIcon::ResizeNorthWest,
        ResizeDirection::NorthWest,
    );

    create_resize_handle(
        ctx,
        "top_right_corner_area",
        "top_right_corner_handle",
        Pos2::new(right - corner_size, top),
        Vec2::splat(corner_size),
        CursorIcon::ResizeNorthEast,
        ResizeDirection::NorthEast,
    );

    create_resize_handle(
        ctx,
        "bottom_left_corner_area",
        "bottom_left_corner_handle",
        Pos2::new(left, bottom - corner_size),
        Vec2::splat(corner_size),
        CursorIcon::ResizeSouthWest,
        ResizeDirection::SouthWest,
    );

    create_resize_handle(
        ctx,
        "bottom_right_corner_area",
        "bottom_right_corner_handle",
        Pos2::new(right - corner_size, bottom - corner_size),
        Vec2::splat(corner_size),
        CursorIcon::ResizeSouthEast,
        ResizeDirection::SouthEast,
    );

    // Edges
    let edge_thickness = 4.0;

    create_resize_handle(
        ctx,
        "top_edge_area",
        "top_edge_handle",
        Pos2::new(left + corner_size, top),
        Vec2::new(width - 2.0 * corner_size, edge_thickness),
        CursorIcon::ResizeVertical,
        ResizeDirection::North,
    );

    create_resize_handle(
        ctx,
        "bottom_edge_area",
        "bottom_edge_handle",
        Pos2::new(left + corner_size, bottom - edge_thickness),
        Vec2::new(width - 2.0 * corner_size, edge_thickness),
        CursorIcon::ResizeVertical,
        ResizeDirection::South,
    );

    create_resize_handle(
        ctx,
        "left_edge_area",
        "left_edge_handle",
        Pos2::new(left, top + corner_size),
        Vec2::new(edge_thickness, height - 2.0 * corner_size),
        CursorIcon::ResizeHorizontal,
        ResizeDirection::West,
    );

    create_resize_handle(
        ctx,
        "right_edge_area",
        "right_edge_handle",
        Pos2::new(right - edge_thickness, top + corner_size),
        Vec2::new(edge_thickness, height - 2.0 * corner_size),
        CursorIcon::ResizeHorizontal,
        ResizeDirection::East,
    );
}

#[cfg(target_os = "windows")]
fn create_resize_handle(
    ctx: &Context,
    area_id: &str,
    handle_id: &str,
    position: Pos2,
    size: Vec2,
    cursor_icon: CursorIcon,
    resize_direction: ResizeDirection,
) {
    Area::new(Id::new(area_id)).fixed_pos(position).show(ctx, |ui| {
        ui.set_min_size(size);
        let (_id, response) = ui.allocate_space(size);
        let interaction_response = ui.interact(response, Id::new(handle_id), Sense::click_and_drag());

        if interaction_response.hovered() {
            ctx.set_cursor_icon(cursor_icon);
        }

        if interaction_response.drag_started_by(PointerButton::Primary) {
            ctx.send_viewport_cmd(ViewportCommand::BeginResize(resize_direction));
        }
    });
}
