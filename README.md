# aith

`aith` is a native CLI for managing account profiles for AI coding tools.

The goal is to make work, personal, and client identities explicit across tools
like Claude Code, Codex, and Cursor without repeated logout/login flows.

## Current commands

```sh
aith tools
aith status
aith status codex
```

## Planned commands

```sh
aith add codex work
aith use claude personal
aith exec cursor work -- cursor agent
aith shell codex client-a
aith status
```

## Design direction

- Local-first: credentials never leave the machine.
- Tool-native: use each upstream tool's supported auth and config mechanisms.
- Explicit: profile switches should be visible and reversible.
- Session-friendly: support one-command and shell-scoped temporary identities.
