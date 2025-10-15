use std::{
    env,
    fs::{create_dir_all, write},
    path::PathBuf,
};

extern crate winres;

fn main() {
    let is_windows = cfg!(target_os = "windows");
    let profile = std::env::var("PROFILE").unwrap_or_default();
    let is_release = profile == "release";

    if is_windows && is_release {
        let mut res = winres::WindowsResource::new();
        res.set_icon("assets/icon.ico");
        res.set_manifest_file("manifest.xml");
        res.compile().unwrap();

        generate_inno_script();
    }
}

fn generate_inno_script() {
    let version = env::var("CARGO_PKG_VERSION").expect("CARGO_PKG_VERSION not set");
    let target = env::var("CARGO_BUILD_TARGET").unwrap_or_else(|_| "x86_64-pc-windows-gnu".into());
    let profile = env::var("PROFILE").unwrap_or_else(|_| "release".into());

    let out_dir: PathBuf = ["target", &target, &profile].iter().collect();
    let installer_dir = out_dir.join("installer");

    let iss_content = format!(
        r#"; Auto-generated Inno Setup script for Gem Player

        [Setup]
        AppName=Gem Player
        AppVersion={version}
        DefaultDirName={{commonpf}}\Gem Player
        OutputDir={installer_dir}
        OutputBaseFilename=GemPlayerInstaller
        Compression=lzma
        SolidCompression=yes

        [Files]
        Source: "{exe_path}"; DestDir: "{{app}}"; Flags: ignoreversion

        [Icons]
        ; Start menu shortcut
        Name: "{{group}}\Gem Player"; Filename: "{{app}}\gem-player.exe"; IconFilename: "{{app}}\gem-player.exe"
        ; Desktop shortcut
        Name: "{{commondesktop}}\Gem Player"; Filename: "{{app}}\gem-player.exe"; IconFilename: "{{app}}\gem-player.exe"
        "#,
        version = version,
        installer_dir = installer_dir.display(),
        exe_path = out_dir.join("gem-player.exe").display()
    );

    create_dir_all(&installer_dir).expect("Failed to create installer directory");
    write(installer_dir.join("windows_installer.iss"), iss_content).expect("Failed to write installer script");
}
