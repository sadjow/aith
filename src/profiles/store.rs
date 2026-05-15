use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result, bail};

use crate::paths::aith_data_dir;
use crate::profiles::{
    BackupEntry, CurrentResult, CurrentState, EnvProfileSpec, ExecResult, RemoveResult,
    RestoreResult, SaveResult, ShellResult, UseResult, validate_backup_id, validate_profile_name,
};
use crate::tools::{self, Tool};

#[derive(Debug)]
pub struct ProfileStore {
    root: PathBuf,
}

impl ProfileStore {
    pub fn new() -> Result<Self> {
        let root = aith_data_dir().context("could not determine aith data directory")?;
        Ok(Self { root })
    }

    pub fn with_root(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn save(&self, tool: Tool, profile: &str, force: bool) -> Result<SaveResult> {
        validate_profile_name(profile)?;

        match tool {
            Tool::Codex => tools::codex::save(self, profile, force),
            Tool::Claude | Tool::Cursor => unsupported(tool),
        }
    }

    pub fn save_env(
        &self,
        tool: Tool,
        profile: &str,
        force: bool,
        spec: EnvProfileSpec,
    ) -> Result<SaveResult> {
        validate_profile_name(profile)?;

        match tool {
            Tool::Claude => tools::claude::save(self, profile, force, spec),
            Tool::Codex | Tool::Cursor => bail!(
                "{} does not support env-based profiles",
                tool.display_name()
            ),
        }
    }

    pub fn use_profile(&self, tool: Tool, profile: &str) -> Result<UseResult> {
        validate_profile_name(profile)?;

        match tool {
            Tool::Codex => tools::codex::use_profile(self, profile),
            Tool::Claude | Tool::Cursor => unsupported(tool),
        }
    }

    pub fn list(&self, tool: Tool) -> Result<Vec<String>> {
        match tool {
            Tool::Codex => tools::codex::list(self),
            Tool::Claude => tools::claude::list(self),
            Tool::Cursor => unsupported(tool),
        }
    }

    pub fn current(&self, tool: Tool) -> Result<CurrentResult> {
        match tool {
            Tool::Codex => tools::codex::current(self),
            Tool::Claude => Ok(CurrentResult {
                tool,
                state: CurrentState::Unknown,
            }),
            Tool::Cursor => unsupported(tool),
        }
    }

    pub fn remove(&self, tool: Tool, profile: &str, force: bool) -> Result<RemoveResult> {
        validate_profile_name(profile)?;

        match tool {
            Tool::Codex => tools::codex::remove(self, profile, force),
            Tool::Claude => tools::claude::remove(self, profile),
            Tool::Cursor => unsupported(tool),
        }
    }

    pub fn backups(&self, tool: Tool) -> Result<Vec<BackupEntry>> {
        match tool {
            Tool::Codex => tools::codex::backups(self),
            Tool::Claude => Ok(Vec::new()),
            Tool::Cursor => unsupported(tool),
        }
    }

    pub fn restore(&self, tool: Tool, backup_id: &str) -> Result<RestoreResult> {
        validate_backup_id(backup_id)?;

        match tool {
            Tool::Codex => tools::codex::restore(self, backup_id),
            Tool::Claude | Tool::Cursor => unsupported(tool),
        }
    }

    pub fn exec_profile(
        &self,
        tool: Tool,
        profile: &str,
        command: &[OsString],
    ) -> Result<ExecResult> {
        validate_profile_name(profile)?;

        if command.is_empty() {
            bail!("command cannot be empty");
        }

        match tool {
            Tool::Codex => tools::codex::exec(self, profile, command),
            Tool::Claude => tools::claude::exec(self, profile, command),
            Tool::Cursor => unsupported(tool),
        }
    }

    pub fn shell_profile(&self, tool: Tool, profile: &str) -> Result<ShellResult> {
        validate_profile_name(profile)?;

        match tool {
            Tool::Codex => tools::codex::shell(self, profile),
            Tool::Claude => tools::claude::shell(self, profile),
            Tool::Cursor => unsupported(tool),
        }
    }

    pub(crate) fn profile_file_path(&self, tool: Tool, profile: &str, file_name: &str) -> PathBuf {
        self.tool_profiles_dir(tool).join(profile).join(file_name)
    }

    pub(crate) fn tool_profiles_dir(&self, tool: Tool) -> PathBuf {
        self.root.join("profiles").join(tool.key())
    }

    pub(crate) fn list_tool_profiles(
        &self,
        tool: Tool,
        required_file: &str,
    ) -> Result<Vec<String>> {
        let root = self.tool_profiles_dir(tool);
        if !root.exists() {
            return Ok(Vec::new());
        }

        let mut profiles = Vec::new();
        for entry in
            fs::read_dir(&root).with_context(|| format!("failed to read {}", root.display()))?
        {
            let entry = entry?;
            if !entry.file_type()?.is_dir() {
                continue;
            }

            let name = entry.file_name().to_string_lossy().into_owned();
            if entry.path().join(required_file).is_file() {
                profiles.push(name);
            }
        }

        profiles.sort();
        Ok(profiles)
    }

    pub(crate) fn tool_backups_dir(&self, tool: Tool) -> PathBuf {
        self.root.join("backups").join(tool.key())
    }

    pub(crate) fn list_tool_backups(&self, tool: Tool) -> Result<Vec<BackupEntry>> {
        let root = self.tool_backups_dir(tool);
        if !root.exists() {
            return Ok(Vec::new());
        }

        let mut backups = Vec::new();
        for entry in
            fs::read_dir(&root).with_context(|| format!("failed to read {}", root.display()))?
        {
            let entry = entry?;
            if !entry.file_type()?.is_file() {
                continue;
            }

            let id = entry.file_name().to_string_lossy().into_owned();
            if validate_backup_id(&id).is_ok() {
                backups.push(BackupEntry {
                    id,
                    path: entry.path(),
                });
            }
        }

        backups.sort_by(|left, right| left.id.cmp(&right.id));
        Ok(backups)
    }

    pub(crate) fn backup_path(&self, tool: Tool, extension: &str) -> Result<PathBuf> {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .context("system clock is before UNIX epoch")?
            .as_secs();

        Ok(self.tool_backups_dir(tool).join(format!(
            "auth-{timestamp}-{}{extension}",
            std::process::id()
        )))
    }

    pub(crate) fn create_private_store_dir_all(&self, path: &Path) -> Result<()> {
        if !path.starts_with(&self.root) {
            bail!(
                "refusing to create store path outside {}",
                self.root.display()
            );
        }

        fs::create_dir_all(path).with_context(|| format!("failed to create {}", path.display()))?;

        #[cfg(unix)]
        {
            crate::profiles::fs::secure_dir(&self.root)?;

            let relative = path
                .strip_prefix(&self.root)
                .with_context(|| format!("failed to inspect {}", path.display()))?;
            let mut current = self.root.clone();

            for component in relative.components() {
                current.push(component);
                crate::profiles::fs::secure_dir(&current)?;
            }
        }

        Ok(())
    }
}

fn unsupported<T>(tool: Tool) -> Result<T> {
    bail!(
        "{} profile switching is not implemented yet",
        tool.display_name()
    );
}
