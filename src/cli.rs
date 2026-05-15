use std::ffi::OsString;
use std::str::FromStr;

use anyhow::{Result, bail};
use clap::{Parser, Subcommand, ValueEnum};

use crate::doctor::{
    DoctorCurrent, DoctorProfileSummary, DoctorReport, DoctorSeverity, ToolDoctor,
};
use crate::profiles::{
    BackupEntry, CurrentResult, CurrentState, EnvProfileMapping, EnvProfileSpec, ProfileStore,
    RemoveResult, RestoreResult, SaveResult, UseResult,
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

        /// Save TARGET env by resolving it from SOURCE env at session start.
        #[arg(long = "from-env", value_name = "TARGET=SOURCE")]
        from_env: Vec<EnvMappingArg>,

        /// Save a literal non-secret env value in the profile.
        #[arg(long = "set-env", value_name = "NAME=VALUE")]
        set_env: Vec<EnvMappingArg>,
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

    /// Start a shell with a temporary profile-scoped auth environment.
    Shell {
        /// Tool whose profile should be used.
        #[arg(value_enum)]
        tool: ToolArg,

        /// Profile name to use for this shell session.
        profile: String,
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

    /// Diagnose auth/profile readiness without printing credentials.
    Doctor {
        /// Limit diagnostics to one tool.
        #[arg(value_enum)]
        tool: Option<ToolArg>,
    },

    /// List supported tools.
    Tools,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum ToolArg {
    #[value(name = "codex-cli", alias = "codex")]
    CodexCli,
    #[value(name = "claude-code", alias = "claude")]
    ClaudeCode,
    #[value(name = "cursor-agent", alias = "cursor")]
    CursorAgent,
}

#[derive(Clone, Debug)]
struct EnvMappingArg {
    name: String,
    value: String,
}

impl FromStr for EnvMappingArg {
    type Err = String;

    fn from_str(value: &str) -> std::result::Result<Self, Self::Err> {
        let Some((name, mapped_value)) = value.split_once('=') else {
            return Err("expected NAME=VALUE".to_owned());
        };

        if name.is_empty() || mapped_value.is_empty() {
            return Err("expected NAME=VALUE with both sides set".to_owned());
        }

        Ok(Self {
            name: name.to_owned(),
            value: mapped_value.to_owned(),
        })
    }
}

impl From<ToolArg> for Tool {
    fn from(value: ToolArg) -> Self {
        match value {
            ToolArg::CodexCli => Tool::CodexCli,
            ToolArg::ClaudeCode => Tool::ClaudeCode,
            ToolArg::CursorAgent => Tool::CursorAgent,
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
            from_env,
            set_env,
        } => {
            let tool = Tool::from(tool);
            let store = ProfileStore::new()?;
            let result = if from_env.is_empty() && set_env.is_empty() {
                if matches!(tool, Tool::ClaudeCode | Tool::CursorAgent) {
                    bail!(
                        "{} profiles are env-based; pass --from-env TARGET=SOURCE or --set-env NAME=VALUE",
                        tool.display_name()
                    );
                }
                store.save(tool, &profile, force)?
            } else {
                store.save_env(tool, &profile, force, env_profile_spec(from_env, set_env)?)?
            };
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
        Command::Shell { tool, profile } => {
            let tool = Tool::from(tool);
            let store = ProfileStore::new()?;
            eprintln!("starting {} shell with profile '{}'", tool.key(), profile);
            eprintln!("exit the shell to end the temporary session");
            let result = store.shell_profile(tool, &profile)?;
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
        Command::Doctor { tool } => {
            let tools = match tool {
                Some(tool) => vec![tool.into()],
                None => Tool::all().to_vec(),
            };
            let store = ProfileStore::new()?;
            let report = crate::doctor::diagnose(&store, &tools)?;
            print_doctor_report(&report);
        }
        Command::Tools => {
            for tool in Tool::all() {
                println!("{:<14} {}", tool.key(), tool.description());
            }
        }
    }

    Ok(())
}

fn print_save_result(result: &SaveResult) {
    println!("saved {} profile '{}'", result.tool.key(), result.profile);
    match &result.source {
        Some(source) => println!("  source      {}", source.display()),
        None => println!("  source      environment"),
    }
    println!("  destination {}", result.destination.display());
}

fn env_profile_spec(
    from_env: Vec<EnvMappingArg>,
    set_env: Vec<EnvMappingArg>,
) -> Result<EnvProfileSpec> {
    Ok(EnvProfileSpec::new(
        from_env
            .into_iter()
            .map(|mapping| EnvProfileMapping::new(mapping.name, mapping.value))
            .collect(),
        set_env
            .into_iter()
            .map(|mapping| EnvProfileMapping::new(mapping.name, mapping.value))
            .collect(),
    ))
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
    print_status_details(status);
}

fn print_status_details(status: &ToolStatus) {
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

fn print_doctor_report(report: &DoctorReport) {
    println!("aith store {}", report.store_root.display());

    for (index, tool) in report.tools.iter().enumerate() {
        if index > 0 {
            println!();
        }

        print_tool_doctor(tool);
    }
}

fn print_tool_doctor(doctor: &ToolDoctor) {
    println!("{} ({})", doctor.tool.display_name(), doctor.tool.key());
    print_status_details(&doctor.status);

    match &doctor.profiles {
        DoctorProfileSummary::Supported {
            profile_count,
            backup_count,
            current,
        } => {
            println!("  profiles          {profile_count}");
            println!("  backups           {backup_count}");
            println!("  current           {}", format_doctor_current(current));
        }
        DoctorProfileSummary::Unsupported => {
            println!("  profiles          unsupported");
            println!("  backups           unsupported");
            println!("  current           unsupported");
        }
    }

    for finding in &doctor.findings {
        println!(
            "  {:<18}{}",
            doctor_severity_label(&finding.severity),
            finding.message
        );
    }
}

fn format_doctor_current(current: &DoctorCurrent) -> String {
    match current {
        DoctorCurrent::Known(profile) => profile.to_owned(),
        DoctorCurrent::Ambiguous(profiles) => format!("ambiguous ({})", profiles.join(", ")),
        DoctorCurrent::Unknown => "unknown".to_owned(),
    }
}

fn doctor_severity_label(severity: &DoctorSeverity) -> &'static str {
    match severity {
        DoctorSeverity::Ok => "ok",
        DoctorSeverity::Info => "info",
        DoctorSeverity::Warning => "warning",
    }
}
