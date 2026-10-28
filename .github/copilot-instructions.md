# GitHub Copilot Instructions for ignr

ignr is a Rust CLI (clap + config crate) that auto-detects languages/tools and
generates `.gitignore` files from the templates in `templates/`.

`AGENTS.md` at the repo root is the authoritative agent guide; this file only
summarizes it.

## Workflow

- `just fmt` after code changes, `just clippy` (must stay clean under
  `-D warnings`), `just test` before committing.
- CLI conventions: subcommand verbs, `-q`/`-v`/`--debug`/`--trace`,
  `--json`/`--yaml`, honor NO_COLOR/FORCE_COLOR, XDG config at
  `$XDG_CONFIG_HOME/ignr/config.toml` (see `examples/config.toml`).

## Issue Tracking (trx)

This project uses trx for ALL task tracking. Do not create markdown TODO lists.

```bash
trx ready              # Show unblocked issues
trx create "Title" -t task -p 2   # Create issue (types: bug/feature/task/epic/chore)
trx update <id> --status in_progress
trx close <id> -r "Done"
trx sync               # Commit .trx/ changes
```

Priorities: 0=critical, 1=high, 2=medium, 3=low, 4=backlog.
