use crate::{track::load_from_file, Track};
use fully_pub::fully_pub;
use log::error;
use std::{
    fs::{self, File},
    io::{self, ErrorKind, Write},
    path::{Path, PathBuf},
    time::SystemTime,
};
use walkdir::WalkDir;

#[fully_pub]
#[derive(Debug, Clone)]
pub struct Playlist {
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

pub fn add_to_playlist(playlist: &mut Playlist, track: Track) -> io::Result<()> {
    if playlist.tracks.iter().any(|s| *s == track) {
        return Err(io::Error::new(
            ErrorKind::Other,
            "The track is already in the playlist. Duplicates are not allowed.",
        ));
    }

    playlist.tracks.push(track);
    save_to_m3u(playlist)?;

    Ok(())
}

pub fn remove_from_playlist(playlist: &mut Playlist, track_key: &Path) -> io::Result<()> {
    let Some(index) = playlist.tracks.iter().position(|t: &Track| t.path == track_key) else {
        return Err(io::Error::new(
            ErrorKind::NotFound,
            "The track to be removed was not found in the playlist.",
        ));
    };

    playlist.tracks.remove(index);
    save_to_m3u(playlist)?;

    Ok(())
}

pub fn read_all_from_a_directory(directory: &Path) -> io::Result<Vec<Playlist>> {
    let mut playlists = Vec::new();

    for entry in WalkDir::new(directory).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();

        let is_m3u_file = path.is_file() && path.extension().is_some_and(|ext| ext.eq_ignore_ascii_case("m3u"));
        if !is_m3u_file {
            continue;
        }

        match load_from_m3u(path) {
            Err(e) => error!("{}", e),
            Ok(playlist) => playlists.push(playlist),
        }
    }

    playlists.sort_by(|a, b| a.creation_date_time.cmp(&b.creation_date_time));

    Ok(playlists)
}

pub fn save_to_m3u(playlist: &mut Playlist) -> io::Result<()> {
    let mut file = File::create(&playlist.m3u_path)?;
    let directory = playlist.m3u_path.parent().unwrap_or_else(|| Path::new(""));

    for track in &playlist.tracks {
        let relative_path = match track.path.strip_prefix(directory) {
            Ok(path) => path.to_string_lossy().into_owned(),
            Err(_) => {
                error!("Failed to strip prefix from path: {}", track.path.display());
                track.path.to_string_lossy().into_owned() // If we can't strip the prefix, just use the full path.
            }
        };

        writeln!(file, "{}", relative_path)?;
    }

    Ok(())
}

pub fn load_from_m3u(path: &Path) -> io::Result<Playlist> {
    let Some(extension) = path.extension() else {
        return Err(io::Error::new(ErrorKind::InvalidInput, "File has no extension"));
    };

    if extension.to_string_lossy().to_ascii_lowercase() != "m3u" {
        return Err(io::Error::new(ErrorKind::InvalidInput, "The file type is not .m3u"));
    }

    let mut name = "Unnamed Playlist".to_owned();
    if let Some(stem) = path.file_stem() {
        name = stem.to_string_lossy().to_string();
    }

    let directory = path.parent().unwrap_or_else(|| Path::new(""));
    let file_contents = fs::read_to_string(path)?;
    let mut tracks = Vec::new();
    for line in file_contents.lines() {
        let trimmed_line = line.trim();
        if trimmed_line.is_empty() || trimmed_line.starts_with("#") {
            continue;
        }

        let relative_path = Path::new(trimmed_line);
        let full_path = if relative_path.is_absolute() {
            relative_path.to_path_buf()
        } else {
            directory.join(relative_path)
        };

        let maybe_track = load_from_file(&full_path);
        match maybe_track {
            Ok(track) => tracks.push(track),
            Err(err) => {
                error!("Failed to load track '{}': {}", full_path.to_string_lossy(), err);
                continue;
            }
        }
    }

    let creation_date_time = fs::metadata(path)
        .and_then(|metadata| metadata.created())
        .unwrap_or_else(|_| SystemTime::now());

    Ok(Playlist {
        name,
        creation_date_time,
        tracks,
        m3u_path: path.to_path_buf(),
    })
}

pub fn rename(playlist: &mut Playlist, new_name: String) -> io::Result<()> {
    let Some(directory) = playlist.m3u_path.parent() else {
        return Err(io::Error::new(ErrorKind::InvalidInput, "Playlist path has no parent directory."));
    };

    let sanitized_name = sanitize_filename::sanitize(new_name.trim());
    if sanitized_name.is_empty() {
        return Err(io::Error::new(ErrorKind::InvalidInput, "Playlist name cannot be empty."));
    }

    let new_filename = format!("{}.m3u", sanitized_name);
    let new_path = directory.join(new_filename);

    if new_path.exists() {
        return Err(io::Error::new(
            ErrorKind::AlreadyExists,
            "A playlist with this name already exists.",
        ));
    }

    fs::rename(&playlist.m3u_path, &new_path)?;

    playlist.name = sanitized_name;
    playlist.m3u_path = new_path;

    Ok(())
}

pub fn create(name: String, directory: &Path) -> io::Result<Playlist> {
    let sanitized_name = sanitize_filename::sanitize(name.trim());
    if sanitized_name.is_empty() {
        return Err(io::Error::new(ErrorKind::InvalidInput, "Playlist name cannot be empty."));
    }

    if !directory.exists() {
        return Err(io::Error::new(ErrorKind::NotFound, "The specified directory does not exist."));
    }

    let filename = format!("{}.m3u", sanitized_name);
    let file_path = directory.join(&filename);

    if file_path.exists() {
        return Err(io::Error::new(
            ErrorKind::AlreadyExists,
            "A playlist with this name already exists.",
        ));
    }

    fs::File::create(&file_path)?;

    let mut playlist = Playlist {
        name: sanitized_name,
        creation_date_time: SystemTime::now(),
        tracks: Vec::new(),
        m3u_path: file_path,
    };

    save_to_m3u(&mut playlist)?;

    Ok(playlist)
}

// Removes the playlist from the list and deletes the associated m3u file.
pub fn delete(playlist_key: &Path, playlists: &mut Vec<Playlist>) -> Result<(), String> {
    let index = playlists
        .iter()
        .position(|p| p.m3u_path == playlist_key)
        .expect("Playlist not found in library");
    let playlist = playlists.remove(index);

    // Send the m3u file to the trash!
    let result = trash::delete(playlist.m3u_path);
    if let Err(e) = result {
        return Err(e.to_string());
    }

    Ok(())
}
