use egui_toast::{Toast, ToastKind, ToastOptions, Toasts};

fn add_toast(toasts: &mut Toasts, kind: ToastKind, message: impl Into<egui::WidgetText>) {
    toasts.add(
        Toast::new()
            .kind(kind)
            .text(message)
            .options(
                ToastOptions::default()
                    .duration_in_seconds(5.0)
                    .show_progress(true)
            ),
    );
}

pub fn success_toast(toasts: &mut Toasts, message: impl Into<egui::WidgetText>) {
    add_toast(toasts, ToastKind::Success, message);
}

pub fn warning_toast(toasts: &mut Toasts, message: impl Into<egui::WidgetText>) {
    add_toast(toasts, ToastKind::Warning, message);
}

pub fn error_toast(toasts: &mut Toasts, message: impl Into<egui::WidgetText>) {
    add_toast(toasts, ToastKind::Error, message);
}