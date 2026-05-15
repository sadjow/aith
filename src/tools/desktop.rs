use std::env;
use std::path::PathBuf;

use crate::paths::home_dir;
use crate::tools::{Tool, ToolStatus};

pub(crate) fn inspect_codex() -> ToolStatus {
    let app_support = app_support_dir("Codex");
    let chat_app_support = app_support_dir("com.openai.chat");

    ToolStatus {
        tool: Tool::CodexDesktop,
        paths: vec![
            super::path_check("app bundle", macos_app_bundle("Codex")),
            super::path_check("user app bundle", macos_user_app_bundle("Codex")),
            super::path_check("app support", app_support.clone()),
            super::path_check("chat app support", chat_app_support),
            super::path_check("preferences", macos_preference("com.openai.codex.plist")),
            super::path_check("cookies", child_path(&app_support, &["Cookies"])),
            super::path_check(
                "local storage",
                child_path(&app_support, &["Local Storage", "leveldb"]),
            ),
            super::path_check(
                "session storage",
                child_path(&app_support, &["Session Storage"]),
            ),
        ],
        env: Vec::new(),
        notes: vec![
            "read-only desktop discovery; credential contents are never read",
            "Codex Desktop uses app-managed browser storage; profile switching is not implemented",
            "desktop app auth is tracked separately from Codex CLI auth",
        ],
    }
}

pub(crate) fn inspect_claude() -> ToolStatus {
    let app_support = app_support_dir("Claude");

    ToolStatus {
        tool: Tool::ClaudeDesktop,
        paths: vec![
            super::path_check("app bundle", macos_app_bundle("Claude")),
            super::path_check("user app bundle", macos_user_app_bundle("Claude")),
            super::path_check("app support", app_support.clone()),
            super::path_check(
                "desktop config",
                child_path(&app_support, &["claude_desktop_config.json"]),
            ),
            super::path_check("app config", child_path(&app_support, &["config.json"])),
            super::path_check(
                "preferences",
                macos_preference("com.anthropic.claudefordesktop.plist"),
            ),
            super::path_check("cookies", child_path(&app_support, &["Cookies"])),
            super::path_check(
                "local storage",
                child_path(&app_support, &["Local Storage", "leveldb"]),
            ),
            super::path_check("indexed db", child_path(&app_support, &["IndexedDB"])),
        ],
        env: Vec::new(),
        notes: vec![
            "read-only desktop discovery; credential contents are never read",
            "Claude Desktop may use app storage and macOS Keychain; Keychain is not inspected",
            "desktop app auth is tracked separately from Claude Code auth",
        ],
    }
}

pub(crate) fn inspect_cursor() -> ToolStatus {
    let app_support = app_support_dir("Cursor");
    let user_data = child_path(&app_support, &["User"]);

    ToolStatus {
        tool: Tool::CursorDesktop,
        paths: vec![
            super::path_check("app bundle", macos_app_bundle("Cursor")),
            super::path_check("user app bundle", macos_user_app_bundle("Cursor")),
            super::path_check("app support", app_support.clone()),
            super::path_check("user data", user_data.clone()),
            super::path_check("user settings", child_path(&user_data, &["settings.json"])),
            super::path_check("global storage", child_path(&user_data, &["globalStorage"])),
            super::path_check(
                "state database",
                child_path(&user_data, &["globalStorage", "state.vscdb"]),
            ),
            super::path_check("cookies", child_path(&app_support, &["Cookies"])),
            super::path_check(
                "local storage",
                child_path(&app_support, &["Local Storage", "leveldb"]),
            ),
        ],
        env: Vec::new(),
        notes: vec![
            "read-only desktop discovery; credential contents are never read",
            "Cursor Desktop uses Electron and VS Code-style user data; profile switching is not implemented",
            "desktop app auth is tracked separately from Cursor Agent auth",
        ],
    }
}

fn app_support_dir(app_name: &str) -> Option<PathBuf> {
    if cfg!(target_os = "macos") {
        home_dir().map(|home| {
            home.join("Library")
                .join("Application Support")
                .join(app_name)
        })
    } else if cfg!(target_os = "windows") {
        env::var_os("APPDATA")
            .map(PathBuf::from)
            .map(|path| path.join(app_name))
    } else {
        env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|| home_dir().map(|home| home.join(".config")))
            .map(|path| path.join(app_name))
    }
}

fn macos_app_bundle(app_name: &str) -> Option<PathBuf> {
    if cfg!(target_os = "macos") {
        Some(PathBuf::from("/Applications").join(format!("{app_name}.app")))
    } else {
        None
    }
}

fn macos_user_app_bundle(app_name: &str) -> Option<PathBuf> {
    if cfg!(target_os = "macos") {
        home_dir().map(|home| home.join("Applications").join(format!("{app_name}.app")))
    } else {
        None
    }
}

fn macos_preference(file_name: &str) -> Option<PathBuf> {
    if cfg!(target_os = "macos") {
        home_dir().map(|home| home.join("Library").join("Preferences").join(file_name))
    } else {
        None
    }
}

fn child_path(base: &Option<PathBuf>, parts: &[&str]) -> Option<PathBuf> {
    base.as_ref().map(|base| {
        parts
            .iter()
            .fold(base.clone(), |path, part| path.join(part))
    })
}
