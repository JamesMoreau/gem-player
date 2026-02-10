#[cfg(target_os = "macos")]
mod app_delegate;

#[cfg(target_os = "macos")]
pub use app_delegate::install_app_delegate;