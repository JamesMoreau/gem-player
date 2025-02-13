use std::{
    fs::{self, File},
    io::{self, ErrorKind, Write},
    path::{Path, PathBuf},
};

use chrono::{DateTime, Utc};
use fully_pub::fully_pub;
use uuid::Uuid;

use crate::{player::get_song_from_file, print_error, Song};

#[fully_pub]
#[derive(Debug, Clone)]
pub struct Playlist {
    id: Uuid,
    name: String,
    creation_date_time: DateTime<Utc>,
    songs: Vec<Song>,
    path: Option<PathBuf>,
}

pub fn _add_songs_to_playlist(playlist: &mut Playlist, songs: Vec<Song>) {
    playlist.songs.extend(songs);
}

pub fn save_playlist_to_m3u(playlist: &mut Playlist, directory: &Path) -> io::Result<()> {
    let filename = format!("{}.m3u", playlist.name);
    let file_path = directory.join(filename);

    let mut file = File::create(&file_path)?;

    for song in &playlist.songs {
        let line = song.file_path.to_string_lossy();
        writeln!(file, "{}", line)?;
    }

    playlist.path = Some(file_path);

    Ok(())
}

pub fn _load_playlist_from_m3u(path: &Path) -> io::Result<Playlist> {
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
                print_error(err);
                continue;
            }
        }
    }

    let mut creation_date_time = Utc::now();
    let metadata_result = fs::metadata(path);
    match metadata_result {
        Err(err) => print_error(err),
        Ok(metadata) => {
            let created_result = metadata.created();
            match created_result {
                Err(err) => print_error(&err),
                Ok(created) => {
                    creation_date_time = created.into();
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

fn _rename_playlist_file(old_name: &str, new_name: &str) -> io::Result<()> {
    //TODO fix!
    let old_filename = format!("{}.m3u", old_name);
    let new_filename = format!("{}.m3u", new_name);
    fs::rename(old_filename, new_filename)
}

pub fn create_a_new_playlist(name: &str) -> Playlist {
    Playlist {
        id: Uuid::new_v4(),
        name: name.to_owned(),
        creation_date_time: Utc::now(),
        songs: Vec::new(),
        path: None,
    }
}
