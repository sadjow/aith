use std::path::PathBuf;

use anyhow::Result;

use crate::profiles::{CurrentState, ProfileStore};
use crate::tools::{Tool, ToolStatus, claude, cursor};

#[derive(Debug)]
pub struct DoctorReport {
    pub store_root: PathBuf,
    pub tools: Vec<ToolDoctor>,
}

#[derive(Debug)]
pub struct ToolDoctor {
    pub tool: Tool,
    pub status: ToolStatus,
    pub profiles: DoctorProfileSummary,
    pub findings: Vec<DoctorFinding>,
}

#[derive(Debug)]
pub enum DoctorProfileSummary {
    Supported {
        profile_count: usize,
        backup_count: usize,
        current: DoctorCurrent,
    },
    Unsupported,
}

#[derive(Debug)]
pub enum DoctorCurrent {
    Known(String),
    Ambiguous(Vec<String>),
    Unknown,
}

#[derive(Debug)]
pub struct DoctorFinding {
    pub severity: DoctorSeverity,
    pub message: String,
}

#[derive(Debug)]
pub enum DoctorSeverity {
    Ok,
    Info,
    Warning,
}

pub fn diagnose(store: &ProfileStore, tools: &[Tool]) -> Result<DoctorReport> {
    let tools = tools
        .iter()
        .copied()
        .map(|tool| diagnose_tool(store, tool))
        .collect::<Result<Vec<_>>>()?;

    Ok(DoctorReport {
        store_root: store.root().to_owned(),
        tools,
    })
}

fn diagnose_tool(store: &ProfileStore, tool: Tool) -> Result<ToolDoctor> {
    match tool {
        Tool::Codex => diagnose_codex(store),
        Tool::Claude => diagnose_claude(store),
        Tool::Cursor => diagnose_cursor(store),
    }
}

fn diagnose_codex(store: &ProfileStore) -> Result<ToolDoctor> {
    let tool = Tool::Codex;
    let status = tool.inspect();
    let profiles = store.list(tool)?;
    let backups = store.backups(tool)?;
    let current = store.current(tool)?;
    let current = DoctorCurrent::from(current.state);
    let mut findings = Vec::new();

    if !path_exists(&status, "auth file") {
        findings.push(DoctorFinding::warning(
            "active Codex auth file is missing; run `codex` login before saving a profile",
        ));
    }

    if profiles.is_empty() {
        findings.push(DoctorFinding::warning(
            "no Codex profiles are saved; run `aith save codex <profile>` to create one",
        ));
    }

    match &current {
        DoctorCurrent::Known(_) => {}
        DoctorCurrent::Unknown if !profiles.is_empty() && path_exists(&status, "auth file") => {
            findings.push(DoctorFinding::warning(
                "active Codex auth does not match a saved profile",
            ));
        }
        DoctorCurrent::Ambiguous(_) => {
            findings.push(DoctorFinding::warning(
                "active Codex auth matches multiple saved profiles",
            ));
        }
        DoctorCurrent::Unknown => {}
    }

    if findings.is_empty() {
        findings.push(DoctorFinding::ok("Codex profile switching is ready"));
    }

    Ok(ToolDoctor {
        tool,
        status,
        profiles: DoctorProfileSummary::Supported {
            profile_count: profiles.len(),
            backup_count: backups.len(),
            current,
        },
        findings,
    })
}

fn diagnose_claude(store: &ProfileStore) -> Result<ToolDoctor> {
    let tool = Tool::Claude;
    let status = tool.inspect();
    let profiles = store.list(tool)?;
    let backups = store.backups(tool)?;
    let current = DoctorCurrent::from(store.current(tool)?.state);
    let mut findings = Vec::new();
    let has_terminal_auth_env = claude::has_terminal_auth_env();
    let has_path_credentials =
        claude::credentials_are_path_backed() && path_exists(&status, claude::CREDENTIALS_LABEL);

    if has_terminal_auth_env {
        findings.push(DoctorFinding::info(
            "Claude terminal auth environment is configured",
        ));
    }

    if claude::credentials_are_path_backed() {
        if has_path_credentials {
            findings.push(DoctorFinding::info("Claude credentials file was found"));
        } else if !has_terminal_auth_env {
            findings.push(DoctorFinding::warning(
                "no Claude terminal auth env or credentials file found; run `claude` and use `/login`, or set a terminal auth env var",
            ));
        }
    } else {
        findings.push(DoctorFinding::info(
            "Claude subscription credentials may exist in macOS Keychain; aith cannot inspect Keychain safely",
        ));
    }

    if !path_exists(&status, claude::CONFIG_DIR_LABEL) {
        findings.push(DoctorFinding::info(
            "Claude user config directory has not been created yet",
        ));
    }

    if profiles.is_empty() {
        findings.push(DoctorFinding::info(
            "no Claude env profiles are saved; run `aith save claude <profile> --from-env ANTHROPIC_API_KEY=SOURCE_ENV` to create one",
        ));
    } else {
        findings.push(DoctorFinding::info(
            "Claude env session profiles are available for exec and shell",
        ));
    }

    findings.push(DoctorFinding::warning(
        "Claude global login switching is not implemented; env profiles support exec and shell only",
    ));

    Ok(ToolDoctor {
        tool,
        status,
        profiles: DoctorProfileSummary::Supported {
            profile_count: profiles.len(),
            backup_count: backups.len(),
            current,
        },
        findings,
    })
}

fn diagnose_cursor(store: &ProfileStore) -> Result<ToolDoctor> {
    let tool = Tool::Cursor;
    let status = tool.inspect();
    let profiles = store.list(tool)?;
    let backups = store.backups(tool)?;
    let current = DoctorCurrent::from(store.current(tool)?.state);
    let mut findings = Vec::new();

    if cursor::has_terminal_auth_env() {
        findings.push(DoctorFinding::info(
            "Cursor terminal auth environment is configured",
        ));
    }

    if profiles.is_empty() {
        findings.push(DoctorFinding::info(
            "no Cursor env profiles are saved; run `aith save cursor <profile> --from-env CURSOR_API_KEY=SOURCE_ENV` to create one",
        ));
    } else {
        findings.push(DoctorFinding::info(
            "Cursor env session profiles are available for exec and shell",
        ));
    }

    findings.push(DoctorFinding::warning(
        "Cursor global login switching is not implemented; env profiles support exec and shell only",
    ));

    Ok(ToolDoctor {
        tool,
        status,
        profiles: DoctorProfileSummary::Supported {
            profile_count: profiles.len(),
            backup_count: backups.len(),
            current,
        },
        findings,
    })
}

fn path_exists(status: &ToolStatus, label: &str) -> bool {
    status
        .paths
        .iter()
        .any(|check| check.label == label && check.exists)
}

impl From<CurrentState> for DoctorCurrent {
    fn from(value: CurrentState) -> Self {
        match value {
            CurrentState::Known(profile) => Self::Known(profile),
            CurrentState::Ambiguous(profiles) => Self::Ambiguous(profiles),
            CurrentState::Unknown => Self::Unknown,
        }
    }
}

impl DoctorFinding {
    fn ok(message: impl Into<String>) -> Self {
        Self {
            severity: DoctorSeverity::Ok,
            message: message.into(),
        }
    }

    fn warning(message: impl Into<String>) -> Self {
        Self {
            severity: DoctorSeverity::Warning,
            message: message.into(),
        }
    }

    fn info(message: impl Into<String>) -> Self {
        Self {
            severity: DoctorSeverity::Info,
            message: message.into(),
        }
    }
}
