// row.col(|ui| {
// 	let cell_rect = ui.max_rect();

// 	if this_playlists_name_is_being_edited {
// 		containers::Sides::new().height(cell_rect.height()).show(
// 			ui,
// 			|ui| {
// 				ui.add_space(8.0);
// 				let text_edit =
// 					TextEdit::singleline(&mut gem_player.edit_playlist_name_buffer).desired_width(100.0);
// 				let response = ui.add(text_edit);

// 				if response.lost_focus() {
// 					print_info(format!("Renamed playlist to: {}", gem_player.edit_playlist_name_buffer));
// 					gem_player.edit_playlist_name_id = None;
// 				}
// 			},
// 			|_ui| {},
// 		);
// 	} else {
// 		containers::Sides::new().height(cell_rect.height()).show(
// 			ui,
// 			|ui| {
// 				ui.add_space(8.0);
// 				ui.add(unselectable_label(&playlist.name));
// 			},
// 			|ui| {
// 				if !ui.rect_contains_pointer(cell_rect) {
// 					return;
// 				}

// 				ui.add_space(16.0); // Add space to the right of the buttons to avoid the scrollbar.

// 				let delete_button = Button::new(icons::ICON_DELETE);
// 				let response = ui.add(delete_button).on_hover_text("Delete");
// 				if response.clicked() {
// 					gem_player.confirm_delete_playlist_modal_is_open = true;
// 				}

// 				let edit_name_button = Button::new(icons::ICON_EDIT);
// 				if ui.add(edit_name_button).on_hover_text("Edit name").clicked() {
// 					gem_player.edit_playlist_name_id = Some(playlist.id);
// 					gem_player.edit_playlist_name_buffer = playlist.name.clone();
// 				}
// 			},
// 		);
// 	}
// });

// ui.painter().line_segment(
//     [
//         control_ui_rect.left_bottom() + vec2(1.0, 0.0),
//         control_ui_rect.right_bottom(),
//     ],
//     ui.visuals().widgets.noninteractive.bg_stroke,
// );

// pub fn start_library_watcher(library_folder: PathBuf, library_is_dirty_flag: Arc<AtomicBool>) -> Result<RecommendedWatcher, String> {
//     let (tx, rx) = mpsc::channel::<notify::Result<Event>>();

//     let mut watcher = match recommended_watcher(tx) {
//         Ok(w) => w,
//         Err(e) => {
//             return Err(format!("Failed to create watcher: {:?}", e));
//         }
//     };

//     if let Err(e) = watcher.watch(&library_folder, RecursiveMode::Recursive) {
//         return Err(format!("Failed to watch folder: {:?}", e));
//     }

//     thread::spawn(move || {
//         for res in rx {
//             match res {
//                 Ok(event) => {
//                     println!("File event detected: {:?}", event);
//                     let is_relevant_event = event.kind.is_create() || event.kind.is_remove() || event.kind.is_modify();
//                     if is_relevant_event {
//                         library_is_dirty_flag.store(true, Ordering::SeqCst);
//                     }
//                 }
//                 Err(e) => eprintln!("Watch error: {:?}", e),
//             }
//         }
//     });

//     Ok(watcher)
// }

// pub fn update_watched_directory(watcher: &mut RecommendedWatcher, old_path: &Path, new_path: &Path) { // Could just start up a new watcher instead of updating the old one.
//     if let Err(e) = watcher.unwatch(old_path) {
//         eprintln!("Failed to unwatch old folder: {:?}", e);
//     }

//     if let Err(e) = watcher.watch(new_path, RecursiveMode::Recursive) {
//         eprintln!("Failed to watch new folder: {:?}", e);
//     }

//     println!("Updated library folder to {:?}", new_path);
// }
	// row.col(|ui| {
	//     let has_artwork = song.artwork.is_some();
	//     if has_artwork {
	//         let uri = format!("bytes://{}", song.artwork.clone().unwrap().len());
	//         let image = egui::Image::from_bytes(uri, song.artwork.clone().unwrap())
	//             .fit_to_exact_size(egui::vec2(48.0, 48.0))
	//             .rounding(4.0);
	//         ui.add(image);
	//     } else {
	//         ui.label("No Artwork");
	//     }
	// });
	
	// ui.heading("My egui Music App");
	
	// ui.horizontal(|ui| {
	//     let name_label = ui.label("Your name: ");
	//     ui.text_edit_singleline(&mut self.name)
	//         .labelled_by(name_label.id);
	// });
	// ui.add(egui::Slider::new(&mut self.age, 0..=120).text("age"));
	// if ui.button("Increment").clicked() {
	//     self.age += 1;
	// }
	// ui.label(format!("Hello '{}', age {}", self.name, self.age));
	
	// ui.image(egui:


	// egui::include_image!(
	//     "../assets/pause_24dp_E8EAED_FILL0_wght400_GRAD0_opsz24.svg"
	// )

	// egui::include_image!(
	//     "../assets/play_arrow_24dp_E8EAED_FILL0_wght400_GRAD0_opsz24.svg"
	// )

	// let volume_icon = match volume {
	//     v if v == 0.0 => egui::include_image!(
	//         "../assets/volume_mute_24dp_E8EAED_FILL0_wght400_GRAD0_opsz24.svg"
	//     ),
	//     v if v < 0.5 => egui::include_image!(
	//         "../assets/volume_down_24dp_E8EAED_FILL0_wght400_GRAD0_opsz24.svg"
	//     ),
	//     _ => egui::include_image!(
	//         "../assets/volume_up_24dp_E8EAED_FILL0_wght400_GRAD0_opsz24.svg"
	//     ),
	// };


	/*let mut fonts = egui::FontDefinitions::default();
	fonts.font_data.insert(
		"my_font".to_owned(),
		egui::FontData::from_static(include_bytes!(
			"../assets/Inconsolata-VariableFont_wdth,wght.ttf"
		)),
	);
	fonts
		.families
		.get_mut(&egui::FontFamily::Proportional)
		.unwrap()
		.insert(0, "my_font".to_owned());
	fonts
		.families
		.get_mut(&egui::FontFamily::Monospace)
		.unwrap()
		.push("my_font".to_owned());
	cc.egui_ctx.set_fonts(fonts);*/
