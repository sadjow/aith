use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitStatus;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result, bail};

pub(crate) fn ensure_file_exists(path: &Path, label: &str) -> Result<()> {
    if path.is_file() {
        Ok(())
    } else {
        bail!("{label} does not exist at {}", path.display());
    }
}

pub(crate) fn parent_dir(path: &Path) -> Result<&Path> {
    path.parent()
        .with_context(|| format!("path has no parent directory: {}", path.display()))
}

pub(crate) fn create_private_dir_all(path: &Path) -> Result<()> {
    fs::create_dir_all(path).with_context(|| format!("failed to create {}", path.display()))?;

    #[cfg(unix)]
    {
        secure_dir(path)?;
    }

    Ok(())
}

#[cfg(unix)]
pub(crate) fn secure_dir(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;

    fs::set_permissions(path, fs::Permissions::from_mode(0o700))
        .with_context(|| format!("failed to secure {}", path.display()))
}

pub(crate) fn copy_file_private(source: &Path, destination: &Path) -> Result<()> {
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

pub(crate) fn write_file_private(destination: &Path, contents: &[u8]) -> Result<()> {
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

    let write_result =
        fs::write(&tmp, contents).with_context(|| format!("failed to write {}", tmp.display()));

    if let Err(error) = write_result {
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
pub(crate) struct TempDir {
    path: PathBuf,
}

impl TempDir {
    pub(crate) fn create(prefix: &str) -> Result<Self> {
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

    pub(crate) fn path(&self) -> &Path {
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

pub(crate) fn status_code(status: ExitStatus) -> i32 {
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
