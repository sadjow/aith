# aith

`aith` is a native CLI for managing account profiles for AI coding tools.

The goal is to make work, personal, and client identities explicit across tools
like Codex CLI, Claude Code, and Cursor Agent without repeated logout/login
flows.

## Supported Surfaces

`aith` treats each auth surface separately. A terminal tool and a desktop app can
be logged into different accounts, so they should not share one profile target.

Current command keys target CLI/agent surfaces:

- `codex`: Codex CLI.
- `claude`: Claude Code.
- `cursor`: Cursor Agent or terminal Cursor auth through `CURSOR_API_KEY`.

Desktop apps are not managed yet. Future support should use separate explicit
targets such as `codex-desktop`, `claude-desktop`, or `cursor-desktop`, starting
with read-only `status` and `doctor` discovery before any switching behavior.

## Status

`aith` is early. Codex CLI profile management is implemented first because
Codex CLI stores its active auth state in a local `auth.json` file.

Implemented:

- Tool status checks for Codex CLI, Claude Code, and Cursor Agent.
- Read-only doctor diagnostics for auth/profile readiness.
- Claude Code auth/config discovery.
- Claude Code env-profile save/list/remove.
- Claude Code one-command and shell-scoped env-profile sessions.
- Cursor env-profile save/list/remove.
- Cursor one-command and shell-scoped env-profile sessions.
- Codex CLI profile save/list/current/use/remove.
- Codex CLI backup list/restore.
- Codex CLI one-command temporary profile execution.
- Codex CLI shell-scoped temporary profile sessions.
- Automatic backup before replacing active Codex CLI auth.
- Private Unix permissions for stored profile directories, auth files, and env
  profile files.

Not implemented yet:

- Claude Code global login switching, including subscription/Keychain account
  switching.
- Cursor global login switching beyond terminal API-key sessions.
- Desktop app auth discovery or switching for Codex, Claude, or Cursor.

## Quick Start

Use `devenv` to enter the pinned Rust development environment:

```sh
devenv shell
cargo run -- status
```

Save the current Codex CLI login as a profile:

```sh
cargo run -- save codex personal
```

Inspect and switch Codex CLI profiles:

```sh
cargo run -- list codex
cargo run -- current codex
cargo run -- doctor codex
cargo run -- use codex personal
```

Inspect backups created by profile switches:

```sh
cargo run -- backups codex
cargo run -- restore codex auth-1778702155-74626.json
```

Run one command with a saved Codex CLI profile without switching your active
login:

```sh
cargo run -- exec codex personal -- codex
```

Start a shell with a saved Codex CLI profile without switching your active login:

```sh
cargo run -- shell codex personal
```

Save a Claude Code env profile without storing the secret value:

```sh
export ANTHROPIC_API_KEY_WORK=sk-ant-...
cargo run -- save claude work --from-env ANTHROPIC_API_KEY=ANTHROPIC_API_KEY_WORK
cargo run -- exec claude work -- claude
```

Save a Cursor env profile without storing the secret value:

```sh
export CURSOR_API_KEY_WORK=...
cargo run -- save cursor work --from-env CURSOR_API_KEY=CURSOR_API_KEY_WORK
cargo run -- exec cursor work -- cursor-agent
```

Remove a saved Codex CLI profile:

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

### Doctor

Show a read-only diagnostic report for tool auth paths, relevant environment
variables, saved profile counts, backup counts, and current profile detection.
Credential file contents are never printed.

```sh
aith doctor
aith doctor codex
aith doctor claude
aith doctor cursor
```

For Claude Code and Cursor, `doctor` reports both safe path/env status and saved
env profiles. It still warns that global login switching is not implemented for
those tools.

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

Claude Code and Cursor profiles are env-based. `aith` stores references to
source environment variables, not their values:

```sh
export ANTHROPIC_API_KEY_WORK=sk-ant-...
aith save claude work --from-env ANTHROPIC_API_KEY=ANTHROPIC_API_KEY_WORK
aith save cursor work --from-env CURSOR_API_KEY=CURSOR_API_KEY_WORK
```

Non-secret settings can be stored as literals:

```sh
aith save claude work \
  --from-env ANTHROPIC_API_KEY=ANTHROPIC_API_KEY_WORK \
  --set-env ANTHROPIC_BASE_URL=https://api.anthropic.com \
  --force
```

`aith` refuses literal values for sensitive names such as `ANTHROPIC_API_KEY`;
use `--from-env` for secrets.

### List

List saved profiles for a tool:

```sh
aith list codex
aith list claude
aith list cursor
```

### Current

Detect which saved profile matches the active auth state:

```sh
aith current codex
aith current claude
aith current cursor
```

Possible outputs:

```text
codex: personal
codex: unknown
codex: ambiguous
  matches personal, duplicate
```

`ambiguous` means more than one saved profile has the same auth snapshot. Env
profiles are session-scoped, so `aith current claude` and `aith current cursor`
report `unknown` instead of trying to infer a global active account.

### Use

Switch a tool to a saved profile:

```sh
aith use codex personal
```

For Codex CLI, this replaces the active `auth.json` with the saved profile
snapshot. The previous active `auth.json` is backed up first.

### Remove

Remove a saved profile:

```sh
aith remove codex old-client
aith remove claude old-client
aith remove cursor old-client
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
CLI auth and does not delete backups.

### Backups

List backups created before profile switches or restores:

```sh
aith backups codex
aith backups claude
aith backups cursor
```

Env profiles do not replace active auth files, so there are no Claude or Cursor
backups to list.

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

For Codex CLI, this copies the selected backup to the active `auth.json`. The
current active `auth.json` is backed up first, so restore is reversible.

### Exec

Run a command with a temporary profile-scoped auth environment:

```sh
aith exec codex personal -- codex
aith exec codex work -- codex exec "review this repo"
aith exec claude work -- claude
aith exec cursor work -- cursor-agent
```

For Codex CLI, `exec` creates a temporary `CODEX_HOME`, copies the selected
profile's `auth.json` into it, copies the current Codex CLI `config.toml` when
one exists, and runs the command with that temporary `CODEX_HOME`.

The active Codex CLI auth file is not modified, and the temporary directory is
removed after the command exits. `aith exec` exits with the same status code as
the child command.

For Claude Code and Cursor, `exec` resolves saved env references at runtime and
starts the child command with those target variables set. Config files, login
state, and Keychain entries are not modified.

### Shell

Start a shell with a temporary profile-scoped auth environment:

```sh
aith shell codex personal
aith shell claude work
aith shell cursor work
```

For Codex CLI, `shell` stages the selected profile exactly like `exec`, then
starts your configured shell with `CODEX_HOME` pointing at the temporary profile
home. This lets separate terminal tabs use different Codex CLI profiles at the
same time.

The active Codex CLI auth file is not modified, and the temporary directory is
removed when the shell exits. `aith shell` exits with the same status code as
the shell.

For Claude Code and Cursor, `shell` resolves saved env references and starts your
configured shell with those variables set. This lets separate terminal tabs use
different API-key profiles at the same time when the upstream command honors
terminal auth env vars.

## Storage

Profiles are stored under `AITH_HOME` when it is set. Otherwise, `aith` uses the
platform data directory:

- macOS: `~/Library/Application Support/aith`
- Linux: `${XDG_DATA_HOME:-~/.local/share}/aith`
- Windows: `%LOCALAPPDATA%\aith`

Codex CLI profiles are stored as:

```text
profiles/codex/<profile>/auth.json
```

Codex CLI backups are stored as:

```text
backups/codex/auth-<timestamp>-<pid>.json
```

Env profiles for Claude Code and Cursor are stored as:

```text
profiles/<tool>/<profile>/profile.toml
```

Example Claude profile:

```toml
[env]
ANTHROPIC_API_KEY = { from_env = "ANTHROPIC_API_KEY_WORK" }
ANTHROPIC_BASE_URL = "https://api.anthropic.com"
```

Example Cursor profile:

```toml
[env]
CURSOR_API_KEY = { from_env = "CURSOR_API_KEY_WORK" }
```

On Unix, profile directories are created with `0700` permissions and auth files
or profile files are written with `0600` permissions.

## Claude Code Discovery and Env Profiles

`aith status claude` and `aith doctor claude` check known Claude Code settings
and auth surfaces without reading credential contents.

Claude Code discovery checks:

- User config directory: `CLAUDE_CONFIG_DIR` or `~/.claude`
- User settings: `settings.json`
- User state: `~/.claude.json`
- Project settings: `.claude/settings.json` and local `.claude/settings.local.json`
- Managed settings path for the current platform
- Terminal auth environment variables such as `ANTHROPIC_API_KEY`,
  `ANTHROPIC_AUTH_TOKEN`, `CLAUDE_CODE_OAUTH_TOKEN`, and cloud-provider mode
  variables

Credential storage differs by platform. On macOS, Claude Code subscription
credentials are stored in Keychain and `aith` does not inspect Keychain. On
Linux and Windows, Claude Code uses `.credentials.json` under the Claude config
directory, including `CLAUDE_CONFIG_DIR` when set.

Claude env profiles are intentionally narrower than Codex CLI file-backed
profiles. They do not switch the logged-in Claude Code subscription account.
They only set terminal auth environment variables for `aith exec claude ...` and
`aith shell claude ...` sessions.

References:

- [Claude Code settings](https://code.claude.com/docs/en/settings)
- [Claude Code authentication](https://code.claude.com/docs/en/team)
- [Claude Code CLI reference](https://code.claude.com/docs/en/cli-reference)

## Cursor Env Profiles

`aith status cursor` and `aith doctor cursor` check the Cursor user data path and
terminal auth environment without printing credential values.

Cursor env profiles set terminal auth environment variables for
`aith exec cursor ...` and `aith shell cursor ...` sessions. They do not modify
Cursor user data or any global login state.

## Safety Model

- Credential file contents are never printed by status/doctor/current/list
  commands.
- Profile names are limited to ASCII letters, numbers, `-`, and `_`.
- `use` and `restore` create a backup before replacing active Codex CLI auth.
- `exec` runs with a temporary `CODEX_HOME` and does not modify active Codex CLI
  auth.
- `shell` starts a temporary `CODEX_HOME` session and does not modify active
  Codex CLI auth.
- Env profiles store source env variable names for secrets, not secret values.
  Secret values are resolved only when `exec` or `shell` starts.
- Claude and Cursor env sessions do not modify config files, credential files,
  user data, or macOS Keychain entries.
- Desktop app auth stores are intentionally out of scope for current profile
  operations.
- `remove` refuses to delete the active matching profile unless `--force` is
  passed.
- `restore` only accepts generated backup IDs in the form
  `auth-<timestamp>-<pid>.json`.
- Claude Code, Cursor, and desktop global login switching intentionally return
  “not implemented yet” until their auth models are handled explicitly.

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

Integration tests run the real `aith` binary against temporary fake `AITH_HOME`,
`CODEX_HOME`, `CLAUDE_CONFIG_DIR`, and `HOME` directories. They do not read or
modify real tool credentials.

## Continuous Integration

GitHub Actions runs the same checks as the local CI script on pushes to `main`
and pull requests:

```sh
devenv shell ci
```

The workflow installs Nix and `devenv`, then runs the pinned Rust toolchain from
`devenv.nix`.

## Project Structure

- `src/cli.rs`: command parsing and user-facing output.
- `src/doctor.rs`: read-only diagnostic report generation.
- `src/profiles/`: shared profile storage, result types, validation, backups,
  and filesystem safety helpers.
- `src/tools/`: tool metadata and tool-specific adapters.
- `src/tools/claude.rs`: Claude Code auth/config discovery and env-profile
  sessions.
- `src/tools/codex.rs`: Codex CLI auth/profile behavior.
- `src/tools/cursor.rs`: Cursor auth discovery and env-profile sessions.
- `src/tools/env_session.rs`: shared env-profile session behavior.

## Design Direction

- Local-first: credentials never leave the machine.
- Tool-native: use each upstream tool's supported auth and config mechanisms.
- Explicit: profile switches should be visible and reversible.
- Session-friendly: support one-command and shell-scoped temporary identities.
