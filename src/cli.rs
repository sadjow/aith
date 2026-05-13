use anyhow::Result;
use clap::{Parser, Subcommand, ValueEnum};

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
