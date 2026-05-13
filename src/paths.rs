use std::env;
use std::path::PathBuf;

pub fn home_dir() -> Option<PathBuf> {
    env::var_os("HOME").map(PathBuf::from)
}

pub fn env_path_or_home(env_name: &str, home_relative: &str) -> Option<PathBuf> {
    env::var_os(env_name)
        .map(PathBuf::from)
        .or_else(|| home_dir().map(|home| home.join(home_relative)))
}

pub fn aith_data_dir() -> Option<PathBuf> {
    env::var_os("AITH_HOME").map(PathBuf::from).or_else(|| {
        if cfg!(target_os = "macos") {
            home_dir().map(|home| home.join("Library/Application Support/aith"))
        } else if cfg!(target_os = "windows") {
            env::var_os("LOCALAPPDATA")
                .map(PathBuf::from)
                .map(|local_app_data| local_app_data.join("aith"))
        } else {
            env::var_os("XDG_DATA_HOME")
                .map(PathBuf::from)
                .or_else(|| home_dir().map(|home| home.join(".local/share")))
                .map(|data| data.join("aith"))
        }
    })
}

pub fn cursor_user_dir() -> Option<PathBuf> {
    if cfg!(target_os = "macos") {
        home_dir().map(|home| home.join("Library/Application Support/Cursor/User"))
    } else if cfg!(target_os = "windows") {
        env::var_os("APPDATA")
            .map(PathBuf::from)
            .map(|app_data| app_data.join("Cursor").join("User"))
    } else {
        env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|| home_dir().map(|home| home.join(".config")))
            .map(|config| config.join("Cursor").join("User"))
    }
}
