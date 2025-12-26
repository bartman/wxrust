use std::path::Path;
use std::sync::Mutex;
use lazy_static::lazy_static;

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
    let home = std::env::var("HOME").unwrap_or(".".to_string());
    let xdg_config = std::env::var("XDG_CONFIG_HOME").unwrap_or_else(|_| format!("{}/.config", home));
    let paths = vec![
        format!("{}/wxrust/credentials.txt", xdg_config),
        format!("{}/.config/wxrust/credentials.txt", home),
        "credentials.txt".to_string(),
    ];

    for path in paths {
        if Path::new(&path).exists() {
            *path_opt = Some(path.clone());
            return Ok(path);
        }
    }

    Err("Credentials file not found.".to_string())
}