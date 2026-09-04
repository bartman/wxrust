use lazy_static::lazy_static;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

lazy_static! {
    static ref CREDENTIALS_PATH: Mutex<Option<String>> = Mutex::new(None);
}

pub fn set_credentials_path(path: &str) {
    if !Path::new(path).exists() {
        eprintln!("Credentials file '{}' not found.", path);
        std::process::exit(1);
    }
    *CREDENTIALS_PATH.lock().unwrap() = Some(path.to_string());
}

pub fn get_credentials_path() -> Result<String, String> {
    let mut path_opt = CREDENTIALS_PATH.lock().unwrap();
    if let Some(ref path) = *path_opt {
        return Ok(path.clone());
    }

    // Discover path
    let mut paths = Vec::new();

    // XDG config directory
    if let Some(config_dir) = dirs::config_dir() {
        paths.push(config_dir.join("wxrust").join("credentials.txt"));
    }

    // Fallback to ~/.config
    if let Ok(home) = std::env::var("HOME") {
        paths.push(
            PathBuf::from(home)
                .join(".config")
                .join("wxrust")
                .join("credentials.txt"),
        );
    }

    // Current directory
    paths.push(PathBuf::from("credentials.txt"));

    for path in paths {
        if path.exists() {
            let path_str = path.to_string_lossy().to_string();
            *path_opt = Some(path_str.clone());
            return Ok(path_str);
        }
    }

    Err("Credentials file not found.".to_string())
}
