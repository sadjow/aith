use std::path::PathBuf;

use crate::paths::cursor_user_dir;

pub(crate) mod claude;
pub(crate) mod codex;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Tool {
    Codex,
    Claude,
    Cursor,
}

#[derive(Debug)]
pub struct ToolStatus {
    pub tool: Tool,
    pub paths: Vec<PathCheck>,
    pub env: Vec<EnvCheck>,
    pub notes: Vec<&'static str>,
}

#[derive(Debug)]
pub struct PathCheck {
    pub label: &'static str,
    pub path: Option<PathBuf>,
    pub exists: bool,
}

#[derive(Debug)]
pub struct EnvCheck {
    pub name: &'static str,
    pub is_set: bool,
}

impl Tool {
    pub fn all() -> &'static [Tool] {
        &[Tool::Codex, Tool::Claude, Tool::Cursor]
    }

    pub fn key(self) -> &'static str {
        match self {
            Tool::Codex => "codex",
            Tool::Claude => "claude",
            Tool::Cursor => "cursor",
        }
    }

    pub fn display_name(self) -> &'static str {
        match self {
            Tool::Codex => "Codex",
            Tool::Claude => "Claude Code",
            Tool::Cursor => "Cursor",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            Tool::Codex => "OpenAI Codex CLI auth and config",
            Tool::Claude => "Claude Code config, keychain, and API-key auth",
            Tool::Cursor => "Cursor Agent API-key auth and Cursor user data",
        }
    }

    pub fn inspect(self) -> ToolStatus {
        match self {
            Tool::Codex => codex::inspect(),
            Tool::Claude => claude::inspect(),
            Tool::Cursor => inspect_cursor(),
        }
    }
}

fn inspect_cursor() -> ToolStatus {
    ToolStatus {
        tool: Tool::Cursor,
        paths: vec![path_check("user data", cursor_user_dir())],
        env: env_checks(&["CURSOR_API_KEY"]),
        notes: vec!["Cursor Agent supports session auth through CURSOR_API_KEY or --api-key"],
    }
}

fn path_check(label: &'static str, path: Option<PathBuf>) -> PathCheck {
    let exists = path.as_ref().is_some_and(|path| path.exists());

    PathCheck {
        label,
        path,
        exists,
    }
}

fn env_checks(names: &[&'static str]) -> Vec<EnvCheck> {
    names
        .iter()
        .map(|name| EnvCheck {
            name,
            is_set: std::env::var_os(name).is_some(),
        })
        .collect()
}
