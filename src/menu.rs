#[cfg(target_os = "macos")]
use std::sync::mpsc::Receiver;

use muda::Menu;
use muda::{MenuEvent, MenuItem, PredefinedMenuItem, Submenu};
use strum_macros::{Display, EnumString};

#[derive(Debug, Clone, Copy, EnumString, Display)]
pub enum MenuCommand {
    OpenFile,
    JumpToPlayingTrack,
    GoToLibrary,
    GoToPlaylists,
    GoToSettings,
    PlayPause,
    NextTrack,
    PreviousTrack,
    VolumeUp,
    VolumeDown,
    Minimize,
    Maximize,
    Fullscreen,
    ReportIssue,
}

#[cfg(target_os = "macos")]
pub fn create_macos_menu() -> (Menu, Receiver<MenuEvent>) {
    use muda::accelerator::{Accelerator, Code, Modifiers};
    use std::sync::mpsc::channel;

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
        &Submenu::with_items("File", true, &[&MenuItem::with_id(MenuCommand::OpenFile, "Open with", true, None)]).unwrap(),
        // The following do not do anything right now but we leave them for convention.
        &Submenu::with_items(
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
        )
        .unwrap(),
        &Submenu::with_items(
            "View",
            true,
            &[
                &MenuItem::with_id(MenuCommand::JumpToPlayingTrack, "Jump to playing track", true, None),
                &PredefinedMenuItem::separator(),
                &MenuItem::with_id(
                    MenuCommand::GoToLibrary,
                    "Go to library",
                    true,
                    Some(Accelerator::new(Some(Modifiers::META), Code::KeyL)),
                ),
                &MenuItem::with_id(
                    MenuCommand::GoToPlaylists,
                    "Go to playlists",
                    true,
                    Some(Accelerator::new(Some(Modifiers::META), Code::KeyP)),
                ),
                &MenuItem::with_id(
                    MenuCommand::GoToSettings,
                    "Go to settings",
                    true,
                    Some(Accelerator::new(Some(Modifiers::META), Code::Comma)),
                ),
            ],
        )
        .unwrap(),
        &Submenu::with_items(
            "Playback",
            true,
            &[
                &MenuItem::with_id(
                    MenuCommand::PlayPause,
                    "Play / Pause",
                    true,
                    Some(Accelerator::new(None, Code::Space)),
                ),
                &MenuItem::with_id(
                    MenuCommand::NextTrack,
                    "Next track",
                    true,
                    Some(Accelerator::new(Some(Modifiers::META), Code::ArrowRight)),
                ),
                &MenuItem::with_id(
                    MenuCommand::PreviousTrack,
                    "Previous track",
                    true,
                    Some(Accelerator::new(Some(Modifiers::META), Code::ArrowLeft)),
                ),
                &MenuItem::with_id(
                    MenuCommand::VolumeUp,
                    "Volume up",
                    true,
                    Some(Accelerator::new(Some(Modifiers::META), Code::ArrowUp)),
                ),
                &MenuItem::with_id(
                    MenuCommand::VolumeDown,
                    "Volume down",
                    true,
                    Some(Accelerator::new(Some(Modifiers::META), Code::ArrowDown)),
                ),
            ],
        )
        .unwrap(),
        &Submenu::with_items(
            "Window",
            true,
            &[
                &PredefinedMenuItem::minimize(None),
                &PredefinedMenuItem::maximize(None),
                &PredefinedMenuItem::separator(),
                &PredefinedMenuItem::fullscreen(None),
            ],
        )
        .unwrap(),
        &Submenu::with_items(
            "Help",
            true,
            &[&MenuItem::with_id(MenuCommand::ReportIssue, "Report an issue", true, None)],
        )
        .unwrap(),
    ])
    .unwrap();

    (menu, receiver)
}
