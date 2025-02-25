use std::{
    fs::{self, File},
    io::{self, ErrorKind, Write},
    path::{Path, PathBuf},
    time::SystemTime,
};

use fully_pub::fully_pub;
use glob::glob;
use log::error;
use uuid::Uuid;

use crate::{track::get_track_from_file, Track};

// Duplicates of tracks are not allowed.
#[fully_pub]
#[derive(Debug, Clone)]
pub struct Playlist {
    id: Uuid, // TODO: eventually remove this and just use m3u_path as id.
    name: String,
    creation_date_time: SystemTime,
    tracks: Vec<Track>,
    m3u_path: PathBuf,
}

pub fn find_playlist(playlist_id: Uuid, playlists: &[Playlist]) -> Option<&Playlist> {
    playlists.iter().find(|p| p.id == playlist_id)
}

pub fn find_playlist_mut(playlist_id: Uuid, playlists: &mut [Playlist]) -> Option<&mut Playlist> {
    playlists.iter_mut().find(|p| p.id == playlist_id)
}

pub fn add_a_track_to_playlist(playlist: &mut Playlist, track: Track) -> io::Result<()> {
    // TODO: This doesn't work if the same song has a different id. unless we create the id using the file path?
    if playlist.tracks.iter().any(|s| s.id == track.id) {
        return Err(io::Error::new(
            ErrorKind::Other,
            "The track is already in the playlist. Duplicates are not allowed.",
        ));
    }

    playlist.tracks.push(track);
    save_playlist_to_m3u(playlist)?;

    Ok(())
}

pub fn remove_a_track_from_playlist(playlist: &mut Playlist, track: &Track) -> io::Result<()> {
    let Some(index) = playlist.tracks.iter().position(|x| x == track) else {
        return Err(io::Error::new(
            ErrorKind::NotFound,
            "The track to be removed was not found in the playlist.",
        ));
    };

    playlist.tracks.remove(index);
    save_playlist_to_m3u(playlist)?;

    Ok(())
}

pub fn read_playlists_from_a_directory(path: &Path) -> io::Result<Vec<Playlist>> {
    let file_type = "m3u";
    let pattern = format!("{}/*.{}", path.to_string_lossy(), file_type);

    let mut m3u_paths = Vec::new();
    let result = glob(&pattern);
    match result {
        Ok(paths) => {
            for path in paths.filter_map(Result::ok) {
                m3u_paths.push(path);
            }
        }
        Err(e) => {
            return Err(io::Error::new(ErrorKind::Other, format!("Invalid pattern: {}", e)));
        }
    }

    let mut playlists = Vec::new();
    for path in m3u_paths {
        let result = get_playlist_from_m3u(&path);
        match result {
            Ok(playlist) => playlists.push(playlist),
            Err(e) => error!("{}", e),
        }
    }

    Ok(playlists)
}

pub fn save_playlist_to_m3u(playlist: &mut Playlist) -> io::Result<()> {
    let mut file = File::create(&playlist.m3u_path)?;

    for track in &playlist.tracks {
        let line = track.file_path.to_string_lossy();
        writeln!(file, "{}", line)?;
    }

    Ok(())
}

pub fn get_playlist_from_m3u(path: &Path) -> io::Result<Playlist> {
    let Some(extension) = path.extension() else {
        return Err(io::Error::new(ErrorKind::InvalidInput, "File has no extension"));
    };

    if extension.to_string_lossy().to_ascii_lowercase() != "m3u" {
        return Err(io::Error::new(ErrorKind::InvalidInput, "The file type is not .m3u"));
    }

    let id: Uuid = Uuid::new_v4();

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
        let maybe_track = get_track_from_file(&path);
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
        id,
        name,
        creation_date_time,
        tracks,
        m3u_path: path,
    })
}

pub fn rename_playlist(playlist: &mut Playlist, new_name: String) -> io::Result<()> {
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

pub fn create_a_new_playlist(name: String, directory: &Path) -> io::Result<Playlist> {
    let filename = format!("{}.m3u", name);
    let file_path = directory.join(filename);

    let mut playlist = Playlist {
        id: Uuid::new_v4(),
        name,
        creation_date_time: SystemTime::now(),
        tracks: Vec::new(),
        m3u_path: file_path,
    };

    save_playlist_to_m3u(&mut playlist)?;

    Ok(playlist)
}

// Removes the playlist from the list and deletes the associated m3u file.
pub fn delete_playlist(playlist_id: Uuid, playlists: &mut Vec<Playlist>) -> Result<(), String> {
    let Some(index) = playlists.iter().position(|p| p.id == playlist_id) else {
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
