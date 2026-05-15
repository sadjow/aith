use std::path::PathBuf;

pub(crate) mod claude;
pub(crate) mod codex;
pub(crate) mod cursor;
pub(crate) mod desktop;
pub(crate) mod env_session;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Tool {
    CodexCli,
    CodexDesktop,
    ClaudeCode,
    ClaudeDesktop,
    CursorAgent,
    CursorDesktop,
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
        &[
            Tool::CodexCli,
            Tool::CodexDesktop,
            Tool::ClaudeCode,
            Tool::ClaudeDesktop,
            Tool::CursorAgent,
            Tool::CursorDesktop,
        ]
    }

    pub fn key(self) -> &'static str {
        match self {
            Tool::CodexCli => "codex-cli",
            Tool::CodexDesktop => "codex-desktop",
            Tool::ClaudeCode => "claude-code",
            Tool::ClaudeDesktop => "claude-desktop",
            Tool::CursorAgent => "cursor-agent",
            Tool::CursorDesktop => "cursor-desktop",
        }
    }

    pub fn storage_key(self) -> &'static str {
        match self {
            Tool::CodexCli => "codex",
            Tool::CodexDesktop => "codex-desktop",
            Tool::ClaudeCode => "claude",
            Tool::ClaudeDesktop => "claude-desktop",
            Tool::CursorAgent => "cursor",
            Tool::CursorDesktop => "cursor-desktop",
        }
    }

    pub fn display_name(self) -> &'static str {
        match self {
            Tool::CodexCli => "Codex CLI",
            Tool::CodexDesktop => "Codex Desktop",
            Tool::ClaudeCode => "Claude Code",
            Tool::ClaudeDesktop => "Claude Desktop",
            Tool::CursorAgent => "Cursor Agent",
            Tool::CursorDesktop => "Cursor Desktop",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            Tool::CodexCli => "OpenAI Codex CLI auth and config",
            Tool::CodexDesktop => "Codex desktop app auth and app data",
            Tool::ClaudeCode => "Claude Code config, keychain, and API-key auth",
            Tool::ClaudeDesktop => "Claude desktop app auth and app data",
            Tool::CursorAgent => "Cursor Agent API-key auth and Cursor user data",
            Tool::CursorDesktop => "Cursor desktop app auth and user data",
        }
    }

    pub fn inspect(self) -> ToolStatus {
        match self {
            Tool::CodexCli => codex::inspect(),
            Tool::CodexDesktop => desktop::inspect_codex(),
            Tool::ClaudeCode => claude::inspect(),
            Tool::ClaudeDesktop => desktop::inspect_claude(),
            Tool::CursorAgent => cursor::inspect(),
            Tool::CursorDesktop => desktop::inspect_cursor(),
        }
    }

    pub fn is_desktop(self) -> bool {
        matches!(
            self,
            Tool::CodexDesktop | Tool::ClaudeDesktop | Tool::CursorDesktop
        )
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
