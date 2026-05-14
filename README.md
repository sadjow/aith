# aith

`aith` is a native CLI for managing account profiles for AI coding tools.

The goal is to make work, personal, and client identities explicit across tools
like Claude Code, Codex, and Cursor without repeated logout/login flows.

## Current commands

```sh
aith tools
aith status
aith status codex
aith save codex work
aith add codex personal
aith list codex
aith use codex work
```

## Development

This project uses [devenv](https://devenv.sh/) to provide a pinned Rust
toolchain.

```sh
devenv shell
cargo check
cargo test
```

Common one-off checks can run without entering an interactive shell:

```sh
devenv shell -- check
devenv shell -- fmt
devenv shell -- clippy
devenv shell -- tests
devenv shell -- ci
```

## Planned commands

```sh
aith exec cursor work -- cursor agent
aith shell codex client-a
```

## Design direction

- Local-first: credentials never leave the machine.
- Tool-native: use each upstream tool's supported auth and config mechanisms.
- Explicit: profile switches should be visible and reversible.
- Session-friendly: support one-command and shell-scoped temporary identities.

## Storage

Profiles are stored under `AITH_HOME` when it is set. Otherwise, `aith` uses the
platform data directory:

- macOS: `~/Library/Application Support/aith`
- Linux: `${XDG_DATA_HOME:-~/.local/share}/aith`
- Windows: `%LOCALAPPDATA%\aith`

Codex profile switching currently snapshots and restores `auth.json` only. The
active auth file is backed up before `aith use codex <profile>` replaces it.
