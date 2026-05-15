use std::ffi::OsString;
use std::fs;
use std::process::Command;

use anyhow::{Context, Result, bail};

use crate::profiles::{
    EnvProfileFile, EnvProfileSpec, ExecResult, ProfileStore, RemoveResult, SaveResult,
    ShellResult, ensure_file_exists, parent_dir, status_code, write_file_private,
};
use crate::tools::Tool;

pub(crate) const PROFILE_FILE: &str = "profile.toml";

pub(crate) fn save(
    store: &ProfileStore,
    tool: Tool,
    profile: &str,
    force: bool,
    spec: EnvProfileSpec,
    profile_label: &str,
) -> Result<SaveResult> {
    let destination = store.profile_file_path(tool, profile, PROFILE_FILE);
    if destination.exists() && !force {
        bail!(
            "profile '{}' already exists for {}; pass --force to overwrite it",
            profile,
            tool.key()
        );
    }

    let profile_file = EnvProfileFile::from_spec(spec)?;
    let contents = profile_file.to_toml()?;

    store.create_private_store_dir_all(parent_dir(&destination)?)?;
    write_file_private(&destination, contents.as_bytes())
        .with_context(|| format!("failed to save {profile_label} '{}'", profile))?;

    Ok(SaveResult {
        tool,
        profile: profile.to_owned(),
        source: None,
        destination,
    })
}

pub(crate) fn list(store: &ProfileStore, tool: Tool) -> Result<Vec<String>> {
    store.list_tool_profiles(tool, PROFILE_FILE)
}

pub(crate) fn remove(
    store: &ProfileStore,
    tool: Tool,
    profile: &str,
    profile_label: &str,
) -> Result<RemoveResult> {
    let profile_dir = store.tool_profiles_dir(tool).join(profile);
    let profile_path = profile_dir.join(PROFILE_FILE);
    ensure_file_exists(&profile_path, &format!("saved {profile_label}"))?;

    fs::remove_dir_all(&profile_dir)
        .with_context(|| format!("failed to remove {}", profile_dir.display()))?;

    Ok(RemoveResult {
        tool,
        profile: profile.to_owned(),
        removed: profile_dir,
    })
}

pub(crate) fn exec(
    store: &ProfileStore,
    tool: Tool,
    profile: &str,
    command: &[OsString],
    profile_label: &str,
) -> Result<ExecResult> {
    let env = load_profile(store, tool, profile, profile_label)?.resolve_env()?;

    let status = Command::new(&command[0])
        .args(&command[1..])
        .envs(env)
        .env("AITH_TOOL", tool.key())
        .env("AITH_PROFILE", profile)
        .status()
        .with_context(|| format!("failed to run command '{}'", command[0].to_string_lossy()))?;

    Ok(ExecResult {
        status_code: status_code(status),
    })
}

pub(crate) fn shell(
    store: &ProfileStore,
    tool: Tool,
    profile: &str,
    profile_label: &str,
) -> Result<ShellResult> {
    let env = load_profile(store, tool, profile, profile_label)?.resolve_env()?;
    let shell = user_shell();

    let status = Command::new(&shell)
        .envs(env)
        .env("AITH_TOOL", tool.key())
        .env("AITH_PROFILE", profile)
        .status()
        .with_context(|| format!("failed to start shell '{}'", shell.to_string_lossy()))?;

    Ok(ShellResult {
        status_code: status_code(status),
    })
}

fn load_profile(
    store: &ProfileStore,
    tool: Tool,
    profile: &str,
    profile_label: &str,
) -> Result<EnvProfileFile> {
    let path = store.profile_file_path(tool, profile, PROFILE_FILE);
    ensure_file_exists(&path, &format!("saved {profile_label}"))?;
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
