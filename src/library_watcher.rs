use std::{
    path::{Path, PathBuf},
    sync::mpsc::{self, Receiver, Sender},
    thread,
    time::Duration,
};

use log::{error, info};
use notify::RecursiveMode;
use notify_debouncer_mini::{new_debouncer, DebounceEventResult};

use crate::{playlist::{load_playlists_from_directory, Playlist}, track::{load_tracks_from_directory, Track}};

pub enum LibraryWatcherCommand {
    Refresh,
    PathChange(PathBuf),
    Shutdown,
}

pub type LibraryAndPlaylists = (Vec<Track>, Vec<Playlist>);

pub fn setup_library_watcher() -> Result<(Sender<LibraryWatcherCommand>, Receiver<LibraryAndPlaylists>), String> {
    let (command_sender, command_receiver) = mpsc::channel();
    let (update_sender, update_receiver) = mpsc::channel();

    let debouncer_cs = command_sender.clone();
    let thread_cs = command_sender.clone();
    thread::spawn(move || {
        let mut debouncer = new_debouncer(Duration::from_secs(2), {
            move |res: DebounceEventResult| match res {
                Err(e) => error!("watch error: {:?}", e),
                Ok(events) => {
                    for e in events.iter() {
                        info!("Event {:?} for {:?}", e.kind, e.path);
                    }

                    let _ = debouncer_cs.send(LibraryWatcherCommand::Refresh);
                }
            }
        })
        .expect("Failed to create watcher");

        let mut current_path: Option<PathBuf> = None; // TODO: could we just start a new debouncer instead?

        while let Ok(command) = command_receiver.recv() {
            match command {
                LibraryWatcherCommand::Refresh => {
                    if let Some(ref path) = current_path {
                        let (tracks, playlists) = load_library_and_playlists(path);
                        let update_result = update_sender.send((tracks, playlists));
                        if update_result.is_err() {
                            let _ = thread_cs.send(LibraryWatcherCommand::Shutdown);
                        }
                    }
                }
                LibraryWatcherCommand::PathChange(new_path) => {
                    if let Some(ref old) = current_path {
                        let _ = debouncer.watcher().unwatch(old);
                    }

                    let watch_result = debouncer.watcher().watch(&new_path, RecursiveMode::Recursive);
                    if let Err(e) = watch_result {
                        error!("Failed to watch new folder: {:?}", e);
                    } else {
                        current_path = Some(new_path.clone());
                        let _ = thread_cs.send(LibraryWatcherCommand::Refresh);
                    }
                }
                LibraryWatcherCommand::Shutdown => {
                    info!("Received shutdown message. Shutting down the library watcher.");
                    return;
                }
            }
        }
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
