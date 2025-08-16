// ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
// 	let leading_space = 0.0;
// 	let style = ui.style();
// 	let text_color = ui.visuals().text_color();
// 	let divider_color = ui.visuals().weak_text_color();

// 	let get_text_format =
// 		|style: &Style, color: Color32| TextFormat::simple(TextStyle::Body.resolve(style), color);

// 	let mut job = text::LayoutJob::default();
// 	job.append(title, leading_space, get_text_format(style, text_color));
// 	job.append(" / ", leading_space, get_text_format(style, divider_color));
// 	job.append(artist, leading_space, get_text_format(style, text_color));
// 	job.append(" / ", leading_space, get_text_format(style, divider_color));
// 	job.append(album, leading_space, get_text_format(style, text_color));

// 	let track_label = Label::new(job).selectable(false).truncate();
// 	ui.add(track_label);
// });

//.fill(ui.visuals().faint_bg_color)

// ui.style_mut().text_styles.insert(TextStyle::Button, FontId::new(18.0, FontFamily::Proportional));
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

/*let nyquist_bin = buffer.len() / 2;
let mut max_log_amplitudes = Vec::new();
let mut global_max_log_amplitude = 1.0_f32;
let band_growth_factor = 1.06_f32;
let mut current_band_start_bin = 1.0_f32;

while (current_band_start_bin as usize) < nyquist_bin {
	// Compute the end of this logarithmic band
	let next_band_start_bin = (current_band_start_bin * band_growth_factor).ceil();
	let start_bin_index = current_band_start_bin as usize;
	let end_bin_index = next_band_start_bin.min(nyquist_bin as f32) as usize;

	// Find the max log amplitude in this band
	let mut band_max_log_amplitude = f32::NEG_INFINITY;
	for c in &buffer[start_bin_index..end_bin_index] {
		let log_power = (c.re * c.re + c.im * c.im + 1e-12).ln();
		if log_power > band_max_log_amplitude {
			band_max_log_amplitude = log_power;
		}
	}

	if band_max_log_amplitude > global_max_log_amplitude {
		global_max_log_amplitude = band_max_log_amplitude;
	}

	max_log_amplitudes.push(band_max_log_amplitude);
	current_band_start_bin = next_band_start_bin;
}*/

/*
pub fn hann_window(n: usize) -> Vec<f32> {
    if n == 0 {
        return Vec::new();
    }

    let mut window = Vec::with_capacity(n);

    for i in 0..n {
        let multiplier = 0.5 - 0.5 * ((2.0 * PI * i as f32) / (n - 1) as f32).cos();
        window.push(multiplier);
    }

    window
 */
