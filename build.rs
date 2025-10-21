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
    }

    Ok(())
}
