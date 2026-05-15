# aith

`aith` is a native CLI for managing account profiles for AI coding tools.

The goal is to make work, personal, and client identities explicit across tools
like Claude Code, Codex, and Cursor without repeated logout/login flows.

## Status

`aith` is early. Codex profile management is implemented first because Codex
stores its active auth state in a local `auth.json` file.

Implemented:

- Tool status checks for Codex, Claude Code, and Cursor.
- Codex profile save/list/current/use/remove.
- Codex backup list/restore.
- Codex one-command temporary profile execution.
- Codex shell-scoped temporary profile sessions.
- Automatic backup before replacing active Codex auth.
- Private Unix permissions for stored profile directories and auth files.

Not implemented yet:

- Claude Code profile switching.
- Cursor profile switching.

## Quick Start

Use `devenv` to enter the pinned Rust development environment:

```sh
devenv shell
cargo run -- status
```

Save the current Codex login as a profile:

```sh
cargo run -- save codex personal
```

Inspect and switch Codex profiles:

```sh
cargo run -- list codex
cargo run -- current codex
cargo run -- use codex personal
```

Inspect backups created by profile switches:

```sh
cargo run -- backups codex
cargo run -- restore codex auth-1778702155-74626.json
```

Run one command with a saved Codex profile without switching your active login:

```sh
cargo run -- exec codex personal -- codex
```

Start a shell with a saved Codex profile without switching your active login:

```sh
cargo run -- shell codex personal
```

Remove a saved Codex profile:

```sh
cargo run -- remove codex old-client
```

## Commands

### Tools

List supported tools:

```sh
aith tools
```

### Status

Show safe auth/config status. This checks whether expected files and
environment variables exist, but does not print credential values.

```sh
aith status
aith status codex
aith status claude
aith status cursor
```

### Save

Save the current auth state as a named profile.

```sh
aith save codex personal
```

`add` is an alias for `save`:

```sh
aith add codex work
```

Profiles can be overwritten explicitly:

```sh
aith save codex personal --force
```

### List

List saved profiles for a tool:

```sh
aith list codex
```

### Current

Detect which saved profile matches the active auth state:

```sh
aith current codex
```

Possible outputs:

```text
codex: personal
codex: unknown
codex: ambiguous
  matches personal, duplicate
```

`ambiguous` means more than one saved profile has the same auth snapshot.

### Use

Switch a tool to a saved profile:

```sh
aith use codex personal
```

For Codex, this replaces the active `auth.json` with the saved profile snapshot.
The previous active `auth.json` is backed up first.

### Remove

Remove a saved profile:

```sh
aith remove codex old-client
```

By default, `remove` refuses to delete a profile that matches the active auth
state:

```text
Error: profile 'personal' is currently active for codex; pass --force to remove it
```

Force removal when you intentionally want to delete the saved profile:

```sh
aith remove codex personal --force
```

This removes only the saved profile directory. It does not touch active Codex
auth and does not delete backups.

### Backups

List backups created before profile switches or restores:

```sh
aith backups codex
```

Example output:

```text
auth-1778702155-74626.json      /Users/sadjow/Library/Application Support/aith/backups/codex/auth-1778702155-74626.json
```

Backup IDs use this generated form:

```text
auth-<timestamp>-<pid>.json
```

### Restore

Restore a backup into the active auth location:

```sh
aith restore codex auth-1778702155-74626.json
```

For Codex, this copies the selected backup to the active `auth.json`. The
current active `auth.json` is backed up first, so restore is reversible.

### Exec

Run a command with a temporary profile-scoped auth environment:

```sh
aith exec codex personal -- codex
aith exec codex work -- codex exec "review this repo"
```

For Codex, `exec` creates a temporary `CODEX_HOME`, copies the selected
profile's `auth.json` into it, copies the current Codex `config.toml` when one
exists, and runs the command with that temporary `CODEX_HOME`.

The active Codex auth file is not modified, and the temporary directory is
removed after the command exits. `aith exec` exits with the same status code as
the child command.

### Shell

Start a shell with a temporary profile-scoped auth environment:

```sh
aith shell codex personal
```

For Codex, `shell` stages the selected profile exactly like `exec`, then starts
your configured shell with `CODEX_HOME` pointing at the temporary profile home.
This lets separate terminal tabs use different Codex profiles at the same time.

The active Codex auth file is not modified, and the temporary directory is
removed when the shell exits. `aith shell` exits with the same status code as
the shell.

## Storage

Profiles are stored under `AITH_HOME` when it is set. Otherwise, `aith` uses the
platform data directory:

- macOS: `~/Library/Application Support/aith`
- Linux: `${XDG_DATA_HOME:-~/.local/share}/aith`
- Windows: `%LOCALAPPDATA%\aith`

Codex profiles are stored as:

```text
profiles/codex/<profile>/auth.json
```

Codex backups are stored as:

```text
backups/codex/auth-<timestamp>-<pid>.json
```

On Unix, profile directories are created with `0700` permissions and auth files
are written with `0600` permissions.

## Safety Model

- Credential file contents are never printed by status/current/list commands.
- Profile names are limited to ASCII letters, numbers, `-`, and `_`.
- `use` and `restore` create a backup before replacing active Codex auth.
- `exec` runs with a temporary `CODEX_HOME` and does not modify active Codex
  auth.
- `shell` starts a temporary `CODEX_HOME` session and does not modify active
  Codex auth.
- `remove` refuses to delete the active matching profile unless `--force` is
  passed.
- `restore` only accepts generated backup IDs in the form
  `auth-<timestamp>-<pid>.json`.
- Claude Code and Cursor profile switching intentionally return “not implemented
  yet” until their auth models are handled explicitly.

## Development

This project uses [devenv](https://devenv.sh/) to provide a pinned Rust
toolchain matching `rust-version` in `Cargo.toml`.

```sh
devenv shell
cargo check
cargo test
cargo run -- status
```

Common one-off checks can run without entering an interactive shell:

```sh
devenv shell cargo check
devenv shell cargo fmt -- --check
devenv shell cargo clippy --all-targets -- -D warnings
devenv shell cargo test
devenv shell ci
```

Integration tests run the real `aith` binary against temporary fake `AITH_HOME`
and `CODEX_HOME` directories. They do not read or modify real Codex credentials.

## Project Structure

- `src/cli.rs`: command parsing and user-facing output.
- `src/profiles/`: shared profile storage, result types, validation, backups,
  and filesystem safety helpers.
- `src/tools/`: tool metadata and tool-specific adapters.
- `src/tools/codex.rs`: Codex auth/profile behavior.

## Planned Commands

```sh
aith exec cursor work -- cursor agent
```

## Design Direction

- Local-first: credentials never leave the machine.
- Tool-native: use each upstream tool's supported auth and config mechanisms.
- Explicit: profile switches should be visible and reversible.
- Session-friendly: support one-command and shell-scoped temporary identities.
