use crate::{sort_songs, ui, Playlist, Song, SortBy, SortOrder};
use glob::glob;
use lofty::{
    file::{AudioFile, TaggedFileExt},
    tag::ItemKey,
};
use rand::seq::SliceRandom;
use rodio::{Decoder, OutputStream, Sink};
use std::{
    io::BufReader,
    path::{Path, PathBuf},
};

pub const SUPPORTED_AUDIO_FILE_TYPES: [&str; 6] = ["mp3", "m4a", "wav", "flac", "ogg", "opus"];

pub struct GemPlayer {
    pub current_view: ui::View,
    pub theme: String,
    pub search_text: String,
    pub sort_by: SortBy,
    pub sort_order: SortOrder,

    pub songs: Vec<Song>,
    pub queue: Vec<Song>,
    pub current_song_index: Option<usize>, // Index of the current song in the queue.
    pub selected_song: Option<usize>, // Index of the selected song in the songs vector.
    pub current_song: Option<Song>,   // The currently playing song.

    pub shuffle: bool,
    pub repeat: bool,
    pub muted: bool,
    pub volume_before_mute: Option<f32>,
    pub paused_before_scrubbing: Option<bool>, // None if not scrubbing, Some(true) if paused, Some(false) if playing.

    pub _stream: OutputStream, // Holds the OutputStream to keep it alive
    pub sink: Sink,            // Controls playback (play, pause, stop, etc.)

    pub music_directory: Option<PathBuf>, // The directory where music is stored.
    pub playlists: Vec<Playlist>,
}

impl GemPlayer {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        egui_extras::install_image_loaders(&cc.egui_ctx);

        egui_material_icons::initialize(&cc.egui_ctx);

        let (_stream, handle) = OutputStream::try_default().unwrap();
        let sink = Sink::try_new(&handle).unwrap();
        sink.pause();
        sink.set_volume(0.6);

        let mut default_self = Self {
            current_view: ui::View::Library,
            theme: "Default".to_owned(),
            search_text: String::new(),
            sort_by: SortBy::Title,
            sort_order: SortOrder::Ascending,

            songs: Vec::new(),
            queue: Vec::new(),
            current_song_index: None,
            selected_song: None,
            current_song: None,

            shuffle: false,
            repeat: false,
            muted: false,
            volume_before_mute: None,
            paused_before_scrubbing: None,

            _stream,
            sink,

            music_directory: None,
            playlists: Vec::new(),
        };

        // Find the music directory.
        let audio_directory = match dirs::audio_dir() {
            Some(dir) => dir,
            None => {
                println!("No music directory found.");
                return default_self;
            }
        };
        let my_music_directory = audio_directory.join("MyMusic");
        default_self.music_directory = Some(my_music_directory);

        let songs = match &default_self.music_directory {
            Some(path) => {
                let result = read_music_from_directory(path);
                match result {
                    Ok(songs) => songs,
                    Err(e) => {
                        println!("{}", e);
                        Vec::new()
                    }
                }
            },
            None => Vec::new(),
        };
        println!("Found {} songs", &songs.len());
        sort_songs(&mut default_self.songs, default_self.sort_by, default_self.sort_order);

        Self { songs, ..default_self }
    }
}

pub fn is_playing(gem_player: &mut GemPlayer) -> bool {
    !gem_player.sink.is_paused()
}

pub fn play_or_pause(gem_player: &mut GemPlayer) {
    if gem_player.sink.is_paused() {
        gem_player.sink.play()
    } else {
        gem_player.sink.pause()
    }
}

pub fn play_next_song_in_queue(gem_player: &mut GemPlayer) {
    if let Some(next_song) = gem_player.queue.pop() {
        load_and_play_song(gem_player, &next_song);
    } else {
        println!("No songs in queue.");
    }
}

pub fn play_previous_song_in_queue(gem_player: &mut GemPlayer) {
    println!("Not implemented yet.");
    todo!();
}

// TODO: Is this ok to call this function from the UI thread since we are doing heavy events like loading a file?
pub fn load_and_play_song(gem_player: &mut GemPlayer, song: &Song) {
    gem_player.sink.stop(); // Stop the current song if any.

    let file_result = std::fs::File::open(&song.file_path);
    let file = match file_result {
        Ok(file) => file,
        Err(e) => {
            println!("Error opening file: {:?}", e);
            return;
        }
    };

    let source_result = Decoder::new(BufReader::new(file));
    let source = match source_result {
        Ok(source) => source,
        Err(e) => {
            println!("Error decoding file: {}, Error: {:?}", song.file_path.to_string_lossy(), e);
            return;
        }
    };

    gem_player.current_song = Some(song.clone());

    gem_player.sink.append(source);
    gem_player.sink.play();
}

pub fn get_song_from_file(path: &Path) -> Option<Song> {
    if !path.is_file() {
        println!("Path is not a file: {:?}", path);
        return None;
    }

    let result_file = lofty::read_from_path(path);
    let tagged_file = match result_file {
        Ok(file) => file,
        Err(e) => {
            println!("Error reading file {}: {}", path.display(), e);
            return None;
        }
    };

    let tag = match tagged_file.primary_tag() {
        Some(tag) => tag,
        None => tagged_file.first_tag()?,
    };

    let title = tag
        .get_string(&ItemKey::TrackTitle)
        .map(|t| t.to_owned())
        .or_else(|| path.file_stem().and_then(|s| s.to_str()).map(|s| s.to_owned()));

    let artist = tag.get_string(&ItemKey::TrackArtist).map(|a| a.to_owned());

    let album = tag.get_string(&ItemKey::AlbumTitle).map(|a| a.to_owned());

    let properties = tagged_file.properties();
    let duration = properties.duration();

    let artwork_result = tag.pictures().first();
    let artwork = artwork_result.map(|artwork| artwork.data().to_vec());

    let file_path = path.to_path_buf();

    Some(Song {
        title,
        artist,
        album,
        duration,
        artwork,
        file_path,
    })
}

pub fn read_music_from_directory(path: &Path) -> Result<Vec<Song>, String> {
    let mut songs = Vec::new();
    let mut file_paths = Vec::new();

    let patterns = SUPPORTED_AUDIO_FILE_TYPES
        .iter()
        .map(|file_type| format!("{}/*.{}", path.to_string_lossy(), file_type))
        .collect::<Vec<String>>();

    for pattern in patterns {
        let file_paths_result = glob(&pattern);
        match file_paths_result {
            Ok(paths) => {
                for path in paths.filter_map(Result::ok) {
                    file_paths.push(path);
                }
            }
            Err(e) => {
                return Err(format!("Error reading pattern {}: {}", pattern, e));
            }
        }
    }

    if file_paths.is_empty() {
        return Err(format!("No music files found in directory: {:?}", path));
    }

    for entry in file_paths {
        let maybe_song = get_song_from_file(&entry);
        match maybe_song {
            Some(song) => songs.push(song),
            None => eprintln!("Error reading song from file: {:?}", entry),
        }
    }

    Ok(songs)
}

pub fn add_song_to_queue(gem_player: &mut GemPlayer, song: Song) {
    gem_player.queue.push(song);
}

pub fn remove_song_from_queue(gem_player: &mut GemPlayer, index: usize) {
    gem_player.queue.remove(index);
}

pub fn shuffle_queue(gem_player: &mut GemPlayer) {
    let mut rng = rand::thread_rng();
    gem_player.queue.shuffle(&mut rng);
}

pub fn clear_queue(gem_player: &mut GemPlayer) {
    gem_player.queue.clear();
}

pub fn move_song_up_in_queue(gem_player: &mut GemPlayer, index: usize) {
    if index == 0 {
        return;
    }

    gem_player.queue.swap(index, index - 1);
}

pub fn move_song_down_in_queue(gem_player: &mut GemPlayer, index: usize) {
    if index == gem_player.queue.len() - 1 {
        return;
    }

    gem_player.queue.swap(index, index + 1);
}

pub fn begin_playlist(gem_player: &mut GemPlayer, playlist: &Playlist) {
    gem_player.queue = playlist.songs.clone();
    play_next_song_in_queue(gem_player);
}
