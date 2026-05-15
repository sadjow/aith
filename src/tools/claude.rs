use std::ffi::OsString;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

use anyhow::{Context, Result, bail};

use crate::paths::{env_path_or_home, home_dir};
use crate::profiles::{
    EnvProfileFile, EnvProfileSpec, ExecResult, ProfileStore, RemoveResult, SaveResult,
    ShellResult, ensure_file_exists, parent_dir, status_code, write_file_private,
};
use crate::tools::{Tool, ToolStatus};

pub(crate) const CONFIG_DIR_LABEL: &str = "user config dir";
pub(crate) const CREDENTIALS_LABEL: &str = "credentials file";
pub(crate) const PROFILE_FILE: &str = "profile.toml";

const AUTH_ENV: &[&str] = &[
    "ANTHROPIC_API_KEY",
    "ANTHROPIC_AUTH_TOKEN",
    "CLAUDE_CODE_OAUTH_TOKEN",
    "CLAUDE_CODE_USE_BEDROCK",
    "CLAUDE_CODE_USE_VERTEX",
    "CLAUDE_CODE_USE_FOUNDRY",
];

pub(crate) fn save(
    store: &ProfileStore,
    profile: &str,
    force: bool,
    spec: EnvProfileSpec,
) -> Result<SaveResult> {
    let destination = store.profile_file_path(Tool::Claude, profile, PROFILE_FILE);
    if destination.exists() && !force {
        bail!(
            "profile '{}' already exists for {}; pass --force to overwrite it",
            profile,
            Tool::Claude.key()
        );
    }

    let profile_file = EnvProfileFile::from_spec(spec)?;
    let contents = profile_file.to_toml()?;

    store.create_private_store_dir_all(parent_dir(&destination)?)?;
    write_file_private(&destination, contents.as_bytes())
        .with_context(|| format!("failed to save Claude env profile '{}'", profile))?;

    Ok(SaveResult {
        tool: Tool::Claude,
        profile: profile.to_owned(),
        source: None,
        destination,
    })
}

pub(crate) fn list(store: &ProfileStore) -> Result<Vec<String>> {
    store.list_tool_profiles(Tool::Claude, PROFILE_FILE)
}

pub(crate) fn remove(store: &ProfileStore, profile: &str) -> Result<RemoveResult> {
    let profile_dir = store.tool_profiles_dir(Tool::Claude).join(profile);
    let profile_path = profile_dir.join(PROFILE_FILE);
    ensure_file_exists(&profile_path, "saved Claude env profile")?;

    fs::remove_dir_all(&profile_dir)
        .with_context(|| format!("failed to remove {}", profile_dir.display()))?;

    Ok(RemoveResult {
        tool: Tool::Claude,
        profile: profile.to_owned(),
        removed: profile_dir,
    })
}

pub(crate) fn exec(
    store: &ProfileStore,
    profile: &str,
    command: &[OsString],
) -> Result<ExecResult> {
    let env = load_profile(store, profile)?.resolve_env()?;

    let status = Command::new(&command[0])
        .args(&command[1..])
        .envs(env)
        .env("AITH_TOOL", Tool::Claude.key())
        .env("AITH_PROFILE", profile)
        .status()
        .with_context(|| format!("failed to run command '{}'", command[0].to_string_lossy()))?;

    Ok(ExecResult {
        status_code: status_code(status),
    })
}

pub(crate) fn shell(store: &ProfileStore, profile: &str) -> Result<ShellResult> {
    let env = load_profile(store, profile)?.resolve_env()?;
    let shell = user_shell();

    let status = Command::new(&shell)
        .envs(env)
        .env("AITH_TOOL", Tool::Claude.key())
        .env("AITH_PROFILE", profile)
        .status()
        .with_context(|| format!("failed to start shell '{}'", shell.to_string_lossy()))?;

    Ok(ShellResult {
        status_code: status_code(status),
    })
}

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

fn load_profile(store: &ProfileStore, profile: &str) -> Result<EnvProfileFile> {
    let path = store.profile_file_path(Tool::Claude, profile, PROFILE_FILE);
    ensure_file_exists(&path, "saved Claude env profile")?;
    EnvProfileFile::read(&path)
}

fn user_shell() -> OsString {
    #[cfg(windows)]
    {
        std::env::var_os("COMSPEC").unwrap_or_else(|| OsString::from("cmd.exe"))
    }

    #[cfg(not(windows))]
    {
        std::env::var_os("SHELL").unwrap_or_else(|| OsString::from("/bin/sh"))
    }
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
