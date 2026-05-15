use std::ffi::OsString;

use anyhow::Result;

use crate::paths::cursor_user_dir;
use crate::profiles::{
    EnvProfileSpec, ExecResult, ProfileStore, RemoveResult, SaveResult, ShellResult,
};
use crate::tools::{Tool, ToolStatus, env_session};

const PROFILE_LABEL: &str = "Cursor env profile";
const AUTH_ENV: &[&str] = &["CURSOR_API_KEY"];

pub(crate) fn save(
    store: &ProfileStore,
    profile: &str,
    force: bool,
    spec: EnvProfileSpec,
) -> Result<SaveResult> {
    env_session::save(
        store,
        Tool::CursorAgent,
        profile,
        force,
        spec,
        PROFILE_LABEL,
    )
}

pub(crate) fn list(store: &ProfileStore) -> Result<Vec<String>> {
    env_session::list(store, Tool::CursorAgent)
}

pub(crate) fn remove(store: &ProfileStore, profile: &str) -> Result<RemoveResult> {
    env_session::remove(store, Tool::CursorAgent, profile, PROFILE_LABEL)
}

pub(crate) fn exec(
    store: &ProfileStore,
    profile: &str,
    command: &[OsString],
) -> Result<ExecResult> {
    env_session::exec(store, Tool::CursorAgent, profile, command, PROFILE_LABEL)
}

pub(crate) fn shell(store: &ProfileStore, profile: &str) -> Result<ShellResult> {
    env_session::shell(store, Tool::CursorAgent, profile, PROFILE_LABEL)
}

pub(crate) fn inspect() -> ToolStatus {
    ToolStatus {
        tool: Tool::CursorAgent,
        paths: vec![super::path_check("user data", cursor_user_dir())],
        env: super::env_checks(&["CURSOR_API_KEY"]),
        notes: vec!["Cursor Agent supports session auth through CURSOR_API_KEY or --api-key"],
    }
}

pub(crate) fn has_terminal_auth_env() -> bool {
    AUTH_ENV.iter().any(|name| std::env::var_os(name).is_some())
}
