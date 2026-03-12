use crate::{track::load_from_file, Track};
use anyhow::{anyhow, bail, Context, Result};
use fully_pub::fully_pub;
use log::warn;
use std::{
    fs::{self, metadata, read_to_string, File},
    io::Write,
    path::{Path, PathBuf},
    time::SystemTime,
};
use walkdir::WalkDir;

#[fully_pub]
struct Playlist {
    name: String,
    creation_date_time: SystemTime,
    tracks: Vec<Track>, // Duplicates of tracks are not allowed.
    m3u_path: PathBuf,
}

impl PartialEq for Playlist {
    #[inline]
    fn eq(&self, other: &Playlist) -> bool {
        self.m3u_path == other.m3u_path
    }
}

pub trait PlaylistRetrieval {
    fn get_by_path(&self, path: &Path) -> &Playlist;
    fn get_by_path_mut(&mut self, path: &Path) -> &mut Playlist;
}

impl PlaylistRetrieval for Vec<Playlist> {
    fn get_by_path(&self, path: &Path) -> &Playlist {
        self.iter().find(|p| p.m3u_path == path).expect("Playlist not found")
    }

    fn get_by_path_mut(&mut self, path: &Path) -> &mut Playlist {
        self.iter_mut().find(|p| p.m3u_path == path).expect("Playlist not found")
    }
}

pub fn add_to_playlist(playlist: &mut Playlist, track: Track) -> Result<()> {
    if playlist.tracks.contains(&track) {
        bail!(
            "The track '{}' is already in the playlist. Duplicates are not allowed.",
            track.path.display()
        );
    }

    playlist.tracks.push(track);
    save_to_m3u(playlist).context("Failed to persist playlist after adding track")?;

    Ok(())
}

pub fn remove_from_playlist(playlist: &mut Playlist, track_key: &Path) -> Result<()> {
    let index = playlist
        .tracks
        .iter()
        .position(|t: &Track| t.path == track_key)
        .ok_or_else(|| anyhow!("Track '{}' not found in playlist", track_key.display()))?;

    playlist.tracks.remove(index);
    save_to_m3u(playlist).context("Failed to persist playlist after removing track")?;

    Ok(())
}

pub fn load_playlists_from_directory(directory: &Path) -> Vec<Playlist> {
    let mut playlists = Vec::new();

    for entry in WalkDir::new(directory).into_iter().filter_map(|e| {
        if let Err(err) = &e {
            warn!("Failed to read directory entry: {}", err);
        }
        e.ok()
    }) {
        let path = entry.path();

        if !is_m3u_file(path) {
            continue;
        }

        match load_from_m3u(path) {
            Ok(playlist) => playlists.push(playlist),
            Err(e) => {
                warn!("Failed to load playlist {:?}: {}", path, e);
            }
        }
    }

    playlists.sort_by_key(|p| p.creation_date_time);
    playlists
}

pub fn is_m3u_file(path: &Path) -> bool {
    path.is_file() && path.extension().is_some_and(|ext| ext.eq_ignore_ascii_case("m3u"))
}

pub fn save_to_m3u(playlist: &mut Playlist) -> Result<()> {
    let mut file =
        File::create(&playlist.m3u_path).with_context(|| format!("Failed to create playlist file '{}'", playlist.m3u_path.display()))?;

    let directory = playlist.m3u_path.parent().unwrap_or_else(|| Path::new(""));

    for track in &playlist.tracks {
        let path = track.path.strip_prefix(directory).unwrap_or(&track.path);
        writeln!(file, "{}", path.to_string_lossy())
            .with_context(|| format!("Failed to write track '{}' to playlist", track.path.display()))?;
    }

    Ok(())
}

pub fn load_from_m3u(path: &Path) -> Result<Playlist> {
    if !is_m3u_file(path) {
        bail!("The file '{}' is not an M3U playlist", path.display());
    }

    let name = path
        .file_stem()
        .map(|stem| stem.to_string_lossy().into_owned())
        .unwrap_or_else(|| "Unnamed Playlist".to_string());

    let directory = path.parent().unwrap_or_else(|| Path::new(""));
    let file_contents = read_to_string(path).with_context(|| format!("Failed to read playlist file '{}'", path.display()))?;

    let mut tracks = Vec::new();

    for line in file_contents
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
    {
        let relative_path = Path::new(line);
        let full_path = if relative_path.is_absolute() {
            relative_path.to_path_buf()
        } else {
            directory.join(relative_path)
        };

        match load_from_file(&full_path) {
            Ok(track) => tracks.push(track),
            Err(err) => {
                warn!("Skipping invalid track '{}': {}", full_path.display(), err);
            }
        }
    }

    let creation_date_time = metadata(path)
        .and_then(|metadata| metadata.created())
        .unwrap_or_else(|_| SystemTime::now());

    Ok(Playlist {
        name,
        creation_date_time,
        tracks,
        m3u_path: path.to_path_buf(),
    })
}

pub fn rename(playlist: &mut Playlist, new_name: String) -> Result<()> {
    let directory = playlist
        .m3u_path
        .parent()
        .ok_or_else(|| anyhow!("Playlist path has no parent directory"))?;

    let sanitized_name = sanitize_filename::sanitize(new_name.trim());
    if sanitized_name.is_empty() {
        bail!("Playlist name cannot be empty.");
    }

    let new_filename = format!("{}.m3u", sanitized_name);
    let new_path = directory.join(new_filename);

    if new_path.exists() {
        bail!("A playlist with this name already exists");
    }

    fs::rename(&playlist.m3u_path, &new_path)
        .with_context(|| format!("Failed to rename '{}' to '{}'", playlist.m3u_path.display(), new_path.display()))?;

    playlist.name = sanitized_name;
    playlist.m3u_path = new_path;

    Ok(())
}

pub fn create(name: String, directory: &Path) -> Result<Playlist> {
    let sanitized_name = sanitize_filename::sanitize(name.trim());
    if sanitized_name.is_empty() {
        bail!("Playlist name cannot be empty.");
    }

    if !directory.exists() {
        bail!("The specified directory does not exist: {}", directory.display());
    }

    let extension = ".m3u";
    let filename = format!("{}{}", sanitized_name, extension);
    let file_path = directory.join(&filename);

    if file_path.exists() {
        bail!("A playlist with this name already exists.");
    }

    File::create(&file_path).with_context(|| format!("Failed to create playlist file '{}'", file_path.display()))?;

    let mut playlist = Playlist {
        name: sanitized_name,
        creation_date_time: SystemTime::now(),
        tracks: Vec::new(),
        m3u_path: file_path,
    };

    save_to_m3u(&mut playlist).context("Failed to initialize playlist file contents")?;

    Ok(playlist)
}

/// Removes the playlist from the list and deletes the associated m3u file.
pub fn delete(playlist_key: &Path, playlists: &mut Vec<Playlist>) -> Result<()> {
    let index = playlists
        .iter()
        .position(|p| p.m3u_path == playlist_key)
        .ok_or_else(|| anyhow!("Playlist '{}' not found in library", playlist_key.display()))?;

    let playlist = playlists.remove(index);

    // Send the m3u file to the trash!
    trash::delete(&playlist.m3u_path).with_context(|| format!("Failed to delete playlist file '{}'", playlist.m3u_path.display()))?;

    Ok(())
}
