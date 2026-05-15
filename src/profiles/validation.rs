use anyhow::{Result, bail};

use crate::profiles::CurrentState;
use crate::tools::Tool;

pub(crate) fn validate_profile_name(profile: &str) -> Result<()> {
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

pub(crate) fn validate_backup_id(backup_id: &str) -> Result<()> {
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

pub(crate) fn current_state(matches: Vec<String>) -> CurrentState {
    match matches.len() {
        0 => CurrentState::Unknown,
        1 => CurrentState::Known(matches.into_iter().next().expect("one profile match")),
        _ => CurrentState::Ambiguous(matches),
    }
}

pub(crate) fn ensure_removable_profile(
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
        current_state, ensure_removable_profile, validate_backup_id, validate_profile_name,
    };
    use crate::profiles::CurrentState;
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
                Tool::CodexCli,
                "work",
                &CurrentState::Known("work".to_owned()),
                false
            )
            .is_err()
        );
        assert!(
            ensure_removable_profile(
                Tool::CodexCli,
                "work",
                &CurrentState::Ambiguous(vec!["personal".to_owned(), "work".to_owned()]),
                false
            )
            .is_err()
        );
        assert!(
            ensure_removable_profile(
                Tool::CodexCli,
                "work",
                &CurrentState::Known("work".to_owned()),
                true
            )
            .is_ok()
        );
        assert!(
            ensure_removable_profile(
                Tool::CodexCli,
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
