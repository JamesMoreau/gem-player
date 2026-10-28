use std::{
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc::Receiver,
    },
    time::Duration,
};

use anyhow::{Context, Result, bail};
use fully_pub::fully_pub;
use log::{error, info};
use notify::{FsEventWatcher, RecursiveMode};
use notify_debouncer_mini::{DebounceEventResult, Debouncer, new_debouncer};

use crate::{
    playlist::{Playlist, load_playlists_from_directory},
    track::{Track, load_tracks_from_directory},
};

#[fully_pub]
struct Library {
    tracks: Vec<Track>,
    playlists: Vec<Playlist>,
    directory: Option<PathBuf>,
    dirty: Arc<AtomicBool>,
    debouncer: Debouncer<FsEventWatcher>,
    reload_receiver: Option<Receiver<Result<LibraryAndPlaylists>>>,
}

pub type LibraryAndPlaylists = (Vec<Track>, Vec<Playlist>);

pub fn setup_library() -> Result<Library> {
    let dirty = Arc::new(AtomicBool::new(false));

    let callback_flag = Arc::clone(&dirty);
    let debouncer = new_debouncer(Duration::from_millis(500), move |res: DebounceEventResult| match res {
        Err(e) => error!("Watch error: {e}"),
        Ok(events) => {
            for e in events {
                info!("Event for {}", e.path.display());
            }

            callback_flag.store(true, Ordering::Relaxed);
        }
    })
    .context("Failed to create filesystem debouncer")?;

    Ok(Library {
        tracks: Vec::new(),
        playlists: Vec::new(),
        debouncer,
        dirty,
        directory: None,
        reload_receiver: None,
    })
}

pub fn load_library(directory: &Path) -> Result<LibraryAndPlaylists> {
    if !directory.is_dir() {
        bail!("Cannot load library: invalid directory {}", directory.display());
    }

    let library = load_tracks_from_directory(directory);
    let playlists = load_playlists_from_directory(directory);

    info!(
        "Loaded library from {}: {} tracks, {} playlists.",
        directory.display(),
        library.len(),
        playlists.len()
    );

    Ok((library, playlists))
}

pub fn set_library_directory(manager: &mut Library, new_directory: &Path) -> Result<()> {
    if !new_directory.is_dir() {
        bail!("Invalid library path: {}", new_directory.display());
    }

    if let Some(old) = &manager.directory {
        manager
            .debouncer
            .watcher()
            .unwatch(old)
            .with_context(|| format!("Failed to unwatch {}", old.display()))?;

        manager.directory = None;
    }

    if let Err(e) = manager.debouncer.watcher().watch(new_directory, RecursiveMode::Recursive) {
        bail!("Failed to watch {}: {e}", new_directory.display());
    }

    manager.directory = Some(new_directory.to_path_buf());
    manager.dirty.store(true, Ordering::Relaxed);

    Ok(())
}
