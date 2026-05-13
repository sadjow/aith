use std::env;
use std::path::PathBuf;

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
            Tool::Codex => inspect_codex(),
            Tool::Claude => inspect_claude(),
            Tool::Cursor => inspect_cursor(),
        }
    }
}

fn inspect_codex() -> ToolStatus {
    let config_dir = env_path_or_home("CODEX_HOME", ".codex");

    ToolStatus {
        tool: Tool::Codex,
        paths: vec![
            path_check("config dir", config_dir.clone()),
            path_check(
                "auth file",
                config_dir.as_ref().map(|path| path.join("auth.json")),
            ),
            path_check(
                "config file",
                config_dir.as_ref().map(|path| path.join("config.toml")),
            ),
        ],
        env: env_checks(&["CODEX_HOME", "OPENAI_API_KEY", "CODEX_ACCESS_TOKEN"]),
        notes: vec!["checks file presence only; credential contents are never read"],
    }
}

fn inspect_claude() -> ToolStatus {
    let config_dir = env_path_or_home("CLAUDE_CONFIG_DIR", ".claude");
    let home = home_dir();

    ToolStatus {
        tool: Tool::Claude,
        paths: vec![
            path_check("config dir", config_dir.clone()),
            path_check(
                "settings file",
                config_dir.as_ref().map(|path| path.join("settings.json")),
            ),
            path_check(
                "user state",
                home.as_ref().map(|path| path.join(".claude.json")),
            ),
        ],
        env: env_checks(&[
            "CLAUDE_CONFIG_DIR",
            "ANTHROPIC_API_KEY",
            "CLAUDE_CODE_SIMPLE",
        ]),
        notes: vec![
            "OAuth/keychain state is tool-managed and is not inspected",
            "bare/API-key sessions can avoid persistent OAuth state",
        ],
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
            is_set: env::var_os(name).is_some(),
        })
        .collect()
}

fn env_path_or_home(env_name: &str, home_relative: &str) -> Option<PathBuf> {
    env::var_os(env_name)
        .map(PathBuf::from)
        .or_else(|| home_dir().map(|home| home.join(home_relative)))
}

fn home_dir() -> Option<PathBuf> {
    env::var_os("HOME").map(PathBuf::from)
}

fn cursor_user_dir() -> Option<PathBuf> {
    if cfg!(target_os = "macos") {
        home_dir().map(|home| home.join("Library/Application Support/Cursor/User"))
    } else if cfg!(target_os = "windows") {
        env::var_os("APPDATA")
            .map(PathBuf::from)
            .map(|app_data| app_data.join("Cursor").join("User"))
    } else {
        env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|| home_dir().map(|home| home.join(".config")))
            .map(|config| config.join("Cursor").join("User"))
    }
}
