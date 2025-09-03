use std::{
    path::PathBuf,
    sync::mpsc::{self, Receiver, Sender},
    thread,
    time::Duration,
};

use log::{error, info};
use notify::RecursiveMode;
use notify_debouncer_mini::{new_debouncer, DebounceEventResult};

use crate::{load_library_and_playlists, playlist::Playlist, track::Track};

pub enum LibraryWatcherCommand {
    Refresh,
    PathChange(PathBuf),
    Shutdown,
}

pub fn setup_library_watcher() -> Result<(Sender<LibraryWatcherCommand>, Receiver<(Vec<Track>, Vec<Playlist>)>), String> {
    let (command_sender, command_receiver) = mpsc::channel();
    let (update_sender, update_receiver) = mpsc::channel();

    let cs = command_sender.clone();
    thread::spawn(move || {
        let mut debouncer = new_debouncer(Duration::from_secs(2), {
            move |res: DebounceEventResult| match res {
                Err(e) => error!("watch error: {:?}", e),
                Ok(events) => {
                    for e in events.iter() {
                        info!("Event {:?} for {:?}", e.kind, e.path);
                    }

                    let _ = cs.send(LibraryWatcherCommand::Refresh);
                }
            }
        })
        .expect("Failed to create watcher");

        let mut current_path: Option<PathBuf> = None;

        while let Ok(command) = command_receiver.recv() {
            match command {
                LibraryWatcherCommand::Refresh => {
                    if let Some(ref path) = current_path {
                        let (tracks, playlists) = load_library_and_playlists(path);
                        let update_result = update_sender.send((tracks, playlists));
                        if update_result.is_err() {
                            error!("Failed update library. Shutting down the library watcher.");
                            return;
                        }
                    }
                }
                LibraryWatcherCommand::PathChange(new_path) => { // TODO: maybe just send refresh to self?
                    if let Some(ref old) = current_path {
                        let _ = debouncer.watcher().unwatch(old);
                    }

                    let watch_result = debouncer.watcher().watch(&new_path, RecursiveMode::Recursive);
                    if let Err(e) = watch_result {
                        error!("Failed to watch new folder: {:?}", e);
                    } else {
                        current_path = Some(new_path.clone());
                        let (tracks, playlists) = load_library_and_playlists(&new_path);
                        let update_result = update_sender.send((tracks, playlists));
                        if update_result.is_err() {
                            error!("Failed update library. Shutting down the library watcher.");
                            return;
                        }
                    }
                }
                LibraryWatcherCommand::Shutdown => { // TODO: maybe just send shutdown in other cases
                    info!("Received shutdown message. Shutting down the library watcher.");
                    return;
                }
            }
        }
    });

    Ok((command_sender, update_receiver))
}
