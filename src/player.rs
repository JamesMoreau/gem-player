use crate::{
    print_error, print_info,
    ui::{self, EditSongMetadaUIState, PlaylistsUIState, UIState},
    Playlist, Song, SortBy, SortOrder, Theme,
};
use eframe::egui::{Context, Event, Key};
use egui_notify::Toasts;
use fully_pub::fully_pub;
use glob::glob;
use indexmap::IndexMap;
use lazy_static::lazy_static;
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
use uuid::Uuid;

pub const SUPPORTED_AUDIO_FILE_TYPES: [&str; 6] = ["mp3", "m4a", "wav", "flac", "ogg", "opus"];

#[fully_pub]
pub struct GemPlayer {
    ui_state: UIState,

    library: Vec<Song>,                 // All the songs stored in the user's music directory.
    library_directory: Option<PathBuf>, // The directory where music is stored.
    playlists: Vec<Playlist>,

    player: Player,
}

#[fully_pub]
pub struct Player {
    current_song: Option<Song>,

    queue: Vec<Song>,
    history: Vec<Song>,

    repeat: bool,
    muted: bool,
    volume_before_mute: Option<f32>,
    paused_before_scrubbing: Option<bool>, // None if not scrubbing, Some(true) if paused, Some(false) if playing.

    _stream: OutputStream, // Holds the OutputStream to keep it alive
    sink: Sink,            // Controls playback (play, pause, stop, etc.)
}

impl GemPlayer {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        egui_extras::install_image_loaders(&cc.egui_ctx);

        egui_material_icons::initialize(&cc.egui_ctx);

        let (_stream, handle) = OutputStream::try_default().unwrap();
        let sink = Sink::try_new(&handle).unwrap();
        sink.pause();
        sink.set_volume(0.6);

        let library_directory = dirs::audio_dir().map(|dir| dir.join("MyMusic"));

        let library = match &library_directory {
            Some(path) => {
                let result = read_music_from_a_directory(path);
                match result {
                    Ok(songs) => songs,
                    Err(e) => {
                        print_error(e);
                        Vec::new()
                    }
                }
            }
            None => Vec::new(),
        };
        print_info(format!("Found {} songs", library.len()));

        Self {
            ui_state: UIState {
                current_view: ui::View::Library,
                theme: Theme::System,
                search_text: String::new(),
                selected_library_song: None,
                sort_by: SortBy::Title,
                sort_order: SortOrder::Ascending,
                playlists_ui_state: PlaylistsUIState {
                    selected_playlist_index: None,
                    edit_playlist_name_info: None,
                    confirm_delete_playlist_modal_is_open: false,
                },
                _edit_song_metadata_ui_state: EditSongMetadaUIState {
                    _buffer_song: None,
                    _edit_song_metadata_modal_is_open: false,
                },
                toasts: Toasts::default(),
            },

            library,
            library_directory,
            playlists: Vec::new(),

            player: Player {
                current_song: None,

                queue: Vec::new(),
                history: Vec::new(),

                repeat: false,
                muted: false,
                volume_before_mute: None,
                paused_before_scrubbing: None,

                _stream,
                sink,
            },
        }
    }
}

pub fn is_playing(player: &mut Player) -> bool {
    !player.sink.is_paused()
}

pub fn play_or_pause(player: &mut Player) {
    if player.sink.is_paused() {
        player.sink.play()
    } else {
        player.sink.pause()
    }
}

pub fn play_next(player: &mut Player) -> Result<(), String> {
    if player.repeat {
        if let Some(current_song) = &player.current_song {
            let song = current_song.clone();
            let result = load_and_play_song(player, &song);
            return result;
        }
        return Ok(()); // If we are in repeat mode but there is no current song, do nothing!
    }

    if player.queue.is_empty() {
        return Ok(());
    }
    let next_song = player.queue.remove(0);

    if let Some(current_song) = player.current_song.take() {
        player.history.push(current_song);
    }

    player.current_song = Some(next_song.clone());
    load_and_play_song(player, &next_song)
}

pub fn play_previous(player: &mut Player) -> Result<(), String> {
    let Some(previous_song) = player.history.pop() else {
        return Ok(());
    };

    if let Some(maybe_current_song) = player.current_song.take() {
        player.queue.insert(0, maybe_current_song);
    }

    player.current_song = Some(previous_song.clone());

    load_and_play_song(player, &previous_song)
}

// TODO: Is this ok to call this function from the UI thread since we are doing heavy events like loading a file?
pub fn load_and_play_song(player: &mut Player, song: &Song) -> Result<(), String> {
    player.sink.stop(); // Stop the current song if any.
    player.current_song = None;

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

    player.current_song = Some(song.clone());
    player.sink.append(source);
    player.sink.play();

    Ok(())
}

pub fn get_song_from_file(path: &Path) -> Result<Song, String> {
    if !path.is_file() {
        return Err("Path is not a file".to_string());
    }

    let result_file = lofty::read_from_path(path);
    let tagged_file = match result_file {
        Ok(file) => file,
        Err(e) => {
            return Err(format!("Error reading file: {}", e));
        }
    };

    let tag = match tagged_file.primary_tag() {
        Some(tag) => tag,
        None => match tagged_file.first_tag() {
            Some(tag) => tag,
            None => return Err(format!("No tags found in file: {:?}", path)),
        },
    };

    let id = Uuid::new_v4();

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

    Ok(Song {
        id,
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
        let result = get_song_from_file(&entry);
        match result {
            Ok(song) => songs.push(song),
            Err(e) => print_error(e.to_string()),
        }
    }

    Ok(songs)
}

pub fn _get_song_position_in_queue(queue: Vec<Song>, song: &Song) -> Option<usize> {
    queue.iter().position(|s| s.id == song.id)
}

pub fn add_to_queue(queue: &mut Vec<Song>, song: Song) {
    queue.push(song);
}

pub fn add_next_to_queue(queue: &mut Vec<Song>, song: Song) {
    queue.insert(0, song);
}

pub fn remove_from_queue(queue: &mut Vec<Song>, index: usize) {
    queue.remove(index);
}

pub fn shuffle_queue(queue: &mut Vec<Song>) {
    let mut rng = rand::rng();
    queue.shuffle(&mut rng);
}

pub fn move_song_to_front(queue: &mut Vec<Song>, index: usize) {
    if index == 0 || index >= queue.len() {
        return;
    }

    let song = queue.remove(index);
    queue.insert(0, song);
}

pub fn mute_or_unmute(player: &mut Player) {
    let mut volume = player.sink.volume();

    player.muted = !player.muted;

    if player.muted {
        player.volume_before_mute = Some(volume);
        volume = 0.0;
    } else if let Some(v) = player.volume_before_mute {
        volume = v;
    }

    player.sink.set_volume(volume);
}

pub fn adjust_volume_by_percentage(player: &mut Player, percentage: f32) {
    let current_volume = player.sink.volume();
    let new_volume = (current_volume + percentage).clamp(0.0, 1.0);
    player.sink.set_volume(new_volume);
}

pub fn play_library_from_song(gem_player: &mut GemPlayer, song: &Song) {
    gem_player.player.queue.clear();

    let maybe_song_index = gem_player.library.iter().position(|s| s.id == song.id);
    match maybe_song_index {
        None => {
            print_error("Song not found in the library.");
        }
        Some(index) => {
            gem_player.player.queue.extend_from_slice(&gem_player.library[index + 1..]);
            gem_player.player.queue.extend_from_slice(&gem_player.library[..index]);

            let result = load_and_play_song(&mut gem_player.player, song);
            if let Err(e) = result {
                print_error(e.to_string());
                gem_player
                    .ui_state
                    .toasts
                    .error(format!("Error playing {}", song.title.as_deref().unwrap_or("Unknown")));
            }
        }
    }
}

pub struct KeyBinding {
    pub name: &'static str,
    pub action: fn(&mut GemPlayer),
}

lazy_static! {
    pub static ref KEYMAP: IndexMap<Key, KeyBinding> = {
        let mut map = IndexMap::new();

        // Insert key bindings in the desired order.
        map.insert(
            Key::Space,
            KeyBinding {
                name: "Play/Pause",
                action: |gp| play_or_pause(&mut gp.player),
            },
        );
        map.insert(
            Key::ArrowRight,
            KeyBinding {
                name: "Next",
                action: |gp| {
                    if let Err(e) = play_next(&mut gp.player) {
                        print_error(e);
                        gp.ui_state.toasts.error("Error playing the next song");
                    }
                },
            },
        );
        map.insert(
            Key::ArrowLeft,
            KeyBinding {
                name: "Previous",
                action: |gp| {
                    if let Err(e) = play_previous(&mut gp.player) {
                        print_error(e);
                        gp.ui_state.toasts.error("Error playing the previous song");
                    }
                },
            },
        );
        map.insert(
            Key::ArrowUp,
            KeyBinding {
                name: "Volume Up",
                action: |gp| adjust_volume_by_percentage(&mut gp.player, 0.1),
            },
        );
        map.insert(
            Key::ArrowDown,
            KeyBinding {
                name: "Volume Down",
                action: |gp| adjust_volume_by_percentage(&mut gp.player, -0.1),
            },
        );
        map.insert(
            Key::M,
            KeyBinding {
                name: "Mute/Unmute",
                action: |gp| mute_or_unmute(&mut gp.player),
            },
        );
        map
    };
}

pub fn handle_input(ctx: &Context, gem_player: &mut GemPlayer) {
    ctx.input(|i| {
        for event in &i.events {
            if let Event::Key {
                key,
                pressed: true,
                physical_key: _,
                repeat: _,
                modifiers: _,
            } = event
            {
                let Some(binding) = KEYMAP.get(key) else {
                    continue;
                };

                print_info(format!("Key pressed: {}", binding.name));

                (binding.action)(gem_player); // Call the action associated with the key binding.
            }
        }
    });
}

pub fn _add_songs_to_playlist(playlist: &mut Playlist, songs: Vec<Song>) {
    playlist.songs.extend(songs);
}

fn _load_playlist_from_m3u(_path: &Path) -> Result<Playlist, String> {
    todo!()
}
