use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result, bail};

use crate::paths::{aith_data_dir, env_path_or_home};
use crate::tools::Tool;

const CODEX_AUTH_FILE: &str = "auth.json";
const CODEX_CONFIG_FILE: &str = "config.toml";

#[derive(Debug)]
pub struct ProfileStore {
    root: PathBuf,
}

#[derive(Debug)]
pub struct SaveResult {
    pub tool: Tool,
    pub profile: String,
    pub source: PathBuf,
    pub destination: PathBuf,
}

#[derive(Debug)]
pub struct UseResult {
    pub tool: Tool,
    pub profile: String,
    pub source: PathBuf,
    pub destination: PathBuf,
    pub backup: Option<PathBuf>,
}

#[derive(Debug)]
pub struct RestoreResult {
    pub tool: Tool,
    pub backup_id: String,
    pub source: PathBuf,
    pub destination: PathBuf,
    pub backup: Option<PathBuf>,
}

#[derive(Debug)]
pub struct ExecResult {
    pub status_code: i32,
}

#[derive(Debug)]
pub struct RemoveResult {
    pub tool: Tool,
    pub profile: String,
    pub removed: PathBuf,
}

#[derive(Debug)]
pub struct CurrentResult {
    pub tool: Tool,
    pub state: CurrentState,
}

#[derive(Debug)]
pub struct BackupEntry {
    pub id: String,
    pub path: PathBuf,
}

#[derive(Debug, Eq, PartialEq)]
pub enum CurrentState {
    Known(String),
    Ambiguous(Vec<String>),
    Unknown,
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
            Tool::Codex => self.save_codex(profile, force),
            Tool::Claude | Tool::Cursor => unsupported(tool),
        }
    }

    pub fn use_profile(&self, tool: Tool, profile: &str) -> Result<UseResult> {
        validate_profile_name(profile)?;

        match tool {
            Tool::Codex => self.use_codex(profile),
            Tool::Claude | Tool::Cursor => unsupported(tool),
        }
    }

    pub fn list(&self, tool: Tool) -> Result<Vec<String>> {
        match tool {
            Tool::Codex => self.list_tool_profiles(tool),
            Tool::Claude | Tool::Cursor => unsupported(tool),
        }
    }

    pub fn current(&self, tool: Tool) -> Result<CurrentResult> {
        match tool {
            Tool::Codex => self.current_codex(),
            Tool::Claude | Tool::Cursor => unsupported(tool),
        }
    }

    pub fn remove(&self, tool: Tool, profile: &str, force: bool) -> Result<RemoveResult> {
        validate_profile_name(profile)?;

        match tool {
            Tool::Codex => self.remove_codex(profile, force),
            Tool::Claude | Tool::Cursor => unsupported(tool),
        }
    }

    pub fn backups(&self, tool: Tool) -> Result<Vec<BackupEntry>> {
        match tool {
            Tool::Codex => self.list_tool_backups(tool),
            Tool::Claude | Tool::Cursor => unsupported(tool),
        }
    }

    pub fn restore(&self, tool: Tool, backup_id: &str) -> Result<RestoreResult> {
        validate_backup_id(backup_id)?;

        match tool {
            Tool::Codex => self.restore_codex(backup_id),
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
            Tool::Codex => self.exec_codex(profile, command),
            Tool::Claude | Tool::Cursor => unsupported(tool),
        }
    }

    fn save_codex(&self, profile: &str, force: bool) -> Result<SaveResult> {
        let source = codex_auth_path()?;
        ensure_file_exists(&source, "Codex auth file")?;

        let destination = self.profile_auth_path(Tool::Codex, profile);
        if destination.exists() && !force {
            bail!(
                "profile '{}' already exists for {}; pass --force to overwrite it",
                profile,
                Tool::Codex.key()
            );
        }

        self.create_private_store_dir_all(parent_dir(&destination)?)?;
        copy_file_private(&source, &destination)
            .with_context(|| format!("failed to save Codex auth profile '{}'", profile))?;

        Ok(SaveResult {
            tool: Tool::Codex,
            profile: profile.to_owned(),
            source,
            destination,
        })
    }

    fn use_codex(&self, profile: &str) -> Result<UseResult> {
        let source = self.profile_auth_path(Tool::Codex, profile);
        ensure_file_exists(&source, "saved Codex auth profile")?;

        let destination = codex_auth_path()?;
        create_private_dir_all(parent_dir(&destination)?)?;

        let backup = if destination.exists() {
            let backup = self.backup_path(Tool::Codex)?;
            self.create_private_store_dir_all(parent_dir(&backup)?)?;
            copy_file_private(&destination, &backup)
                .context("failed to back up current Codex auth file")?;
            Some(backup)
        } else {
            None
        };

        copy_file_private(&source, &destination)
            .with_context(|| format!("failed to switch Codex to profile '{}'", profile))?;

        Ok(UseResult {
            tool: Tool::Codex,
            profile: profile.to_owned(),
            source,
            destination,
            backup,
        })
    }

    fn list_tool_profiles(&self, tool: Tool) -> Result<Vec<String>> {
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
            let auth_path = entry.path().join(CODEX_AUTH_FILE);
            if auth_path.is_file() {
                profiles.push(name);
            }
        }

        profiles.sort();
        Ok(profiles)
    }

    fn current_codex(&self) -> Result<CurrentResult> {
        let active = codex_auth_path()?;
        if !active.is_file() {
            return Ok(CurrentResult {
                tool: Tool::Codex,
                state: CurrentState::Unknown,
            });
        }

        let active_auth = fs::read(&active)
            .with_context(|| format!("failed to read active Codex auth at {}", active.display()))?;

        let mut matches = Vec::new();
        for profile in self.list_tool_profiles(Tool::Codex)? {
            let profile_auth_path = self.profile_auth_path(Tool::Codex, &profile);
            let profile_auth = fs::read(&profile_auth_path).with_context(|| {
                format!(
                    "failed to read saved Codex profile '{}' at {}",
                    profile,
                    profile_auth_path.display()
                )
            })?;

            if profile_auth == active_auth {
                matches.push(profile);
            }
        }

        Ok(CurrentResult {
            tool: Tool::Codex,
            state: current_state(matches),
        })
    }

    fn remove_codex(&self, profile: &str, force: bool) -> Result<RemoveResult> {
        let profile_dir = self.tool_profiles_dir(Tool::Codex).join(profile);
        let auth_path = profile_dir.join(CODEX_AUTH_FILE);
        ensure_file_exists(&auth_path, "saved Codex auth profile")?;

        ensure_removable_profile(Tool::Codex, profile, &self.current_codex()?.state, force)?;

        fs::remove_dir_all(&profile_dir)
            .with_context(|| format!("failed to remove {}", profile_dir.display()))?;

        Ok(RemoveResult {
            tool: Tool::Codex,
            profile: profile.to_owned(),
            removed: profile_dir,
        })
    }

    fn restore_codex(&self, backup_id: &str) -> Result<RestoreResult> {
        let source = self.tool_backups_dir(Tool::Codex).join(backup_id);
        ensure_file_exists(&source, "Codex auth backup")?;

        let destination = codex_auth_path()?;
        create_private_dir_all(parent_dir(&destination)?)?;

        let backup = if destination.exists() {
            let backup = self.backup_path(Tool::Codex)?;
            self.create_private_store_dir_all(parent_dir(&backup)?)?;
            copy_file_private(&destination, &backup)
                .context("failed to back up current Codex auth file")?;
            Some(backup)
        } else {
            None
        };

        copy_file_private(&source, &destination)
            .with_context(|| format!("failed to restore Codex backup '{}'", backup_id))?;

        Ok(RestoreResult {
            tool: Tool::Codex,
            backup_id: backup_id.to_owned(),
            source,
            destination,
            backup,
        })
    }

    fn exec_codex(&self, profile: &str, command: &[OsString]) -> Result<ExecResult> {
        let source = self.profile_auth_path(Tool::Codex, profile);
        ensure_file_exists(&source, "saved Codex auth profile")?;

        let temp_dir = TempDir::create("aith-codex")?;
        let temp_codex_home = temp_dir.path();

        copy_file_private(&source, &temp_codex_home.join(CODEX_AUTH_FILE))
            .with_context(|| format!("failed to stage Codex profile '{}'", profile))?;

        let codex_config_path = codex_config_path()?;
        if codex_config_path.is_file() {
            copy_file_private(&codex_config_path, &temp_codex_home.join(CODEX_CONFIG_FILE))
                .context("failed to stage Codex config")?;
        }

        let status = Command::new(&command[0])
            .args(&command[1..])
            .env("CODEX_HOME", temp_codex_home)
            .status()
            .with_context(|| format!("failed to run command '{}'", command[0].to_string_lossy()))?;

        Ok(ExecResult {
            status_code: status_code(status),
        })
    }

    fn profile_auth_path(&self, tool: Tool, profile: &str) -> PathBuf {
        self.tool_profiles_dir(tool)
            .join(profile)
            .join(CODEX_AUTH_FILE)
    }

    fn tool_profiles_dir(&self, tool: Tool) -> PathBuf {
        self.root.join("profiles").join(tool.key())
    }

    fn list_tool_backups(&self, tool: Tool) -> Result<Vec<BackupEntry>> {
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

    fn tool_backups_dir(&self, tool: Tool) -> PathBuf {
        self.root.join("backups").join(tool.key())
    }

    fn backup_path(&self, tool: Tool) -> Result<PathBuf> {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .context("system clock is before UNIX epoch")?
            .as_secs();

        Ok(self
            .tool_backups_dir(tool)
            .join(format!("auth-{timestamp}-{}.json", std::process::id())))
    }

    fn create_private_store_dir_all(&self, path: &Path) -> Result<()> {
        if !path.starts_with(&self.root) {
            bail!(
                "refusing to create store path outside {}",
                self.root.display()
            );
        }

        fs::create_dir_all(path).with_context(|| format!("failed to create {}", path.display()))?;

        #[cfg(unix)]
        {
            secure_dir(&self.root)?;

            let relative = path
                .strip_prefix(&self.root)
                .with_context(|| format!("failed to inspect {}", path.display()))?;
            let mut current = self.root.clone();

            for component in relative.components() {
                current.push(component);
                secure_dir(&current)?;
            }
        }

        Ok(())
    }
}

fn codex_auth_path() -> Result<PathBuf> {
    Ok(codex_config_dir()?.join(CODEX_AUTH_FILE))
}

fn codex_config_path() -> Result<PathBuf> {
    Ok(codex_config_dir()?.join(CODEX_CONFIG_FILE))
}

fn codex_config_dir() -> Result<PathBuf> {
    let config_dir = env_path_or_home("CODEX_HOME", ".codex")
        .context("could not determine Codex config directory")?;

    Ok(config_dir)
}

fn validate_profile_name(profile: &str) -> Result<()> {
    if profile.is_empty() {
        bail!("profile name cannot be empty");
    }

    if profile.len() > 64 {
        bail!("profile name cannot be longer than 64 characters");
    }

    if !profile
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
    {
        bail!("profile names may only contain ASCII letters, numbers, '-' and '_'");
    }

    Ok(())
}

fn validate_backup_id(backup_id: &str) -> Result<()> {
    let Some(rest) = backup_id.strip_prefix("auth-") else {
        bail!("backup id must use the form auth-<timestamp>-<pid>.json");
    };

    let Some(rest) = rest.strip_suffix(".json") else {
        bail!("backup id must use the form auth-<timestamp>-<pid>.json");
    };

    let Some((timestamp, pid)) = rest.split_once('-') else {
        bail!("backup id must use the form auth-<timestamp>-<pid>.json");
    };

    if timestamp.is_empty() || pid.is_empty() {
        bail!("backup id must use the form auth-<timestamp>-<pid>.json");
    }

    if !timestamp.bytes().all(|byte| byte.is_ascii_digit()) {
        bail!("backup timestamp must contain only digits");
    }

    if !pid.bytes().all(|byte| byte.is_ascii_digit()) {
        bail!("backup process id must contain only digits");
    }

    Ok(())
}

fn ensure_file_exists(path: &Path, label: &str) -> Result<()> {
    if path.is_file() {
        Ok(())
    } else {
        bail!("{label} does not exist at {}", path.display());
    }
}

fn parent_dir(path: &Path) -> Result<&Path> {
    path.parent()
        .with_context(|| format!("path has no parent directory: {}", path.display()))
}

fn create_private_dir_all(path: &Path) -> Result<()> {
    fs::create_dir_all(path).with_context(|| format!("failed to create {}", path.display()))?;

    #[cfg(unix)]
    {
        secure_dir(path)?;
    }

    Ok(())
}

#[cfg(unix)]
fn secure_dir(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;

    fs::set_permissions(path, fs::Permissions::from_mode(0o700))
        .with_context(|| format!("failed to secure {}", path.display()))
}

fn copy_file_private(source: &Path, destination: &Path) -> Result<()> {
    let parent = parent_dir(destination)?;
    create_private_dir_all(parent)?;

    let tmp = parent.join(format!(
        ".{}.tmp-{}",
        destination
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("aith"),
        std::process::id()
    ));

    let copy_result = fs::copy(source, &tmp)
        .with_context(|| format!("failed to copy {} to {}", source.display(), tmp.display()));

    if let Err(error) = copy_result {
        let _ = fs::remove_file(&tmp);
        return Err(error);
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        if let Err(error) = fs::set_permissions(&tmp, fs::Permissions::from_mode(0o600)) {
            let _ = fs::remove_file(&tmp);
            return Err(error).with_context(|| format!("failed to secure {}", tmp.display()));
        }
    }

    if destination.exists() {
        fs::remove_file(destination)
            .with_context(|| format!("failed to replace {}", destination.display()))?;
    }

    fs::rename(&tmp, destination).with_context(|| {
        format!(
            "failed to move {} to {}",
            tmp.display(),
            destination.display()
        )
    })
}

#[derive(Debug)]
struct TempDir {
    path: PathBuf,
}

impl TempDir {
    fn create(prefix: &str) -> Result<Self> {
        let root = std::env::temp_dir();

        for attempt in 0..100 {
            let path = root.join(format!(
                "{}-{}-{}-{}",
                prefix,
                std::process::id(),
                unix_timestamp_nanos()?,
                attempt
            ));

            match fs::create_dir(&path) {
                Ok(()) => {
                    #[cfg(unix)]
                    secure_dir(&path)?;

                    return Ok(Self { path });
                }
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(error) => {
                    return Err(error)
                        .with_context(|| format!("failed to create {}", path.display()));
                }
            }
        }

        bail!("failed to create a unique temporary directory");
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn unix_timestamp_nanos() -> Result<u128> {
    Ok(SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .context("system clock is before UNIX epoch")?
        .as_nanos())
}

fn status_code(status: std::process::ExitStatus) -> i32 {
    if let Some(code) = status.code() {
        return code;
    }

    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;

        if let Some(signal) = status.signal() {
            return 128 + signal;
        }
    }

    1
}

fn unsupported<T>(tool: Tool) -> Result<T> {
    bail!(
        "{} profile switching is not implemented yet",
        tool.display_name()
    );
}

fn current_state(matches: Vec<String>) -> CurrentState {
    match matches.len() {
        0 => CurrentState::Unknown,
        1 => CurrentState::Known(matches.into_iter().next().expect("one profile match")),
        _ => CurrentState::Ambiguous(matches),
    }
}

fn ensure_removable_profile(
    tool: Tool,
    profile: &str,
    current: &CurrentState,
    force: bool,
) -> Result<()> {
    if force {
        return Ok(());
    }

    match current {
        CurrentState::Known(active_profile) if active_profile == profile => {
            bail!(
                "profile '{}' is currently active for {}; pass --force to remove it",
                profile,
                tool.key()
            );
        }
        CurrentState::Ambiguous(active_profiles)
            if active_profiles.iter().any(|active| active == profile) =>
        {
            bail!(
                "profile '{}' matches the active {} auth state; pass --force to remove it",
                profile,
                tool.key()
            );
        }
        CurrentState::Known(_) | CurrentState::Ambiguous(_) | CurrentState::Unknown => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        CurrentState, current_state, ensure_removable_profile, validate_backup_id,
        validate_profile_name,
    };
    use crate::tools::Tool;

    #[test]
    fn accepts_simple_profile_names() {
        assert!(validate_profile_name("work").is_ok());
        assert!(validate_profile_name("client-a").is_ok());
        assert!(validate_profile_name("client_a").is_ok());
        assert!(validate_profile_name("ClientA42").is_ok());
    }

    #[test]
    fn rejects_path_like_profile_names() {
        assert!(validate_profile_name("").is_err());
        assert!(validate_profile_name("../work").is_err());
        assert!(validate_profile_name("client.a").is_err());
        assert!(validate_profile_name("client/a").is_err());
        assert!(validate_profile_name("client a").is_err());
    }

    #[test]
    fn reports_current_state_from_profile_matches() {
        assert_eq!(current_state(Vec::new()), CurrentState::Unknown);
        assert_eq!(
            current_state(vec!["work".to_owned()]),
            CurrentState::Known("work".to_owned())
        );
        assert_eq!(
            current_state(vec!["personal".to_owned(), "work".to_owned()]),
            CurrentState::Ambiguous(vec!["personal".to_owned(), "work".to_owned()])
        );
    }

    #[test]
    fn blocks_removing_current_profile_without_force() {
        assert!(
            ensure_removable_profile(
                Tool::Codex,
                "work",
                &CurrentState::Known("work".to_owned()),
                false
            )
            .is_err()
        );
        assert!(
            ensure_removable_profile(
                Tool::Codex,
                "work",
                &CurrentState::Ambiguous(vec!["personal".to_owned(), "work".to_owned()]),
                false
            )
            .is_err()
        );
        assert!(
            ensure_removable_profile(
                Tool::Codex,
                "work",
                &CurrentState::Known("work".to_owned()),
                true
            )
            .is_ok()
        );
        assert!(
            ensure_removable_profile(
                Tool::Codex,
                "old",
                &CurrentState::Known("work".to_owned()),
                false
            )
            .is_ok()
        );
    }

    #[test]
    fn validates_generated_backup_ids() {
        assert!(validate_backup_id("auth-1778702155-74626.json").is_ok());
        assert!(validate_backup_id("auth-1778702155.json").is_err());
        assert!(validate_backup_id("auth-1778702155-74626").is_err());
        assert!(validate_backup_id("auth-1778702155-pid.json").is_err());
        assert!(validate_backup_id("../auth-1778702155-74626.json").is_err());
    }
}
