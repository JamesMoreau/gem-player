use std::{
    fs::{self, File},
    io::{self, ErrorKind, Write},
    path::{Path, PathBuf},
    time::SystemTime,
};

use fully_pub::fully_pub;
use glob::glob;
use log::{error, warn};
use uuid::Uuid;

use crate::{song::get_song_from_file, Song};

// Duplicates of songs are not allowed.
#[fully_pub]
#[derive(Debug, Clone)]
pub struct Playlist {
    id: Uuid,
    name: String,
    creation_date_time: SystemTime,
    songs: Vec<Song>, 
    path: Option<PathBuf>,
}

pub fn add_a_song_to_playlist(playlist: &mut Playlist, song: Song) {
    if playlist.songs.iter().any(|s| s.id == song.id) {
        return;
    }
    
    playlist.songs.push(song);
}

pub fn remove_a_song_from_playlist(playlist: &mut Playlist, song: &Song) -> Result<(), String> {
    let Some(index) = playlist.songs.iter().position(|x| x.id == song.id) else {
        return Err("Song not found in playlist".to_string());
    };

    playlist.songs.remove(index);
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

pub fn save_playlist_to_m3u(playlist: &mut Playlist, directory: &Path) -> io::Result<()> {
    let filename = format!("{}.m3u", playlist.name);
    let file_path = directory.join(filename);

    let mut file = File::create(&file_path)?;

    for song in &playlist.songs {
        let line = song.file_path.to_string_lossy();
        writeln!(file, "{}", line)?;
    }

    // Update the object once the file operations are successful.
    playlist.path = Some(file_path);

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
    let mut songs = Vec::new();
    for line in file_contents.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with("#") {
            continue;
        }

        let path = PathBuf::from(trimmed);
        let maybe_song = get_song_from_file(&path);
        match maybe_song {
            Ok(song) => songs.push(song),
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

    let path = Some(path.to_path_buf());

    Ok(Playlist {
        id,
        name,
        creation_date_time,
        songs,
        path,
    })
}

pub fn rename_playlist(playlist: &mut Playlist, new_name: String) -> io::Result<()> {
    let Some(old_path) = playlist.path.as_ref() else {
        return Err(io::Error::new(ErrorKind::NotFound, "Playlist has no associated file path"));
    };

    let Some(directory) = old_path.parent() else {
        return Err(io::Error::new(ErrorKind::InvalidInput, "Playlist path has no parent directory"));
    };

    let new_filename = format!("{}.m3u", new_name);
    let new_path = directory.join(new_filename);

    fs::rename(old_path, &new_path)?;

    playlist.name = new_name;
    playlist.path = Some(new_path);

    Ok(())
}

pub fn create_a_new_playlist(name: String, directory: &Path) -> io::Result<Playlist> {
    let mut playlist = Playlist {
        id: Uuid::new_v4(),
        name,
        creation_date_time: SystemTime::now(),
        songs: Vec::new(),
        path: None,
    };

    save_playlist_to_m3u(&mut playlist, directory)?;

    Ok(playlist)
}

// Removes the playlist from the list and deletes the associated m3u file.
pub fn delete_playlist(playlist_to_delete: &Playlist, playlists: &mut Vec<Playlist>) -> Result<(), String> {
    // Remove the playlist before deleting the m3u file so that the app and file state remains consistent.
    let Some(index) = playlists.iter().position(|x| x.id == playlist_to_delete.id) else {
        return Err("Playlist not found in library".to_string());
    };

    let playlist = playlists.remove(index);

    if let Err(err) = delete_playlist_m3u(&playlist) {
        warn!("Warning: Failed to delete the playlist's associated m3u file: {}", err);
    }

    Ok(())
}

pub fn delete_playlist_m3u(playlist: &Playlist) -> io::Result<()> {
    let Some(path) = playlist.path.as_ref() else {
        return Err(io::Error::new(ErrorKind::NotFound, "Playlist has no associated file path"));
    };

    // Send the m3u file to the trash!
    let result = trash::delete(path);
    if let Err(e) = result {
        return Err(io::Error::new(ErrorKind::Other, e));
    }

    Ok(())
}
