# GitHub Copilot Instructions for ignr

## Project Overview

**ignr** is a CLI tool for generating .gitignore files. It uses templates for various languages and frameworks.

## Tech Stack

- **Language**: Rust
- **CLI Framework**: Clap
- **Config**: TOML via config crate

## Coding Guidelines

### Testing
- Run `cargo test` before committing
- Run `cargo fmt` after code changes

### Code Style
- Follow Clippy best practices
- Inline `format!` arguments
- Prefer method references over redundant closures

## Issue Tracking with bd

**CRITICAL**: This project uses **bd** for ALL task tracking. Do NOT create markdown TODO lists.

### Essential Commands

```bash
# Find work
bd ready --json                    # Unblocked issues

# Create and manage
bd create "Title" -t bug|feature|task -p 0-4 --json
bd create "Subtask" --parent <epic-id> --json  # Hierarchical subtask
bd update <id> --status in_progress --json
bd close <id> --reason "Done" --json

# Search
bd list --status open --priority 1 --json
bd show <id> --json
```

### Workflow

1. **Check ready work**: `bd ready --json`
2. **Claim task**: `bd update <id> --status in_progress`
3. **Work on it**: Implement, test, document
4. **Discover new work?** `bd create "Found bug" -p 1 --deps discovered-from:<parent-id> --json`
5. **Complete**: `bd close <id> --reason "Done" --json`

### Priorities

- `0` - Critical (security, data loss, broken builds)
- `1` - High (major features, important bugs)
- `2` - Medium (default, nice-to-have)
- `3` - Low (polish, optimization)
- `4` - Backlog (future ideas)

## Project Structure

```
ignr/
├── src/
│   └── main.rs          # CLI entry point
├── templates/           # .gitignore templates
├── examples/
│   └── config.toml      # Example configuration
├── .beads/
│   └── issues.jsonl     # Git-synced issue storage
├── Cargo.toml
└── AGENTS.md            # AI agent guide
```

## CLI Help

Run `bd <command> --help` to see all available flags for any command.
For example: `bd create --help` shows `--parent`, `--deps`, `--assignee`, etc.

## Important Rules

- Use bd for ALL task tracking
- Always use `--json` flag for programmatic use
- Run `bd <cmd> --help` to discover available flags
- Do NOT create markdown TODO lists
- Do NOT use external issue trackers

---

**For detailed workflows and advanced features, see [AGENTS.md](../AGENTS.md)**
