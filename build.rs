fn main() -> Result<(), ()> {
    #[cfg(target_os = "windows")]
    windows::build()?;

    Ok(())
}

#[cfg(target_os = "windows")]
mod windows {
    use std::{fs::File, path::Path};
    use ico::{IconDir, IconDirEntry, IconImage, ResourceType};

    pub fn build() -> Result<(), ()> {
        let png_path = Path::new("assets/icon.png");
        let ico_path = Path::new("assets/icon.ico");

        if let Err(e) = convert_png_to_ico(png_path, ico_path) {
            eprintln!("⚠️ Failed to convert PNG to ICO: {e}");
            return Err(());
        }

        let mut res = winres::WindowsResource::new();
        res.set_icon(ico_path.to_str().unwrap());
        res.set_manifest_file("platform/windows/manifest.xml");

        if let Err(e) = res.compile() {
            eprintln!("⚠️ Failed to package Windows resources: {e}");
            return Err(());
        }

        Ok(())
    }

    fn convert_png_to_ico(src: &Path, dst: &Path) -> Result<(), String> {
        let img = image::open(src).map_err(|e| e.to_string())?;
        let rgba = img.to_rgba8();

        let mut icon_dir = IconDir::new(ResourceType::Icon);
        let entry = IconImage::from_rgba_data(img.width(), img.height(), rgba.into_raw());
        icon_dir.add_entry(IconDirEntry::encode(&entry).map_err(|e| e.to_string())?);

        let mut file = File::create(dst).map_err(|e| e.to_string())?;
        icon_dir.write(&mut file).map_err(|e| e.to_string())
    }
}