#[cfg(target_os = "windows")]
use handlebars::Handlebars;
#[cfg(target_os = "windows")]
use serde::Serialize;
#[cfg(target_os = "windows")]
use std::{
    env,
    fs::{create_dir_all, read_to_string, File},
    io::Write,
    path::PathBuf,
};

fn main() -> Result<(), ()> {
    #[cfg(all(target_os = "windows", not(debug_assertions)))]
    {
        let mut res = winres::WindowsResource::new();
        res.set_icon("assets/icon.ico");
        res.set_manifest_file("manifest.xml");
        let compile_result = res.compile();
        if let Err(e) = compile_result {
            eprintln!("⚠️  Failed to package Windows resources: {e}");
            return Err(());
        }

        if let Err(e) = generate_inno_setup_script() {
            eprintln!("⚠️  Failed to generate Inno Setup script: {e}");
            return Err(());
        }
    }

    Ok(())
}

#[cfg(target_os = "windows")]
#[derive(Serialize)]
struct InnoSetupScriptData {
    version: String,
    output_name: String,
    installer_dir: String,
    exe_path: String,
}

#[cfg(target_os = "windows")]
fn generate_inno_setup_script() -> Result<(), String> {
    let version = env::var("CARGO_PKG_VERSION").map_err(|e| e.to_string())?;
    let target = env::var("CARGO_BUILD_TARGET").unwrap_or_else(|_| "x86_64-pc-windows-gnu".into());
    let profile = env::var("PROFILE").unwrap_or_else(|_| "release".into());

    let out_dir: PathBuf = ["target", &target, &profile].iter().collect();
    let installer_dir = out_dir.join("installer");
    create_dir_all(&installer_dir).map_err(|e| e.to_string())?;

    let template_path = PathBuf::from("windows_installer.iss.hbs")
        .canonicalize()
        .map_err(|e| e.to_string())?;
    let template = read_to_string(&template_path).map_err(|e| e.to_string())?;

    let mut handlebars = Handlebars::new();
    handlebars
        .register_template_string("installer", &template)
        .map_err(|e| e.to_string())?;

    // Keep paths relative or absolute but do not canonicalize to avoid \\?\ prefix
    let exe_path_str = PathBuf::from("..\\gem-player.exe")
        .to_str()
        .ok_or("Failed to convert exe_path to string")?
        .replace('/', "\\");

    let data = InnoSetupScriptData {
        version,
        output_name,
        exe_path: exe_path_str,
    };

    // Render .iss file
    let rendered = handlebars.render("installer", &data).map_err(|e| e.to_string())?;

    let output_path = installer_dir.join("windows_installer.iss");
    let mut file = File::create(&output_path).map_err(|e| e.to_string())?;
    file.write_all(rendered.as_bytes()).map_err(|e| e.to_string())?;

    println!("✅ Generated Inno Setup script at {}", output_path.display());

    Ok(())
}
