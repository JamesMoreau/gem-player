use std::{
    path::{Path, PathBuf},
    sync::mpsc::{channel, Receiver, TryRecvError},
    thread,
};

use log::{error, info};
use rfd::FileDialog;

use crate::{library_watcher::LibraryWatcherCommand, GemPlayer};

pub fn poll_library_folder_picker(gem: &mut GemPlayer) {
    let Some(receiver) = &gem.folder_picker_receiver else {
        return;
    };

    match receiver.try_recv() {
        Ok(maybe_directory) => {
            gem.folder_picker_receiver = None;

            if let Some(directory) = maybe_directory {
                info!("Selected folder: {:?}", directory);

                let command = LibraryWatcherCommand::SetPath(directory.clone());
                let result = gem.library_watcher.command_sender.send(command);
                if result.is_err() {
                    let message = "Failed to start watching library directory. Reverting back to old directory.";
                    error!("{}", message);
                    gem.ui.toasts.error(message);
                } else {
                    gem.library_directory = Some(directory);
                    gem.ui.library_and_playlists_are_loading = true;
                }
            } else {
                info!("No folder selected");
            }
        }
        Err(TryRecvError::Empty) => {} // folder picker is still open.
        Err(TryRecvError::Disconnected) => {
            error!("Folder picker channel disconnected unexpectedly.");
            gem.folder_picker_receiver = None;
        }
    }
}

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
