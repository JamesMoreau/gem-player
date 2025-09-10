use std::{
    path::{Path, PathBuf},
    sync::mpsc::{self, Receiver, Sender},
    thread,
    time::Duration,
};

use log::{error, info, warn};
use notify::RecursiveMode;
use notify_debouncer_mini::{new_debouncer, DebounceEventResult};

use crate::{
    playlist::{load_playlists_from_directory, Playlist},
    track::{load_tracks_from_directory, Track},
};

pub enum LibraryWatcherCommand {
    Load,
    SetPath(PathBuf),
    Shutdown,
}

pub type LibraryAndPlaylists = (Vec<Track>, Vec<Playlist>);

pub fn setup_library_watcher() -> Result<(Sender<LibraryWatcherCommand>, Receiver<Option<LibraryAndPlaylists>>), String> {
    let (command_sender, command_receiver) = mpsc::channel();
    let (update_sender, update_receiver) = mpsc::channel();

    let debouncer_cs = command_sender.clone();
    let thread_cs = command_sender.clone();
    thread::spawn(move || {
        let mut debouncer = new_debouncer(Duration::from_secs(2), {
            move |res: DebounceEventResult| match res {
                Err(e) => error!("watch error: {:?}", e),
                Ok(events) => {
                    events.iter().for_each(|e| info!("Event for {:?}.", e.path));
                    let _ = debouncer_cs.send(LibraryWatcherCommand::Load);
                }
            }
        })
        .expect("Failed to create watcher");

        let mut watcher_directory: Option<PathBuf> = None;

        while let Ok(command) = command_receiver.recv() {
            match command {
                LibraryWatcherCommand::Load => {
                    if let Some(path) = &watcher_directory {
                        let is_valid = path.exists() && path.is_dir();
                        if !is_valid {
                            error!("Cannot load library: invalid path {:?}", path);
                            let _ = update_sender.send(None);
                            continue;
                        }

                        let library_and_playlists = load_library_and_playlists(path);
                        let _ = update_sender.send(Some(library_and_playlists));
                    } else {
                        warn!("Load command received with no watcher_directory set");
                        let _ = update_sender.send(None);
                    }
                }
                LibraryWatcherCommand::SetPath(new_directory) => {
                    let is_valid = new_directory.exists() && new_directory.is_dir();
                    if !is_valid {
                        warn!("Invalid library path: {:?}", new_directory);
                        let _ = update_sender.send(None);
                        continue;
                    }

                    if let Some(old) = &watcher_directory {
                        let unwatch_result = debouncer.watcher().unwatch(old);
                        if let Err(e) = unwatch_result {
                            error!("Failed to unwatch old folder {:?}: {:?}", old, e);
                            let _ = update_sender.send(None);
                            continue;
                        }
                    }

                    if let Err(e) = debouncer.watcher().watch(&new_directory, RecursiveMode::Recursive) {
                        error!("Failed to watch new folder {:?}: {:?}", new_directory, e);
                        let _ = update_sender.send(None);
                        continue;
                    }

                    watcher_directory = Some(new_directory.clone());
                    let _ = thread_cs.send(LibraryWatcherCommand::Load);
                }
                LibraryWatcherCommand::Shutdown => {
                    info!("Received shutdown message. Shutting down the library watcher.");
                    return;
                }
            }
        }

        info!("Command channel closed. Shutting down watcher.");
    });

    Ok((command_sender, update_receiver))
}

fn load_library_and_playlists(directory: &Path) -> LibraryAndPlaylists {
    let mut library = Vec::new();
    let mut playlists = Vec::new();

    match load_tracks_from_directory(directory) {
        Ok(found_tracks) => {
            library = found_tracks;
        }
        Err(e) => {
            error!("{}", e);
        }
    }

    match load_playlists_from_directory(directory) {
        Ok(found_playlists) => {
            playlists = found_playlists;
        }
        Err(e) => {
            error!("{}", e);
        }
    }

    info!(
        "Loaded library from {:?}: {} tracks, {} playlists.",
        directory,
        library.len(),
        playlists.len()
    );

    (library, playlists)
}
