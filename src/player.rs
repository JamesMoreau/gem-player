use std::{io::BufReader, path::{Path, PathBuf}};
use glob::glob;

use rodio::{Decoder, OutputStream, Sink};

use crate::{models::{get_song_from_file, sort_songs, Song, SortBy, SortOrder}, ui};

pub const SUPPORTED_AUDIO_FILE_TYPES: [&str; 6] = ["mp3", "m4a", "wav", "flac", "ogg", "opus"];
pub struct GemPlayer {
    pub current_view: ui::View,
    pub songs: Vec<Song>,

    pub music_directory: Option<PathBuf>,

    pub selected_song: Option<usize>, // Index of the selected song in the songs vector.
    pub queue: Vec<Song>,
    pub current_song: Option<Song>, // The currently playing song.
    pub _stream: OutputStream,      // Holds the OutputStream to keep it alive
    pub sink: Sink,                 // Controls playback (play, pause, stop, etc.)
    
    pub muted: bool,
    pub volume_before_mute: Option<f32>,

    pub paused_before_scrubbing: Option<bool>, // None if not scrubbing, Some(true) if paused, Some(false) if playing.

    pub search_text: String,
    pub sort_by: SortBy,
    pub sort_order: SortOrder,

    pub theme: String,
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
            songs: Vec::new(),
            selected_song: None,
            queue: Vec::new(),
            search_text: String::new(),
            music_directory: None,
            sort_by: SortBy::Title,
            sort_order: SortOrder::Ascending,
            current_song: None,
            // queue: Vec::new(),
            _stream,
            sink,
            muted: false,
            volume_before_mute: None,
            paused_before_scrubbing: None,
            theme: "Default".to_owned(),
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
            Some(path) => read_music_from_directory(path),
            None => Vec::new(),
        };
        println!("Found {} songs", &songs.len());
        sort_songs(
            &mut default_self.songs,
            default_self.sort_by,
            default_self.sort_order,
        );

        Self {
            songs,
            ..default_self
        }
    }

    pub fn is_playing(&self) -> bool {
        !self.sink.is_paused()
    }

    // TODO: Is this ok to call this function from the UI thread since we are doing heavy events like loading a file?
    pub fn load_and_play_song(&mut self, song: &Song) {
        self.sink.stop(); // Stop the current song if any.

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
                println!(
                    "Error decoding file: {}, Error: {:?}",
                    song.file_path.to_string_lossy(),
                    e
                );
                return;
            }
        };

        self.current_song = Some(song.clone());

        self.sink.append(source);
        self.sink.play();
    }

    pub fn play_or_pause(&mut self) {
        if self.sink.is_paused() {
            self.sink.play()
        } else {
            self.sink.pause()
        }
    }
}

pub fn read_music_from_directory(path: &Path) -> Vec<Song> {
    let mut songs = Vec::new();
    let mut file_paths: Vec<PathBuf> = Vec::new();

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
                println!("Error reading pattern {}: {}", pattern, e);
            }
        }
    }

    if file_paths.is_empty() {
        println!("No music files found in directory: {:?}", path);
        return songs;
    }

    for entry in file_paths {
        let song_option = get_song_from_file(&entry);
        let song = match song_option {
            Some(song) => song,
            None => {
                println!("Error reading song from file: {:?}", entry);
                continue;
            }
        };
        songs.push(song);
    }

    songs
}
