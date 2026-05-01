use anyhow::{Result, anyhow};
use eframe::wgpu::rwh::{RawWindowHandle, WindowHandle};
#[cfg(target_os = "macos")]
use muda::{ContextMenu, Menu, MenuItem, dpi::PhysicalPosition};

use crate::playlist::Playlist;

pub fn show_context_menu(window_handle: WindowHandle, menu: &Menu, position: PhysicalPosition<f32>) -> Result<()> {
    match window_handle.as_raw() {
        #[cfg(target_os = "macos")]
        RawWindowHandle::AppKit(h) => unsafe {
            menu.show_context_menu_for_nsview(h.ns_view.as_ptr(), Some(position.into()));
            Ok(())
        },

        #[cfg(target_os = "windows")]
        RawWindowHandle::Win32(h) => unsafe {
            use muda::ContextMenu;

            menu.show_context_menu_for_hwnd(h.hwnd.get(), Some(position.into()));
            Ok(())
        },

        _ => Err(anyhow!("Unsupported platform for context menu")),
    }
}
// enum LibraryContextMenuAction {
//     AddToPlaylist(PathBuf),
//     EnqueueNext,
//     Enqueue,
//     OpenFileLocation,
// }

pub fn build_library_context_menu(selected_tracks_count: usize, playlists: &[Playlist]) -> Menu {
    let menu = Menu::with_items(&[
        &MenuItem::new("Hello, sailor", true, None),
        &MenuItem::new(format!("Number of tracks selected: {}", selected_tracks_count), true, None),
        &MenuItem::new(format!("Number of playlists available: {}", playlists.len()), true, None),
    ])
    .unwrap();

    menu
}
