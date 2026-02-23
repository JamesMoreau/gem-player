#[cfg(target_os = "macos")]
use std::sync::mpsc::Receiver;

use muda::Menu;
use muda::{MenuEvent, MenuItem, PredefinedMenuItem, Submenu};

pub fn handle_menu_event(event: MenuEvent) {
    // Get the menu item ID
    let id = event.id;

    // You'll need to store menu item IDs to identify which was clicked
    // For now, you can print to see what's being clicked
    println!("Menu event received: {id:?}");

    // Handle specific menu items
    // Example:
    // if id == self.save_item_id {
    //     // Handle save
    // }
}

#[cfg(target_os = "macos")]
pub fn create_macos_menu() -> (Menu, Receiver<MenuEvent>) {
    use muda::accelerator::{Accelerator, Code, Modifiers};
    use std::sync::mpsc::channel;

    let menu = Menu::new();
    
    let (sender, receiver) = channel();
    MenuEvent::set_event_handler(Some(move |event| {
        let _ = sender.send(event);
    }));
    
    let app_menu = Submenu::with_items(
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
    ).unwrap();

    let file_menu = Submenu::with_items(
        "File",
        true,
        &[
            &MenuItem::new("Open with", true, None),
            &PredefinedMenuItem::separator(),
            &PredefinedMenuItem::close_window(None),
        ],
    ).unwrap();

    let edit_menu = Submenu::with_items(
        "Edit",
        true,
        &[
            &PredefinedMenuItem::undo(None),
            &PredefinedMenuItem::redo(None),
            &PredefinedMenuItem::separator(),
            &PredefinedMenuItem::cut(None),
            &PredefinedMenuItem::copy(None),
            &PredefinedMenuItem::paste(None),
            &PredefinedMenuItem::select_all(None),
        ],
    ).unwrap();

    let view_menu = Submenu::with_items(
        "View",
        true,
        &[
            &MenuItem::new("Jump to playing track", true, None),
            &MenuItem::new("Go to library", true, Some(Accelerator::new(Some(Modifiers::META), Code::KeyL))),
            &MenuItem::new("Go to playlists", true, Some(Accelerator::new(Some(Modifiers::META), Code::KeyP))),
            &MenuItem::new("Go to settings", true, Some(Accelerator::new(Some(Modifiers::META), Code::Comma))),
        ],
    ).unwrap();

    let playback_menu = Submenu::with_items(
        "Playback",
        true,
        &[
            &MenuItem::new("Play / Pause", true, Some(Accelerator::new(None, Code::Space))),
            &MenuItem::new("Next track", true, Some(Accelerator::new(Some(Modifiers::META), Code::ArrowRight))),
            &MenuItem::new(
                "Previous track",
                true,
                Some(Accelerator::new(Some(Modifiers::META), Code::ArrowLeft)),
            ),
        ],
    ).unwrap();

    let window_menu = Submenu::with_items(
        "Window",
        true,
        &[
            &PredefinedMenuItem::minimize(None),
            &PredefinedMenuItem::maximize(None),
            &PredefinedMenuItem::separator(),
            &PredefinedMenuItem::fullscreen(None),
        ],
    ).unwrap();

    let help_menu = Submenu::new("Help", true);
    let report_an_issue = MenuItem::new("Report an issue", true, None);
    help_menu.append_items(&[&report_an_issue]).unwrap();

    menu.append_items(&[&app_menu, &file_menu, &edit_menu, &view_menu, &playback_menu, &window_menu, &help_menu])
        .unwrap();

    (menu, receiver)
}
