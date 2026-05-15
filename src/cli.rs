use anyhow::Result;
use clap::{Parser, Subcommand, ValueEnum};

use crate::profiles::{CurrentResult, CurrentState, ProfileStore, SaveResult, UseResult};
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
