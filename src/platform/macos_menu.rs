use std::sync::mpsc::{Receiver, channel};

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
        &Submenu::with_items(
            "App",
            true,
            &[
                &PredefinedMenuItem::about(None, None),
                &PredefinedMenuItem::separator(),
                &PredefinedMenuItem::services(None),
                &PredefinedMenuItem::separator(),
                &PredefinedMenuItem::hide(None),
                &PredefinedMenuItem::hide_others(None),
                &PredefinedMenuItem::show_all(None),
                &PredefinedMenuItem::separator(),
                &PredefinedMenuItem::quit(None),
            ],
        )
        .unwrap(),
        &Submenu::with_items("File", true, &[&PredefinedMenuItem::close_window(None)]).unwrap(),
        &Submenu::with_items(
            "Window",
            true,
            &[
                &PredefinedMenuItem::minimize(None),
                &PredefinedMenuItem::separator(),
                &PredefinedMenuItem::fullscreen(None),
            ],
        )
        .unwrap(),
        &Submenu::with_items(
            "Help",
            true,
            &[&MenuItem::with_id(GemCommand::ReportIssue, "Report an issue", true, None)],
        )
        .unwrap(),
    ])
    .unwrap();

    (menu, receiver)
}
