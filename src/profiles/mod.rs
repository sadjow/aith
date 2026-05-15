mod env_profile;
mod fs;
mod store;
mod types;
mod validation;

pub use env_profile::{EnvProfileMapping, EnvProfileSpec};
pub use store::ProfileStore;
pub use types::{
    BackupEntry, CurrentResult, CurrentState, ExecResult, RemoveResult, RestoreResult, SaveResult,
    ShellResult, UseResult,
};

pub(crate) use env_profile::EnvProfileFile;
pub(crate) use fs::{
    TempDir, copy_file_private, create_private_dir_all, ensure_file_exists, parent_dir,
    status_code, write_file_private,
};
pub(crate) use validation::{
    current_state, ensure_removable_profile, validate_backup_id, validate_profile_name,
};
