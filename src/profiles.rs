use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result, bail};

use crate::paths::{aith_data_dir, env_path_or_home};
use crate::tools::Tool;

const CODEX_AUTH_FILE: &str = "auth.json";

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

impl ProfileStore {
    pub fn new() -> Result<Self> {
        let root = aith_data_dir().context("could not determine aith data directory")?;
        Ok(Self { root })
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

    fn profile_auth_path(&self, tool: Tool, profile: &str) -> PathBuf {
        self.tool_profiles_dir(tool)
            .join(profile)
            .join(CODEX_AUTH_FILE)
    }

    fn tool_profiles_dir(&self, tool: Tool) -> PathBuf {
        self.root.join("profiles").join(tool.key())
    }

    fn backup_path(&self, tool: Tool) -> Result<PathBuf> {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .context("system clock is before UNIX epoch")?
            .as_secs();

        Ok(self
            .root
            .join("backups")
            .join(tool.key())
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
    let config_dir = env_path_or_home("CODEX_HOME", ".codex")
        .context("could not determine Codex config directory")?;

    Ok(config_dir.join(CODEX_AUTH_FILE))
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

fn unsupported<T>(tool: Tool) -> Result<T> {
    bail!(
        "{} profile switching is not implemented yet",
        tool.display_name()
    );
}

#[cfg(test)]
mod tests {
    use super::validate_profile_name;

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
}
