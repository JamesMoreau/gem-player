use crate::{sort_songs, ui, Playlist, Song, SortBy, SortOrder, Theme};
use glob::glob;
use lofty::{
    file::{AudioFile, TaggedFileExt},
    tag::ItemKey,
};
use notify::{recommended_watcher, Event, RecursiveMode, Watcher};
use rand::seq::SliceRandom;
use rodio::{Decoder, OutputStream, Sink};
use std::{
    io::BufReader,
    path::{Path, PathBuf}, sync::mpsc, thread,
};

pub const SUPPORTED_AUDIO_FILE_TYPES: [&str; 6] = ["mp3", "m4a", "wav", "flac", "ogg", "opus"];

pub struct GemPlayer {
    pub current_view: ui::View,
    pub theme: Theme,
    pub search_text: String,
    pub sort_by: SortBy,
    pub sort_order: SortOrder,

    pub library: Vec<Song>, // All the songs stored in the music directory.
    pub queue: Vec<Song>,
    pub history: Vec<Song>,
    pub current_song: Option<Song>,

    pub selected_song: Option<Song>, // Currently selected song in the songs vector. TODO: multiple selection.
    pub repeat: bool,
    pub muted: bool,
    pub volume_before_mute: Option<f32>,
    pub paused_before_scrubbing: Option<bool>, // None if not scrubbing, Some(true) if paused, Some(false) if playing.

    pub _stream: OutputStream, // Holds the OutputStream to keep it alive
    pub sink: Sink,            // Controls playback (play, pause, stop, etc.)

    pub library_directory: Option<PathBuf>, // The directory where music is stored.
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
            theme: Theme::System,
            search_text: String::new(),
            sort_by: SortBy::Title,
            sort_order: SortOrder::Ascending,

            library: Vec::new(),
            queue: Vec::new(),
            history: Vec::new(),
            current_song: None,

            selected_song: None,
            repeat: false,
            muted: false,
            volume_before_mute: None,
            paused_before_scrubbing: None,

            _stream,
            sink,

            library_directory: None,
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
        default_self.library_directory = Some(my_music_directory.clone());

        let songs = match &default_self.library_directory {
            Some(path) => {
                let result = read_music_from_a_directory(path);
                match result {
                    Ok(songs) => songs,
                    Err(e) => {
                        println!("{}", e);
                        Vec::new()
                    }
                }
            }
            None => Vec::new(),
        };
        println!("Found {} songs", &songs.len());

        watch_music_folder(my_music_directory.to_path_buf());

        Self {
            library: songs,
            ..default_self
        }
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

pub fn play_next(gem_player: &mut GemPlayer) {
    if gem_player.repeat {
        if let Some(current_song) = &gem_player.current_song {
            let result = load_and_play_song(gem_player, &current_song.clone());
            if let Err(e) = result {
                println!("{}", e);
            }
        }

        return;
    }

    let next_song = if gem_player.queue.is_empty() {
        return;
    } else {
        gem_player.queue.remove(0)
    };

    let maybe_current_song = gem_player.current_song.take();
    if let Some(current_song) = maybe_current_song {
        gem_player.history.push(current_song);
    }

    gem_player.current_song = Some(next_song.clone());
    let result = load_and_play_song(gem_player, &next_song);
    if let Err(e) = result {
        println!("{}", e);
    }
}

pub fn play_previous(gem_player: &mut GemPlayer) {
    let previous_song = if gem_player.history.is_empty() {
        return;
    } else {
        gem_player.history.pop().unwrap()
    };

    let maybe_current_song = gem_player.current_song.take();
    if let Some(current_song) = maybe_current_song {
        gem_player.queue.insert(0, current_song);
    }

    gem_player.current_song = Some(previous_song.clone());
    let result = load_and_play_song(gem_player, &previous_song);
    if let Err(e) = result {
        println!("{}", e);
    }
}

// TODO: Is this ok to call this function from the UI thread since we are doing heavy events like loading a file?
pub fn load_and_play_song(gem_player: &mut GemPlayer, song: &Song) -> Result<(), String> {
    gem_player.sink.stop(); // Stop the current song if any.
    gem_player.current_song = None;

    let file_result = std::fs::File::open(&song.file_path);
    let file = match file_result {
        Ok(file) => file,
        Err(e) => {
            return Err(format!("Error opening file: {:?}", e));
        }
    };

    let source_result = Decoder::new(BufReader::new(file));
    let source = match source_result {
        Ok(source) => source,
        Err(e) => {
            return Err(format!("Error decoding file: {:?}", e));
        }
    };

    gem_player.current_song = Some(song.clone());
    gem_player.sink.append(source);
    gem_player.sink.play();

    Ok(())
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

pub fn read_music_from_a_directory(path: &Path) -> Result<Vec<Song>, String> {
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

pub fn get_song_position_in_queue(gem_player: &GemPlayer, song: &Song) -> Option<usize> {
    gem_player.queue.iter().position(|s| *s == *song)
}

pub fn add_to_queue(gem_player: &mut GemPlayer, song: Song) {
    gem_player.queue.push(song);
}

pub fn add_next_to_queue(gem_player: &mut GemPlayer, song: Song) {
    gem_player.queue.insert(0, song);
}

pub fn remove_from_queue(gem_player: &mut GemPlayer, index: usize) {
    gem_player.queue.remove(index);
}

pub fn shuffle_queue(gem_player: &mut GemPlayer) {
    let mut rng = rand::thread_rng();
    gem_player.queue.shuffle(&mut rng);
}

pub fn move_song_to_front(gem_player: &mut GemPlayer, index: usize) {
    if index == 0 || index >= gem_player.queue.len() {
        return;
    }

    let song = gem_player.queue.remove(index);
    gem_player.queue.insert(0, song);
}

pub fn play_library_from_song(gem_player: &mut GemPlayer, song: &Song) {
    gem_player.queue.clear();

    let maybe_song_index = gem_player.library.iter().position(|s| s == song);
    match maybe_song_index {
        None => {
            println!("Song not found in the library.");
        }
        Some(index) => {
            gem_player.queue.extend_from_slice(&gem_player.library[index + 1..]);
            gem_player.queue.extend_from_slice(&gem_player.library[..index]);

            let result = load_and_play_song(gem_player, song);
            if let Err(e) = result {
                println!("{}", e);
            }
        }
    }
}

fn watch_music_folder(music_folder: PathBuf) {
    thread::spawn(move || {
        let (tx, rx) = mpsc::channel::<notify::Result<Event>>();

        let mut watcher = match recommended_watcher(tx) {
            Ok(w) => w,
            Err(e) => {
                eprintln!("Failed to create file watcher: {:?}", e);
                return;
            }
        };

        if let Err(e) = watcher.watch(&music_folder, RecursiveMode::Recursive) {
            eprintln!("Failed to watch folder: {:?}", e);
            return;
        }

        println!("Watching music folder: {:?}", music_folder);

        for res in rx {
            match res {
                Ok(event) => {
                    println!("File event detected: {:?}", event);
                    
                    let is_event_we_care_about = event.kind.is_create() || event.kind.is_remove() || event.kind.is_modify();
                    if is_event_we_care_about {
                        // let mut player = gem_player.lock().unwrap();
                        match read_music_from_a_directory(&music_folder) {
                            Ok(new_songs) => {
                                // player.library = new_songs;
                                println!("Library updated successfully!");
                            }
                            Err(e) => eprintln!("Failed to update library: {}", e),
                        }
                    }
                },
                Err(e) => eprintln!("Watch error: {:?}", e),
            }
        }
    });
}
