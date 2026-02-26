pub mod executor;

#[cfg(target_os = "macos")]
pub mod macos_menu;

#[cfg(target_os = "windows")]
pub mod windows_shortcuts;

#[cfg(target_os = "windows")]
pub mod formatting;