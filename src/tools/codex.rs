use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, bail};
use serde_json::Value;

use crate::paths::env_path_or_home;
use crate::profiles::{
    BackupEntry, CurrentResult, CurrentState, ExecResult, ProfileStore, RemoveResult,
    RestoreResult, SaveResult, ShellResult, TempDir, UseResult, copy_file_private,
    create_private_dir_all, current_state, ensure_file_exists, ensure_removable_profile,
    parent_dir, status_code, write_file_private,
};
use crate::tools::{Tool, ToolStatus};

const AUTH_FILE: &str = "auth.json";
const CONFIG_FILE: &str = "config.toml";
const BACKUP_EXTENSION: &str = ".json";
const ACTIVE_PROFILE_FILE: &str = "active-profile";

pub(crate) fn save(store: &ProfileStore, profile: &str, force: bool) -> Result<SaveResult> {
    let source = auth_path()?;
    ensure_file_exists(&source, "Codex CLI auth file")?;

    let destination = store.profile_file_path(Tool::CodexCli, profile, AUTH_FILE);
    if destination.exists() && !force {
        bail!(
            "profile '{}' already exists for {}; pass --force to overwrite it",
            profile,
            Tool::CodexCli.key()
        );
    }

    store.create_private_store_dir_all(parent_dir(&destination)?)?;
    copy_file_private(&source, &destination)
        .with_context(|| format!("failed to save Codex CLI auth profile '{}'", profile))?;
    write_active_profile_marker(store, profile)?;

    Ok(SaveResult {
        tool: Tool::CodexCli,
        profile: profile.to_owned(),
        source: Some(source),
        destination,
    })
}

pub(crate) fn use_profile(store: &ProfileStore, profile: &str) -> Result<UseResult> {
    let source = store.profile_file_path(Tool::CodexCli, profile, AUTH_FILE);
    ensure_file_exists(&source, "saved Codex CLI auth profile")?;

    sync_tracked_active_profile(store, profile)?;

    let destination = auth_path()?;
    create_private_dir_all(parent_dir(&destination)?)?;

    let backup = backup_active_auth(store, &destination)?;

    copy_file_private(&source, &destination)
        .with_context(|| format!("failed to switch Codex to profile '{}'", profile))?;
    write_active_profile_marker(store, profile)?;

    Ok(UseResult {
        tool: Tool::CodexCli,
        profile: profile.to_owned(),
        source,
        destination,
        backup,
    })
}

pub(crate) fn list(store: &ProfileStore) -> Result<Vec<String>> {
    store.list_tool_profiles(Tool::CodexCli, AUTH_FILE)
}

pub(crate) fn current(store: &ProfileStore) -> Result<CurrentResult> {
    let active = auth_path()?;
    if !active.is_file() {
        return Ok(CurrentResult {
            tool: Tool::CodexCli,
            state: CurrentState::Unknown,
        });
    }

    let active_auth = fs::read(&active).with_context(|| {
        format!(
            "failed to read active Codex CLI auth at {}",
            active.display()
        )
    })?;

    let mut matches = Vec::new();
    for profile in list(store)? {
        let profile_auth_path = store.profile_file_path(Tool::CodexCli, &profile, AUTH_FILE);
        let profile_auth = fs::read(&profile_auth_path).with_context(|| {
            format!(
                "failed to read saved Codex CLI profile '{}' at {}",
                profile,
                profile_auth_path.display()
            )
        })?;

        if profile_auth == active_auth {
            matches.push(profile);
        }
    }

    Ok(CurrentResult {
        tool: Tool::CodexCli,
        state: current_state(matches),
    })
}

pub(crate) fn remove(store: &ProfileStore, profile: &str, force: bool) -> Result<RemoveResult> {
    let profile_dir = store.tool_profiles_dir(Tool::CodexCli).join(profile);
    let auth_path = profile_dir.join(AUTH_FILE);
    ensure_file_exists(&auth_path, "saved Codex CLI auth profile")?;

    let current = current(store)?;
    ensure_removable_profile(Tool::CodexCli, profile, &current.state, force)?;

    fs::remove_dir_all(&profile_dir)
        .with_context(|| format!("failed to remove {}", profile_dir.display()))?;
    clear_active_profile_marker_if_matches(store, profile)?;

    Ok(RemoveResult {
        tool: Tool::CodexCli,
        profile: profile.to_owned(),
        removed: profile_dir,
    })
}

pub(crate) fn backups(store: &ProfileStore) -> Result<Vec<BackupEntry>> {
    store.list_tool_backups(Tool::CodexCli)
}

pub(crate) fn restore(store: &ProfileStore, backup_id: &str) -> Result<RestoreResult> {
    let source = store.tool_backups_dir(Tool::CodexCli).join(backup_id);
    ensure_file_exists(&source, "Codex CLI auth backup")?;

    let destination = auth_path()?;
    create_private_dir_all(parent_dir(&destination)?)?;

    let backup = backup_active_auth(store, &destination)?;

    copy_file_private(&source, &destination)
        .with_context(|| format!("failed to restore Codex backup '{}'", backup_id))?;
    clear_active_profile_marker(store)?;

    Ok(RestoreResult {
        tool: Tool::CodexCli,
        backup_id: backup_id.to_owned(),
        source,
        destination,
        backup,
    })
}

pub(crate) fn exec(
    store: &ProfileStore,
    profile: &str,
    command: &[OsString],
) -> Result<ExecResult> {
    let session = CodexSession::stage(store, profile)?;

    let status = Command::new(&command[0])
        .args(&command[1..])
        .env("CODEX_HOME", session.home())
        .status()
        .with_context(|| format!("failed to run command '{}'", command[0].to_string_lossy()))?;

    session.sync_profile()?;

    Ok(ExecResult {
        status_code: status_code(status),
    })
}

pub(crate) fn shell(store: &ProfileStore, profile: &str) -> Result<ShellResult> {
    let session = CodexSession::stage(store, profile)?;
    let shell = user_shell();

    let status = Command::new(&shell)
        .env("CODEX_HOME", session.home())
        .env("AITH_TOOL", Tool::CodexCli.key())
        .env("AITH_PROFILE", profile)
        .status()
        .with_context(|| format!("failed to start shell '{}'", shell.to_string_lossy()))?;

    session.sync_profile()?;

    Ok(ShellResult {
        status_code: status_code(status),
    })
}

pub(crate) fn inspect() -> ToolStatus {
    let config_dir = config_dir();

    ToolStatus {
        tool: Tool::CodexCli,
        paths: vec![
            super::path_check("config dir", config_dir.clone()),
            super::path_check(
                "auth file",
                config_dir.as_ref().map(|path| path.join(AUTH_FILE)),
            ),
            super::path_check(
                "config file",
                config_dir.as_ref().map(|path| path.join(CONFIG_FILE)),
            ),
        ],
        env: super::env_checks(&["CODEX_HOME", "OPENAI_API_KEY", "CODEX_ACCESS_TOKEN"]),
        notes: vec!["checks file presence only; credential contents are never read"],
    }
}

#[derive(Debug)]
struct CodexSession {
    temp_dir: TempDir,
    profile_auth_path: PathBuf,
}

impl CodexSession {
    fn stage(store: &ProfileStore, profile: &str) -> Result<Self> {
        let source = store.profile_file_path(Tool::CodexCli, profile, AUTH_FILE);
        ensure_file_exists(&source, "saved Codex CLI auth profile")?;

        let temp_dir = TempDir::create("aith-codex")?;
        let home = temp_dir.path();

        copy_file_private(&source, &home.join(AUTH_FILE))
            .with_context(|| format!("failed to stage Codex CLI profile '{}'", profile))?;

        let config_path = active_config_path()?;
        if config_path.is_file() {
            copy_file_private(&config_path, &home.join(CONFIG_FILE))
                .context("failed to stage Codex config")?;
        }

        Ok(Self {
            temp_dir,
            profile_auth_path: source,
        })
    }

    fn home(&self) -> &Path {
        self.temp_dir.path()
    }

    fn sync_profile(&self) -> Result<()> {
        let session_auth = self.home().join(AUTH_FILE);
        if !session_auth.is_file() {
            return Ok(());
        }

        copy_file_private(&session_auth, &self.profile_auth_path)
            .context("failed to sync refreshed Codex CLI auth back to saved profile")
    }
}

fn backup_active_auth(store: &ProfileStore, active_auth_path: &Path) -> Result<Option<PathBuf>> {
    if !active_auth_path.exists() {
        return Ok(None);
    }

    let backup = store.backup_path(Tool::CodexCli, BACKUP_EXTENSION)?;
    store.create_private_store_dir_all(parent_dir(&backup)?)?;
    copy_file_private(active_auth_path, &backup)
        .context("failed to back up current Codex CLI auth file")?;

    Ok(Some(backup))
}

fn sync_tracked_active_profile(store: &ProfileStore, next_profile: &str) -> Result<()> {
    let Some(active_profile) = read_active_profile_marker(store)? else {
        return Ok(());
    };

    if active_profile == next_profile {
        return Ok(());
    }

    let active_auth = auth_path()?;
    if !active_auth.is_file() {
        return Ok(());
    }

    let profile_auth = store.profile_file_path(Tool::CodexCli, &active_profile, AUTH_FILE);
    if !profile_auth.is_file() || !should_sync_auth(&active_auth, &profile_auth)? {
        return Ok(());
    }

    copy_file_private(&active_auth, &profile_auth).with_context(|| {
        format!(
            "failed to sync refreshed Codex CLI auth back to profile '{}'",
            active_profile
        )
    })
}

fn should_sync_auth(active_auth: &Path, profile_auth: &Path) -> Result<bool> {
    let active = fs::read(active_auth).with_context(|| {
        format!(
            "failed to read active Codex CLI auth at {}",
            active_auth.display()
        )
    })?;
    let profile = fs::read(profile_auth).with_context(|| {
        format!(
            "failed to read saved Codex CLI auth at {}",
            profile_auth.display()
        )
    })?;

    if active == profile {
        return Ok(false);
    }

    Ok(auth_account_id(&active).is_some_and(|active_account| {
        auth_account_id(&profile).is_some_and(|profile_account| active_account == profile_account)
    }))
}

fn auth_account_id(auth: &[u8]) -> Option<String> {
    let value: Value = serde_json::from_slice(auth).ok()?;
    value
        .get("tokens")?
        .get("account_id")?
        .as_str()
        .filter(|account_id| !account_id.is_empty())
        .map(str::to_owned)
}

fn active_profile_marker_path(store: &ProfileStore) -> PathBuf {
    store
        .root()
        .join("state")
        .join(Tool::CodexCli.storage_key())
        .join(ACTIVE_PROFILE_FILE)
}

fn read_active_profile_marker(store: &ProfileStore) -> Result<Option<String>> {
    let path = active_profile_marker_path(store);
    if !path.is_file() {
        return Ok(None);
    }

    let profile = fs::read_to_string(&path)
        .with_context(|| format!("failed to read {}", path.display()))?
        .trim()
        .to_owned();

    Ok((!profile.is_empty()).then_some(profile))
}

fn write_active_profile_marker(store: &ProfileStore, profile: &str) -> Result<()> {
    let path = active_profile_marker_path(store);
    write_file_private(&path, format!("{profile}\n").as_bytes())
        .with_context(|| format!("failed to track active Codex CLI profile '{}'", profile))
}

fn clear_active_profile_marker_if_matches(store: &ProfileStore, profile: &str) -> Result<()> {
    if read_active_profile_marker(store)?.as_deref() == Some(profile) {
        clear_active_profile_marker(store)?;
    }

    Ok(())
}

fn clear_active_profile_marker(store: &ProfileStore) -> Result<()> {
    let path = active_profile_marker_path(store);
    if path.exists() {
        fs::remove_file(&path).with_context(|| format!("failed to remove {}", path.display()))?;
    }

    Ok(())
}

fn auth_path() -> Result<PathBuf> {
    Ok(active_config_dir()?.join(AUTH_FILE))
}

fn active_config_path() -> Result<PathBuf> {
    Ok(active_config_dir()?.join(CONFIG_FILE))
}

fn active_config_dir() -> Result<PathBuf> {
    config_dir().context("could not determine Codex config directory")
}

fn config_dir() -> Option<PathBuf> {
    env_path_or_home("CODEX_HOME", ".codex")
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
