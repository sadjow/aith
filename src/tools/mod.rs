use std::path::PathBuf;

pub(crate) mod claude;
pub(crate) mod codex;
pub(crate) mod cursor;
pub(crate) mod env_session;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Tool {
    CodexCli,
    ClaudeCode,
    CursorAgent,
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
        &[Tool::CodexCli, Tool::ClaudeCode, Tool::CursorAgent]
    }

    pub fn key(self) -> &'static str {
        match self {
            Tool::CodexCli => "codex-cli",
            Tool::ClaudeCode => "claude-code",
            Tool::CursorAgent => "cursor-agent",
        }
    }

    pub fn storage_key(self) -> &'static str {
        match self {
            Tool::CodexCli => "codex",
            Tool::ClaudeCode => "claude",
            Tool::CursorAgent => "cursor",
        }
    }

    pub fn display_name(self) -> &'static str {
        match self {
            Tool::CodexCli => "Codex CLI",
            Tool::ClaudeCode => "Claude Code",
            Tool::CursorAgent => "Cursor Agent",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            Tool::CodexCli => "OpenAI Codex CLI auth and config",
            Tool::ClaudeCode => "Claude Code config, keychain, and API-key auth",
            Tool::CursorAgent => "Cursor Agent API-key auth and Cursor user data",
        }
    }

    pub fn inspect(self) -> ToolStatus {
        match self {
            Tool::CodexCli => codex::inspect(),
            Tool::ClaudeCode => claude::inspect(),
            Tool::CursorAgent => cursor::inspect(),
        }
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
