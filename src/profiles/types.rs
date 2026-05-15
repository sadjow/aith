use std::path::PathBuf;

use crate::tools::Tool;

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
