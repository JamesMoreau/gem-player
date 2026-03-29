use std::path::PathBuf;

pub fn resource_path(name: &str) -> PathBuf {
    #[cfg(target_os = "macos")]
    {
        use core_foundation::bundle::CFBundle;
        if let Some(mut path) = CFBundle::main_bundle().resources_path() {
            path.push(name);
            if path.exists() {
                return path;
            }
        }
    }

    #[cfg(target_os = "windows")]
    {
        let mut path = std::env::current_exe().unwrap();
        path.pop();
        path.push("resources");
        path.push(name);
        if path.exists() {
            return path;
        }
    }

    // Dev fallback. Of course there is no resources folder in debug mode.
    PathBuf::from(format!("assets/{}", name))
}
