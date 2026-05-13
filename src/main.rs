use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "aith")]
#[command(about = "Account profile switching for AI coding tools")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Show the current project status.
    Status,
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Command::Status => {
            println!("aith is initialized");
        }
    }
}
