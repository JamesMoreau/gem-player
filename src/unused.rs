// ceate_async_watcher() -> notify::Result<(RecommendedWatcher, Receiver<notify::Result<Event>>)> {
	//     let (mut tx, rx) = channel(1);
	//     let watcher = RecommendedWatcher::new(
	//         move |res| {
	//             block_on(async {
	//                 tx.send(res).await.unwrap();
	//             })
	//         },
	//         Config::default(),
	//     )?;
	//     Ok((watcher, rx))
	// }
	
	// async fn start_watching(path: PathBuf) -> notify::Result<()> {
	//     let (mut watcher, mut rx) = create_async_watcher()?;
	
	//     watcher.watch(&path, RecursiveMode::Recursive)?;
	
	//     while let Some(res) = rx.next().await {
	//         match res {
	//             Ok(event) => {
	//                 println!("File changed: {:?}", event);
	//                 // Logic to update your song list when changes occur
	//             }
	//             Err(e) => println!("Watch error: {:?}", e),
	//         }
	//     }
	
	//     Ok(())
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