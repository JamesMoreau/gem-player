use std::{
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc::{Receiver, channel},
    },
    thread,
    time::Duration,
};

use anyhow::{Context, Result, bail};
use fully_pub::fully_pub;
use log::{error, info};
use notify::{FsEventWatcher, RecursiveMode};
use notify_debouncer_mini::{DebounceEventResult, Debouncer, new_debouncer};
use rfd::FileDialog;

use crate::{
    playlist::{Playlist, load_playlists_from_directory},
    track::{Track, load_tracks_from_directory},
};

#[cfg(target_os = "macos")]
use apple_scoped_bookmarks::{Bookmark, ScopedAccess};

#[fully_pub]
struct Library {
    tracks: Vec<Track>,
    playlists: Vec<Playlist>,

    directory: Option<PathBuf>,

    #[cfg(target_os = "macos")]
    bookmark: Option<Bookmark>,
    #[cfg(target_os = "macos")]
    access: Option<ScopedAccess>,

    dirty: Arc<AtomicBool>,
    debouncer: Debouncer<FsEventWatcher>,
    reload_receiver: Option<Receiver<Result<LibraryAndPlaylists>>>,

    #[cfg(any(target_os = "windows", target_os = "linux"))]
    folder_picker_receiver: Option<Receiver<Option<PathBuf>>>, // None -> No folder picker dialog. Some -> Folder picker dialog open.

    #[cfg(target_os = "macos")]
    folder_picker_receiver: Option<Receiver<Option<(PathBuf, Bookmark)>>>, // Addionally store a security bookmark on macos.
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
        directory: None,
        #[cfg(target_os = "macos")]
        bookmark: None,
        #[cfg(target_os = "macos")]
        access: None,
        dirty,
        debouncer,
        reload_receiver: None,
        folder_picker_receiver: None,
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

pub fn set_library_directory(library: &mut Library, new_directory: &Path) -> Result<()> {
    if !new_directory.is_dir() {
        bail!("Invalid library path: {}", new_directory.display());
    }

    if library.directory.as_deref() == Some(new_directory) {
        return Ok(());
    }

    library
        .debouncer
        .watcher()
        .watch(new_directory, RecursiveMode::Recursive)
        .with_context(|| format!("Failed to watch {}", new_directory.display()))?;

    if let Some(old_directory) = &library.directory
        && let Err(e) = library.debouncer.watcher().unwatch(old_directory)
    {
        // Roll back the new watch to preserve the previous configuration.
        if let Err(rollback_error) = library.debouncer.watcher().unwatch(new_directory) {
            error!("Failed to roll back watch on {}: {rollback_error}", new_directory.display());
        }

        return Err(e).with_context(|| format!("Failed to unwatch {}", old_directory.display()));
    }

    library.directory = Some(new_directory.to_path_buf());
    library.dirty.store(true, Ordering::Relaxed);

    Ok(())
}

/// Spawns a folder picker in a background thread and returns the receiver where the selected folder will eventually be sent.
#[cfg(any(target_os = "windows", target_os = "linux"))]
pub fn spawn_library_folder_picker(start_dir: &Path) -> Receiver<Option<PathBuf>> {
    let (sender, receiver) = channel();
    let start_dir = start_dir.to_path_buf();

    thread::spawn(move || {
        let selected_directory = FileDialog::new()
            .set_directory(start_dir)
            .pick_folder()
            .map(|path| path.to_path_buf());

        let _ = sender.send(selected_directory);
    });

    receiver
}

// Same as above but uses the security bookmark api to save access to the folder / directory.
#[cfg(target_os = "macos")]
pub fn spawn_library_folder_picker(start_dir: &Path) -> Receiver<Option<(PathBuf, Bookmark)>> {
    let (sender, receiver) = channel();
    let start_dir = start_dir.to_path_buf();

    thread::spawn(move || {
        let selected_directory = FileDialog::new()
            .set_directory(start_dir)
            .pick_folder()
            .and_then(|path| match Bookmark::create(&path) {
                Ok(bookmark) => Some((path, bookmark)),
                Err(e) => {
                    error!("Failed to create security bookmark: {e}");
                    None
                }
            });

        let _ = sender.send(selected_directory);
    });

    receiver
}
