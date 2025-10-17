use handlebars::Handlebars;
use serde::Serialize;
use std::{
    env,
    fs::{create_dir_all, read_to_string, File},
    io::Write,
    path::PathBuf,
};

fn main() -> Result<(), ()> {
    let is_windows = cfg!(target_os = "windows");
    let profile = env::var("PROFILE").unwrap_or_default();
    let is_release = profile == "release";

    if is_windows && is_release {
        let mut res = winres::WindowsResource::new();
        res.set_icon("assets/icon.ico");
        res.set_manifest_file("manifest.xml");
        if let Err(e) = res.compile() {
            println!("Could not package resources for Windows: {}", e);
            return Err(());
        }

        if let Err(e) = generate_inno_script() {
            println!("Could not generate the inno setup script: {}", e);
            return Err(());
        }
    }

    Ok(())
}

#[derive(Serialize)]
struct InnoSetupScriptData {
    version: String,
    installer_dir: String,
    exe_path: String,
}

fn generate_inno_script() -> Result<(), String> {
    let version = env::var("CARGO_PKG_VERSION").map_err(|e| e.to_string())?;
    let target = env::var("CARGO_BUILD_TARGET").unwrap_or_else(|_| "x86_64-pc-windows-gnu".into());
    let profile = env::var("PROFILE").unwrap_or_else(|_| "release".into());

    let out_dir: PathBuf = ["target", &target, &profile].iter().collect();
    let installer_dir = out_dir.join("installer");
    create_dir_all(&installer_dir).map_err(|e| e.to_string())?;

    let out_dir = out_dir.canonicalize().map_err(|e| e.to_string())?;
    let installer_dir = installer_dir.canonicalize().map_err(|e| e.to_string())?;

    let template_path = PathBuf::from("windows_installer.iss.hbs")
        .canonicalize()
        .map_err(|e| e.to_string())?;
    let template = read_to_string(&template_path).map_err(|e| e.to_string())?;

    let mut handlebars = Handlebars::new();
    handlebars
        .register_template_string("installer", &template)
        .map_err(|e| e.to_string())?;

    let exe_path = out_dir.join("gem-player.exe").canonicalize().map_err(|e| e.to_string())?;

    let data = InnoSetupScriptData {
        version,
        installer_dir: installer_dir.display().to_string(),
        exe_path: exe_path.display().to_string(),
    };

    let rendered = handlebars.render("installer", &data).map_err(|e| e.to_string())?;

    let output_path = installer_dir.join("windows_installer.iss");
    let mut file = File::create(&output_path).map_err(|e| e.to_string())?;
    file.write_all(rendered.as_bytes()).map_err(|e| e.to_string())?;

    println!("✅ Generated Inno Setup script at {}", output_path.display());

    Ok(())
}
