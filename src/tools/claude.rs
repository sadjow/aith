use std::path::PathBuf;

use crate::paths::{env_path_or_home, home_dir};
use crate::tools::{Tool, ToolStatus};

pub(crate) const CONFIG_DIR_LABEL: &str = "user config dir";
pub(crate) const CREDENTIALS_LABEL: &str = "credentials file";

const AUTH_ENV: &[&str] = &[
    "ANTHROPIC_API_KEY",
    "ANTHROPIC_AUTH_TOKEN",
    "CLAUDE_CODE_OAUTH_TOKEN",
    "CLAUDE_CODE_USE_BEDROCK",
    "CLAUDE_CODE_USE_VERTEX",
    "CLAUDE_CODE_USE_FOUNDRY",
];

pub(crate) fn inspect() -> ToolStatus {
    let config_dir = config_dir();

    ToolStatus {
        tool: Tool::Claude,
        paths: vec![
            super::path_check(CONFIG_DIR_LABEL, config_dir.clone()),
            super::path_check(
                "user settings",
                config_dir.as_ref().map(|path| path.join("settings.json")),
            ),
            super::path_check(CREDENTIALS_LABEL, credentials_path()),
            super::path_check(
                "user state",
                home_dir().map(|path| path.join(".claude.json")),
            ),
            super::path_check(
                "project settings",
                current_dir().map(|path| path.join(".claude").join("settings.json")),
            ),
            super::path_check(
                "project local",
                current_dir().map(|path| path.join(".claude").join("settings.local.json")),
            ),
            super::path_check("managed settings", managed_settings_path()),
        ],
        env: super::env_checks(&[
            "CLAUDE_CONFIG_DIR",
            "ANTHROPIC_API_KEY",
            "ANTHROPIC_AUTH_TOKEN",
            "CLAUDE_CODE_OAUTH_TOKEN",
            "ANTHROPIC_BASE_URL",
            "CLAUDE_CODE_USE_BEDROCK",
            "CLAUDE_CODE_USE_VERTEX",
            "CLAUDE_CODE_USE_FOUNDRY",
            "CLAUDE_CODE_API_KEY_HELPER_TTL_MS",
        ]),
        notes: notes(),
    }
}

pub(crate) fn has_terminal_auth_env() -> bool {
    AUTH_ENV.iter().any(|name| std::env::var_os(name).is_some())
}

pub(crate) fn credentials_are_path_backed() -> bool {
    !cfg!(target_os = "macos")
}

fn config_dir() -> Option<PathBuf> {
    env_path_or_home("CLAUDE_CONFIG_DIR", ".claude")
}

fn credentials_path() -> Option<PathBuf> {
    if credentials_are_path_backed() {
        config_dir().map(|path| path.join(".credentials.json"))
    } else {
        None
    }
}

fn managed_settings_path() -> Option<PathBuf> {
    if cfg!(target_os = "macos") {
        Some(PathBuf::from(
            "/Library/Application Support/ClaudeCode/managed-settings.json",
        ))
    } else if cfg!(target_os = "windows") {
        std::env::var_os("ProgramData")
            .or_else(|| std::env::var_os("PROGRAMDATA"))
            .map(PathBuf::from)
            .map(|path| path.join("ClaudeCode").join("managed-settings.json"))
    } else {
        Some(PathBuf::from("/etc/claude-code/managed-settings.json"))
    }
}

fn current_dir() -> Option<PathBuf> {
    std::env::current_dir().ok()
}

fn notes() -> Vec<&'static str> {
    let mut notes = vec![
        "checks file presence and env presence only; credential contents are never printed",
        "terminal auth can use ANTHROPIC_API_KEY, ANTHROPIC_AUTH_TOKEN, CLAUDE_CODE_OAUTH_TOKEN, or cloud-provider env",
    ];

    if cfg!(target_os = "macos") {
        notes.push("macOS subscription credentials are stored in Keychain and are not inspected");
    } else {
        notes.push("Claude Code manages .credentials.json through /login and /logout");
    }

    notes
}
