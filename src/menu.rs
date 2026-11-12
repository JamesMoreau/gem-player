use std::sync::mpsc::{Receiver, channel};

use anyhow::Result;
use eframe::Frame;
use egui::ThemePreference;
use fully_pub::fully_pub;
use muda::{Menu, MenuEvent, MenuItem, PredefinedMenuItem, Submenu};

use crate::commands::GemCommand;

#[fully_pub]
struct MenuBar {
    menu: Menu,
    menu_receiver: Receiver<MenuEvent>,
}

// Create a native macOS menu using the Muda crate. Custom menu items use
// GemCommand as their ID, allowing menu events to be forwarded directly
// into the application's command system.
pub fn create_menu() -> (Menu, Receiver<MenuEvent>) {
    let (sender, receiver) = channel();

    MenuEvent::set_event_handler(Some(move |event| {
        let _ = sender.send(event);
    }));

    let menu = Menu::with_items(&[
        #[cfg(target_os = "macos")]
        &Submenu::with_items(
            t!("app"),
            true,
            &[
                &PredefinedMenuItem::about(Some(&t!("about")), None),
                &PredefinedMenuItem::separator(),
                &PredefinedMenuItem::services(Some(&t!("services"))),
                &PredefinedMenuItem::separator(),
                &PredefinedMenuItem::hide(Some(&t!("hide", app = t!("app")))),
                &PredefinedMenuItem::hide_others(Some(&t!("hide_others"))),
                &PredefinedMenuItem::show_all(Some(&t!("show_all"))),
                &PredefinedMenuItem::separator(),
                &PredefinedMenuItem::quit(Some(&t!("quit"))),
            ],
        )
        .unwrap(),
        &Submenu::with_items(t!("file"), true, &[&PredefinedMenuItem::close_window(Some(&t!("close_window")))]).unwrap(),
        &Submenu::with_items(
            t!("window"),
            true,
            &[
                &PredefinedMenuItem::minimize(Some(&t!("minimize"))),
                &PredefinedMenuItem::separator(),
                &PredefinedMenuItem::fullscreen(Some(&t!("fullscreen"))),
            ],
        )
        .unwrap(),
        &Submenu::with_items(
            t!("help"),
            true,
            &[&MenuItem::with_id(GemCommand::ReportIssue, t!("report_issue"), true, None)],
        )
        .unwrap(),
    ])
    .unwrap();

    (menu, receiver)
}

pub fn remove_menu(menu: &Menu) -> Result<()> {
    #[cfg(target_os = "macos")]
    menu.remove_for_nsapp();

    #[cfg(target_os = "windows")]
    menu.remove_for_hwnd(/* hwnd */)?;

    #[cfg(target_os = "linux")]
    menu.remove_for_gtk_window(/* ... */)?;

    Ok(())
}

pub fn init_menu(menu: &Menu, _theme_preference: ThemePreference, _frame: &Frame) -> anyhow::Result<()> {
    #[cfg(target_os = "windows")]
    unsafe {
        let handle = _frame.window_handle()?;

        let hwnd = match handle.as_raw() {
            RawWindowHandle::Win32(handle) => handle.hwnd.get(),
            _ => anyhow::bail!("Window is not a Win32 window"),
        };

        let _mode = resolve_theme_mode(theme_preference);
        let menu_theme = to_menu_theme(mode);

        menu.init_for_hwnd_with_theme(hwnd, menu_theme)?;
        menu.hide_for_hwnd(hwnd)?;
    }

    #[cfg(target_os = "linux")]
    menu.init_for_gtk_window(&gtk_window, Some(&vertical_gtk_box));

    #[cfg(target_os = "macos")]
    menu.init_for_nsapp();

    Ok(())
}
