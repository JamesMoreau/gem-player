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

pub fn _find<'a>(playlist_identifier: &Path, playlists: &'a [Playlist]) -> Option<&'a Playlist> {
    playlists.iter().find(|p| p.m3u_path == playlist_identifier)
}

pub fn find_mut<'a>(playlist_identifier: &Path, playlists: &'a mut [Playlist]) -> Option<&'a mut Playlist> {
    playlists.iter_mut().find(|p| p.m3u_path == playlist_identifier)
}

pub fn add_a_track_to_playlist(playlist: &mut Playlist, track: Track) -> io::Result<()> {
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

pub fn remove_track(playlist: &mut Playlist, track: &Track) -> io::Result<()> {
    let Some(index) = playlist.tracks.iter().position(|x| x == track) else {
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

        let is_m3u_file = path.is_file() && path.extension().map_or(false, |ext| ext.to_ascii_lowercase() == "m3u");
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

    for track in &playlist.tracks {
        let line = track.path.to_string_lossy();
        writeln!(file, "{}", line)?;
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
    let maybe_stem = path.file_stem();
    if let Some(stem) = maybe_stem {
        name = stem.to_string_lossy().to_string();
    }

    let file_contents = fs::read_to_string(path)?;
    let mut tracks = Vec::new();
    for line in file_contents.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with("#") {
            continue;
        }

        let path = PathBuf::from(trimmed);
        let maybe_track = load_from_file(&path);
        match maybe_track {
            Ok(track) => tracks.push(track),
            Err(err) => {
                error!("{}", err);
                continue;
            }
        }
    }

    let mut creation_date_time = SystemTime::now();
    let metadata_result = fs::metadata(path);
    match metadata_result {
        Err(err) => error!("{}", err),
        Ok(metadata) => {
            let created_result = metadata.created();
            match created_result {
                Err(err) => error!("{}", &err),
                Ok(created) => {
                    creation_date_time = created;
                }
            }
        }
    };

    let path = path.to_path_buf();

    Ok(Playlist {
        name,
        creation_date_time,
        tracks,
        m3u_path: path,
    })
}

pub fn rename(playlist: &mut Playlist, new_name: String) -> io::Result<()> {
    let Some(directory) = playlist.m3u_path.parent() else {
        return Err(io::Error::new(ErrorKind::InvalidInput, "Playlist path has no parent directory."));
    };

    let new_filename = format!("{}.m3u", new_name);
    let new_path = directory.join(new_filename);

    fs::rename(&playlist.m3u_path, &new_path)?;

    playlist.name = new_name;
    playlist.m3u_path = new_path;

    Ok(())
}

pub fn create(name: String, directory: &Path) -> io::Result<Playlist> {
    let filename = format!("{}.m3u", name);
    let file_path = directory.join(filename);

    let mut playlist = Playlist {
        name,
        creation_date_time: SystemTime::now(),
        tracks: Vec::new(),
        m3u_path: file_path,
    };

    save_to_m3u(&mut playlist)?;

    Ok(playlist)
}

// Removes the playlist from the list and deletes the associated m3u file.
pub fn delete(playlist_identifier: &Path, playlists: &mut Vec<Playlist>) -> Result<(), String> {
    let Some(index) = playlists.iter().position(|p| p.m3u_path == playlist_identifier) else {
        return Err("Playlist not found in library".to_string());
    };

    let playlist = playlists.remove(index);

    // Send the m3u file to the trash!
    let result = trash::delete(playlist.m3u_path);
    if let Err(e) = result {
        return Err(e.to_string());
    }

    Ok(())
}
