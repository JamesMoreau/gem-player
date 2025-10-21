#[cfg(target_os = "windows")]
mod windows_build {
    use handlebars::Handlebars;
    use serde::Serialize;
    use std::{env, fs, path::Path};

    #[derive(Serialize)]
    pub struct InnoSetupScriptData {
        version: String,
    }

    pub fn generate_inno_setup_script() -> Result<(), String> {
        let version = env::var("CARGO_PKG_VERSION").map_err(|e| e.to_string())?;

        let template = fs::read_to_string("windows_installer.iss.hbs").map_err(|e| e.to_string())?;
        let mut handlebars = Handlebars::new();
        handlebars
            .register_template_string("installer", &template)
            .map_err(|e| e.to_string())?;

        let data = InnoSetupScriptData { version };
        let rendered = handlebars.render("installer", &data).map_err(|e| e.to_string())?;

        let output_path = Path::new("windows_installer.iss");
        fs::write(output_path, rendered).map_err(|e| e.to_string())?;

        println!("✅ Generated Inno Setup script at {}", output_path.display());
        Ok(())
    }
}

fn main() -> Result<(), ()> {
    #[cfg(all(target_os = "windows", not(debug_assertions)))]
    {
        let mut res = winres::WindowsResource::new();
        res.set_icon("assets/icon.ico");
        res.set_manifest_file("manifest.xml");
        if let Err(e) = res.compile() {
            eprintln!("⚠️  Failed to package Windows resources: {e}");
            return Err(());
        }

        if let Err(e) = windows_build::generate_inno_setup_script() {
            eprintln!("⚠️  Failed to generate Inno Setup script: {e}");
            return Err(());
        }
    }

    Ok(())
}
