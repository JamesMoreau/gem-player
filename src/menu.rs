use std::sync::mpsc::{Receiver, channel};

use anyhow::Result;
#[cfg(target_os = "windows")]
use dark_light::Mode;
use fully_pub::fully_pub;
#[cfg(target_os = "windows")]
use muda::MenuTheme;
use muda::{Menu, MenuEvent, MenuItem, PredefinedMenuItem, Submenu};

use crate::commands::GemCommand;

#[cfg(target_os = "windows")]
use {
    crate::resolve_theme_mode,
    eframe::Frame,
    eframe::wgpu::rwh::{HasWindowHandle, RawWindowHandle},
    egui::ThemePreference,
};

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

#[cfg(target_os = "macos")]
pub fn remove_menu(menu: &Menu) {
    menu.remove_for_nsapp();
}

#[cfg(target_os = "windows")]
pub fn remove_menu(menu: &Menu, frame: &Frame) -> Result<()> {
    unsafe {
        let handle = frame.window_handle()?;

        let hwnd = match handle.as_raw() {
            RawWindowHandle::Win32(handle) => handle.hwnd.get(),
            _ => anyhow::bail!("Window is not a Win32 window"),
        };

        menu.remove_for_hwnd(hwnd)?;
    }

    Ok(())
}

#[cfg(target_os = "linux")]
pub fn remove_menu(menu: &Menu) {
    todo!();
}

#[cfg(target_os = "macos")]
pub fn init_menu(menu: &Menu) -> Result<()> {
    menu.init_for_nsapp();
    Ok(())
}

#[cfg(target_os = "windows")]
pub fn init_menu(menu: &Menu, theme_preference: ThemePreference, frame: &Frame) -> Result<()> {
    unsafe {
        let handle = frame.window_handle()?;

        let hwnd = match handle.as_raw() {
            RawWindowHandle::Win32(handle) => handle.hwnd.get(),
            _ => anyhow::bail!("Window is not a Win32 window"),
        };

        let mode = resolve_theme_mode(theme_preference);
        let menu_theme = to_menu_theme(mode);

        menu.init_for_hwnd_with_theme(hwnd, menu_theme)?;
        menu.hide_for_hwnd(hwnd)?;
    }

    Ok(())
}

#[cfg(target_os = "windows")]
pub fn to_menu_theme(mode: Mode) -> MenuTheme {
    match mode {
        Mode::Dark => MenuTheme::Dark,
        Mode::Light => MenuTheme::Light,
        Mode::Unspecified => MenuTheme::Auto,
    }
}
