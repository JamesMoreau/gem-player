use std::{
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
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
struct LibraryManager {
    debouncer: Debouncer<FsEventWatcher>,
    library_dirty: Arc<AtomicBool>,
}

pub type LibraryAndPlaylists = (Vec<Track>, Vec<Playlist>);

pub fn setup_library_manager() -> Result<LibraryManager> {
    let library_dirty = Arc::new(AtomicBool::new(false));

    let callback_flag = Arc::clone(&library_dirty);
    let debouncer = new_debouncer(Duration::from_millis(500), move |res: DebounceEventResult| match res {
        Err(e) => error!("watch error: {e}"),
        Ok(events) => {
            for e in events {
                info!("Event for {}", e.path.display());
            }

            callback_flag.store(true, Ordering::Relaxed);
        }
    })
    .context("failed to create filesystem debouncer")?;

    Ok(LibraryManager { debouncer, library_dirty })
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

pub fn set_watcher_directory(watcher: &mut LibraryManager, old_directory: Option<&Path>, new_directory: &Path) -> Result<()> {
    if !new_directory.is_dir() {
        bail!("Invalid library path: {}", new_directory.display());
    }

    if let Some(old) = old_directory {
        watcher
            .debouncer
            .watcher()
            .unwatch(old)
            .with_context(|| format!("Failed to unwatch {}", old.display()))?;
    }

    watcher
        .debouncer
        .watcher()
        .watch(new_directory, RecursiveMode::Recursive)
        .with_context(|| format!("Failed to watch {}", new_directory.display()))?;

    watcher.library_dirty.store(true, Ordering::Relaxed);

    Ok(())
}
