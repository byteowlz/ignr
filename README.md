# ignr

Auto-detect languages and tools in your project and generate a `.gitignore` file with the right patterns.

## Installation

```bash
cargo install --path .
```

## Usage

```bash
# Auto-detect stack and generate .gitignore in current directory
ignr generate

# Print to stdout instead of writing
ignr generate --print

# Append to existing .gitignore
ignr generate --append

# Add specific templates
ignr generate --add terraform --add docker

# Skip auto-detection, only use specified templates
ignr generate --no-detect --add rust --add macos

# Scan a specific directory
ignr generate --dir /path/to/project
```

## Subcommands

| Command                        | Description                                                                |
| ------------------------------ | -------------------------------------------------------------------------- |
| `generate` (alias: `gen`, `g`) | Auto-detect stack and generate `.gitignore`                                |
| `list` (alias: `ls`)           | List available templates                                                   |
| `sync`                         | Sync templates from remote source (gitignore.io)                           |
| `init`                         | Create config directories and default config file                          |
| `config show\|path\|reset`     | Inspect and manage configuration                                           |
| `completions <shell>`          | Generate shell completions (`bash`, `zsh`, `fish`, `powershell`, `elvish`) |

## Supported Technologies

**Languages:** Rust, Python, Node.js, Go, Java, C#, C++, Ruby, Swift, Kotlin, PHP, Scala, Elixir, Haskell, Zig, Dart

**Tools:** Terraform, Ansible, Docker

**IDEs/Editors:** VS Code, IntelliJ, Vim, Emacs

**Operating Systems:** Linux, macOS, Windows

## Global Flags

| Flag                            | Description                           |
| ------------------------------- | ------------------------------------- |
| `-q`, `--quiet`                 | Reduce output to only errors          |
| `-v`, `--verbose`               | Increase verbosity (stackable: `-vv`) |
| `--debug`                       | Enable debug logging                  |
| `--trace`                       | Enable trace logging                  |
| `--json`                        | Output machine-readable JSON          |
| `--yaml`                        | Output machine-readable YAML          |
| `--no-color`                    | Disable ANSI colors                   |
| `--color <auto\|always\|never>` | Control color output                  |
| `--dry-run`                     | Do not change anything on disk        |
| `-y`, `--yes`                   | Assume "yes" for interactive prompts  |
| `--config <path>`               | Override the config file path         |

## Configuration

Config file path: `$XDG_CONFIG_HOME/ignr/config.toml` (falls back to `~/.config/ignr/config.toml`).

A default config is created on first run. See `examples/config.toml` for all options.

Environment overrides use the `IGNR__` prefix with `__` as separator:

```bash
IGNR__DETECTION__MAX_DEPTH=5 ignr generate
```

### Config Options

```toml
[templates]
template_dir = "~/.config/ignr/templates"  # Custom templates directory
template_url = "https://www.toptal.com/developers/gitignore/api"
prefer_local = true
always_include = ["macos", "vscode"]  # Always add these templates

[detection]
max_depth = 10      # Directory scan depth
detect_os = true    # Add OS-specific patterns
detect_ide = true   # Detect IDE directories

[paths]
data_dir = "~/.local/share/ignr"
cache_dir = "~/.cache/ignr"
```

## Development

```bash
cargo fmt           # Format code
cargo test          # Run tests
cargo clippy        # Lint
cargo run -- --help # Run locally
```
