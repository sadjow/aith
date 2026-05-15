use std::ffi::OsString;

use anyhow::Result;
use clap::{Parser, Subcommand, ValueEnum};

use crate::profiles::{
    BackupEntry, CurrentResult, CurrentState, ProfileStore, RemoveResult, RestoreResult,
    SaveResult, UseResult,
};
use crate::tools::{Tool, ToolStatus};

#[derive(Debug, Parser)]
#[command(name = "aith")]
#[command(about = "Account profile switching for AI coding tools")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Save the current auth state as a named profile.
    #[command(alias = "add")]
    Save {
        /// Tool whose current auth state should be saved.
        #[arg(value_enum)]
        tool: ToolArg,

        /// Profile name to create.
        profile: String,

        /// Overwrite the profile if it already exists.
        #[arg(long, short)]
        force: bool,
    },

    /// Switch a tool to a saved profile.
    Use {
        /// Tool to switch.
        #[arg(value_enum)]
        tool: ToolArg,

        /// Profile name to use.
        profile: String,
    },

    /// List saved profiles.
    List {
        /// Tool whose profiles should be listed.
        #[arg(value_enum)]
        tool: ToolArg,
    },

    /// Remove a saved profile.
    Remove {
        /// Tool whose profile should be removed.
        #[arg(value_enum)]
        tool: ToolArg,

        /// Profile name to remove.
        profile: String,

        /// Remove the profile even if it matches the active auth state.
        #[arg(long, short)]
        force: bool,
    },

    /// List auth backups created before profile switches or restores.
    Backups {
        /// Tool whose backups should be listed.
        #[arg(value_enum)]
        tool: ToolArg,
    },

    /// Restore an auth backup.
    Restore {
        /// Tool whose backup should be restored.
        #[arg(value_enum)]
        tool: ToolArg,

        /// Backup id from `aith backups <tool>`.
        backup_id: String,
    },

    /// Run a command with a temporary profile-scoped auth environment.
    Exec {
        /// Tool whose profile should be used.
        #[arg(value_enum)]
        tool: ToolArg,

        /// Profile name to use for this command.
        profile: String,

        /// Command to run after `--`.
        #[arg(required = true, trailing_var_arg = true, allow_hyphen_values = true)]
        command: Vec<OsString>,
    },

    /// Show which saved profile matches the active auth state.
    Current {
        /// Tool whose active profile should be detected.
        #[arg(value_enum)]
        tool: ToolArg,
    },

    /// Show auth/config status for supported tools.
    Status {
        /// Limit the status output to one tool.
        #[arg(value_enum)]
        tool: Option<ToolArg>,
    },

    /// List supported tools.
    Tools,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum ToolArg {
    Codex,
    Claude,
    Cursor,
}

impl From<ToolArg> for Tool {
    fn from(value: ToolArg) -> Self {
        match value {
            ToolArg::Codex => Tool::Codex,
            ToolArg::Claude => Tool::Claude,
            ToolArg::Cursor => Tool::Cursor,
        }
    }
}

pub fn run() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::Save {
            tool,
            profile,
            force,
        } => {
            let store = ProfileStore::new()?;
            let result = store.save(tool.into(), &profile, force)?;
            print_save_result(&result);
        }
        Command::Use { tool, profile } => {
            let store = ProfileStore::new()?;
            let result = store.use_profile(tool.into(), &profile)?;
            print_use_result(&result);
        }
        Command::List { tool } => {
            let tool = Tool::from(tool);
            let store = ProfileStore::new()?;
            let profiles = store.list(tool)?;

            if profiles.is_empty() {
                println!(
                    "no {} profiles saved in {}",
                    tool.key(),
                    store.root().display()
                );
            } else {
                for profile in profiles {
                    println!("{profile}");
                }
            }
        }
        Command::Remove {
            tool,
            profile,
            force,
        } => {
            let store = ProfileStore::new()?;
            let result = store.remove(tool.into(), &profile, force)?;
            print_remove_result(&result);
        }
        Command::Backups { tool } => {
            let tool = Tool::from(tool);
            let store = ProfileStore::new()?;
            let backups = store.backups(tool)?;

            if backups.is_empty() {
                println!(
                    "no {} backups saved in {}",
                    tool.key(),
                    store.root().display()
                );
            } else {
                print_backups(&backups);
            }
        }
        Command::Restore { tool, backup_id } => {
            let store = ProfileStore::new()?;
            let result = store.restore(tool.into(), &backup_id)?;
            print_restore_result(&result);
        }
        Command::Exec {
            tool,
            profile,
            command,
        } => {
            let store = ProfileStore::new()?;
            let result = store.exec_profile(tool.into(), &profile, &command)?;
            std::process::exit(result.status_code);
        }
        Command::Current { tool } => {
            let store = ProfileStore::new()?;
            let result = store.current(tool.into())?;
            print_current_result(&result);
        }
        Command::Status { tool } => {
            let tools = match tool {
                Some(tool) => vec![tool.into()],
                None => Tool::all().to_vec(),
            };

            for (index, tool) in tools.iter().enumerate() {
                if index > 0 {
                    println!();
                }

                print_status(&tool.inspect());
            }
        }
        Command::Tools => {
            for tool in Tool::all() {
                println!("{:<8} {}", tool.key(), tool.description());
            }
        }
    }

    Ok(())
}

fn print_save_result(result: &SaveResult) {
    println!("saved {} profile '{}'", result.tool.key(), result.profile);
    println!("  source      {}", result.source.display());
    println!("  destination {}", result.destination.display());
}

fn print_use_result(result: &UseResult) {
    println!(
        "switched {} to profile '{}'",
        result.tool.key(),
        result.profile
    );
    println!("  source      {}", result.source.display());
    println!("  destination {}", result.destination.display());

    if let Some(backup) = &result.backup {
        println!("  backup      {}", backup.display());
    }
}

fn print_remove_result(result: &RemoveResult) {
    println!("removed {} profile '{}'", result.tool.key(), result.profile);
    println!("  removed {}", result.removed.display());
}

fn print_backups(backups: &[BackupEntry]) {
    for backup in backups {
        println!("{:<32} {}", backup.id, backup.path.display());
    }
}

fn print_restore_result(result: &RestoreResult) {
    println!(
        "restored {} backup '{}'",
        result.tool.key(),
        result.backup_id
    );
    println!("  source      {}", result.source.display());
    println!("  destination {}", result.destination.display());

    if let Some(backup) = &result.backup {
        println!("  backup      {}", backup.display());
    }
}

fn print_current_result(result: &CurrentResult) {
    match &result.state {
        CurrentState::Known(profile) => println!("{}: {profile}", result.tool.key()),
        CurrentState::Ambiguous(profiles) => {
            println!("{}: ambiguous", result.tool.key());
            println!("  matches {}", profiles.join(", "));
        }
        CurrentState::Unknown => println!("{}: unknown", result.tool.key()),
    }
}

fn print_status(status: &ToolStatus) {
    println!("{} ({})", status.tool.display_name(), status.tool.key());

    for check in &status.paths {
        let state = if check.exists { "found" } else { "missing" };

        match &check.path {
            Some(path) => println!("  {:<18} {:<7} {}", check.label, state, path.display()),
            None => println!("  {:<18} unknown", check.label),
        }
    }

    for env in &status.env {
        let state = if env.is_set { "set" } else { "unset" };
        println!("  env {:<14} {}", env.name, state);
    }

    for note in &status.notes {
        println!("  note              {note}");
    }
}
