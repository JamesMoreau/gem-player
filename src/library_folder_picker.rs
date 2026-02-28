use std::{
    path::{Path, PathBuf},
    sync::mpsc::{channel, Receiver},
    thread,
};

use rfd::FileDialog;

/// Spawns a folder picker in a background thread and returns the receiver where the selected folder will eventually be sent.
pub fn spawn_library_folder_picker(start_dir: &Path) -> Receiver<Option<PathBuf>> {
    let (sender, receiver) = channel();
    let start_dir = start_dir.to_path_buf();

    thread::spawn(move || {
        let selected_directory = FileDialog::new().set_directory(start_dir).pick_folder().map(|p| p.to_path_buf());
        let _ = sender.send(selected_directory);
    });

    receiver
}
