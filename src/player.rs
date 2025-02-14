use std::{io::BufReader, path::PathBuf};

use eframe::egui::{Context, Event, Key};
use egui_notify::Toasts;
use fully_pub::fully_pub;
use indexmap::IndexMap;
use lazy_static::lazy_static;
use log::{error, info};
use rand::seq::SliceRandom;
use rodio::{Decoder, OutputStream, Sink};

use crate::{
    playlist::{read_playlists_from_a_directory, Playlist},
    song::{read_music_from_a_directory, Song, SortBy, SortOrder},
    ui::{self, EditSongMetadaUIState, PlaylistsUIState, UIState},
    Theme,
};

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

pub fn init_gem_player(cc: &eframe::CreationContext<'_>) -> GemPlayer {
    egui_extras::install_image_loaders(&cc.egui_ctx);

    egui_material_icons::initialize(&cc.egui_ctx);

    let (_stream, handle) = OutputStream::try_default().unwrap();
    let sink = Sink::try_new(&handle).unwrap();
    sink.pause();
    sink.set_volume(0.6);

    let library_directory = dirs::audio_dir().map(|dir| dir.join("MyMusic"));

    let mut library = Vec::new();
    let mut playlists = Vec::new();
    if let Some(directory) = &library_directory {
        let result = read_music_from_a_directory(directory);
        match result {
            Ok(found_songs) => {
                library.extend(found_songs);
            }
            Err(e) => {
                error!("{}", e);
            }
        }

        let result = read_playlists_from_a_directory(directory);
        match result {
            Ok(found_playlists) => {
                playlists.extend(found_playlists);
            }
            Err(e) => {
                error!("{}", e);
            }
        }
    }
    info!("Found {} songs", library.len());

    GemPlayer {
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
        playlists,

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
            error!("Song not found in the library.");
        }
        Some(index) => {
            gem_player.player.queue.extend_from_slice(&gem_player.library[index + 1..]);
            gem_player.player.queue.extend_from_slice(&gem_player.library[..index]);

            let result = load_and_play_song(&mut gem_player.player, song);
            if let Err(e) = result {
                error!("{}", e);
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
    pub static ref KEY_COMMANDS: IndexMap<Key, KeyBinding> = {
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
                        error!("{}", e);
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
                        error!("{}", e);
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

pub fn handle_key_commands(ctx: &Context, gem_player: &mut GemPlayer) {
    if ctx.wants_keyboard_input() {
        // Return early if any widget that accepts keyboard input is focused.
        return;
    }

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
                let Some(binding) = KEY_COMMANDS.get(key) else {
                    continue;
                };

                info!("Key pressed: {}", binding.name);

                (binding.action)(gem_player); // Call the action associated with the key binding.
            }
        }
    });
}
