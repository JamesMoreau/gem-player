use std::{
    path::PathBuf,
    sync::mpsc::{channel, Receiver, Sender},
    thread,
    time::Duration,
};

use anyhow::{Context, Result};
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

pub fn setup_library_watcher() -> Result<(Sender<LibraryWatcherCommand>, Receiver<Option<LibraryAndPlaylists>>)> {
    let (command_sender, command_receiver) = channel();
    let (update_sender, update_receiver) = channel();

    let debouncer_command_sender = command_sender.clone();

    // The debouncer, using a channel, will message the watcher thread, notifying it when the library changes.
    let mut debouncer = new_debouncer(Duration::from_secs(2), move |res: DebounceEventResult| match res {
        Err(e) => error!("watch error: {:?}", e),
        Ok(events) => {
            for e in events {
                info!("Event for {:?}", e.path);
            }
            let _ = debouncer_command_sender.send(LibraryWatcherCommand::Load);
        }
    })
    .context("failed to create filesystem debouncer")?;

    let watcher_command_sender = command_sender.clone();

    thread::spawn(move || {
        let mut watcher_directory: Option<PathBuf> = None;

        while let Ok(command) = command_receiver.recv() {
            match command {
                LibraryWatcherCommand::Load => {
                    if let Some(path) = &watcher_directory {
                        if !path.is_dir() {
                            error!("Cannot load library: invalid path {:?}", path);
                            let _ = update_sender.send(None);
                            continue;
                        }

                        let library = load_tracks_from_directory(path);
                        let playlists = load_playlists_from_directory(path);

                        info!(
                            "Loaded library from {:?}: {} tracks, {} playlists.",
                            path,
                            library.len(),
                            playlists.len()
                        );

                        let _ = update_sender.send(Some((library, playlists)));
                    } else {
                        warn!("Load command received with no watcher_directory set");
                        let _ = update_sender.send(None);
                    }
                }
                LibraryWatcherCommand::SetPath(new_directory) => {
                    if !new_directory.is_dir() {
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
                    let _ = watcher_command_sender.send(LibraryWatcherCommand::Load);
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
