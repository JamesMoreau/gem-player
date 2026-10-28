use std::{
    path::{Path, PathBuf},
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
    library_directory: Option<PathBuf>,
}

pub type LibraryAndPlaylists = (Vec<Track>, Vec<Playlist>);

pub fn setup_library_manager() -> Result<LibraryManager> {
    let library_dirty = Arc::new(AtomicBool::new(false));

    let callback_flag = Arc::clone(&library_dirty);
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

    Ok(LibraryManager {
        debouncer,
        library_dirty,
        library_directory: None,
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

pub fn set_library_directory(manager: &mut LibraryManager, new_directory: &Path) -> Result<()> {
    if !new_directory.is_dir() {
        bail!("Invalid library path: {}", new_directory.display());
    }

    if let Some(old) = &manager.library_directory {
        manager
            .debouncer
            .watcher()
            .unwatch(old)
            .with_context(|| format!("Failed to unwatch {}", old.display()))?;

        manager.library_directory = None;
    }

    if let Err(e) = manager.debouncer.watcher().watch(new_directory, RecursiveMode::Recursive) {
        bail!("Failed to watch {}: {e}", new_directory.display());
    }

    manager.library_directory = Some(new_directory.to_path_buf());
    manager.library_dirty.store(true, Ordering::Relaxed);

    Ok(())
}
