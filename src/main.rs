use std::collections::BTreeSet;
use std::env;
use std::fmt;
use std::fs;
use std::io::{self, IsTerminal, Write};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow};
use chrono::Utc;
use clap::{Args, CommandFactory, Parser, Subcommand, ValueEnum};
use clap_complete::Shell;
use config::{Config, Environment, File, FileFormat};
use env_logger::fmt::WriteStyle;
use ignore::WalkBuilder;
use log::{LevelFilter, debug, info, warn};
use serde::{Deserialize, Serialize};

const APP_NAME: &str = env!("CARGO_PKG_NAME");

// Embedded templates from the templates/ directory
const EMBEDDED_TEMPLATES: &[(&str, &str)] = &[
    ("rust", include_str!("../templates/rust.gitignore")),
    ("python", include_str!("../templates/python.gitignore")),
    ("node", include_str!("../templates/node.gitignore")),
    ("go", include_str!("../templates/go.gitignore")),
    ("java", include_str!("../templates/java.gitignore")),
    ("csharp", include_str!("../templates/csharp.gitignore")),
    ("cpp", include_str!("../templates/cpp.gitignore")),
    ("ruby", include_str!("../templates/ruby.gitignore")),
    ("swift", include_str!("../templates/swift.gitignore")),
    ("kotlin", include_str!("../templates/kotlin.gitignore")),
    ("php", include_str!("../templates/php.gitignore")),
    ("scala", include_str!("../templates/scala.gitignore")),
    ("elixir", include_str!("../templates/elixir.gitignore")),
    ("haskell", include_str!("../templates/haskell.gitignore")),
    ("zig", include_str!("../templates/zig.gitignore")),
    ("dart", include_str!("../templates/dart.gitignore")),
    ("terraform", include_str!("../templates/terraform.gitignore")),
    ("ansible", include_str!("../templates/ansible.gitignore")),
    ("docker", include_str!("../templates/docker.gitignore")),
    ("vscode", include_str!("../templates/vscode.gitignore")),
    ("intellij", include_str!("../templates/intellij.gitignore")),
    ("vim", include_str!("../templates/vim.gitignore")),
    ("emacs", include_str!("../templates/emacs.gitignore")),
    ("linux", include_str!("../templates/linux.gitignore")),
    ("macos", include_str!("../templates/macos.gitignore")),
    ("windows", include_str!("../templates/windows.gitignore")),
];

fn main() {
    if let Err(err) = try_main() {
        let _ = writeln!(io::stderr(), "{err:?}");
        std::process::exit(1);
    }
}

fn try_main() -> Result<()> {
    let cli = Cli::parse();

    let ctx = RuntimeContext::new(cli.common.clone())?;
    ctx.init_logging()?;
    debug!("resolved paths: {:#?}", ctx.paths);

    match cli.command {
        Command::Generate(cmd) => handle_generate(&ctx, cmd),
        Command::Sync(cmd) => handle_sync(&ctx, cmd),
        Command::List => handle_list(&ctx),
        Command::Init(cmd) => handle_init(&ctx, cmd),
        Command::Config { command } => handle_config(&ctx, command),
        Command::Completions { shell } => handle_completions(shell),
    }
}

#[derive(Debug, Parser)]
#[command(
    author,
    version,
    about = "Auto-detect languages/tools and generate .gitignore files",
    propagate_version = true
)]
struct Cli {
    #[command(flatten)]
    common: CommonOpts,
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Clone, Args)]
struct CommonOpts {
    /// Override the config file path
    #[arg(long, value_name = "PATH", global = true)]
    config: Option<PathBuf>,
    /// Reduce output to only errors
    #[arg(short, long, action = clap::ArgAction::SetTrue, global = true)]
    quiet: bool,
    /// Increase logging verbosity (stackable)
    #[arg(short = 'v', long = "verbose", action = clap::ArgAction::Count, global = true)]
    verbose: u8,
    /// Enable debug logging (equivalent to -vv)
    #[arg(long, global = true)]
    debug: bool,
    /// Enable trace logging (overrides other levels)
    #[arg(long, global = true)]
    trace: bool,
    /// Output machine readable JSON
    #[arg(long, global = true, conflicts_with = "yaml")]
    json: bool,
    /// Output machine readable YAML
    #[arg(long, global = true)]
    yaml: bool,
    /// Disable ANSI colors in output
    #[arg(long = "no-color", global = true, conflicts_with = "color")]
    no_color: bool,
    /// Control color output (auto, always, never)
    #[arg(long, value_enum, default_value_t = ColorOption::Auto, global = true)]
    color: ColorOption,
    /// Do not change anything on disk
    #[arg(long = "dry-run", global = true)]
    dry_run: bool,
    /// Assume "yes" for interactive prompts
    #[arg(short = 'y', long = "yes", global = true)]
    assume_yes: bool,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum ColorOption {
    Auto,
    Always,
    Never,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Generate .gitignore (default behavior, auto-detects stack)
    #[command(alias = "gen", alias = "g")]
    Generate(GenerateCommand),
    /// Sync templates from remote source
    Sync(SyncCommand),
    /// List available templates
    #[command(alias = "ls")]
    List,
    /// Create config directories and default files
    Init(InitCommand),
    /// Inspect and manage configuration
    Config {
        #[command(subcommand)]
        command: ConfigCommand,
    },
    /// Generate shell completions
    Completions {
        #[arg(value_enum)]
        shell: Shell,
    },
}

#[derive(Debug, Clone, Args)]
struct GenerateCommand {
    /// Print to stdout instead of writing to .gitignore
    #[arg(long, short = 'p')]
    print: bool,
    /// Append to existing .gitignore instead of replacing managed section
    #[arg(long, short = 'a')]
    append: bool,
    /// Skip auto-detection, only use explicitly specified templates
    #[arg(long)]
    no_detect: bool,
    /// Additional templates to include
    #[arg(long, short = 't', value_name = "TEMPLATE")]
    add: Vec<String>,
    /// Directory to scan (defaults to current directory)
    #[arg(long, short = 'd', value_name = "PATH")]
    dir: Option<PathBuf>,
    /// Maximum directory depth to scan
    #[arg(long, default_value = "10")]
    depth: usize,
    /// Create .gitignore even if not in a git repo
    #[arg(long, short = 'f')]
    force: bool,
}

#[derive(Debug, Clone, Args)]
struct SyncCommand {
    /// Override the remote URL to sync from
    #[arg(long, value_name = "URL")]
    url: Option<String>,
}

#[derive(Debug, Clone, Args)]
struct InitCommand {
    /// Recreate configuration even if it already exists
    #[arg(long = "force")]
    force: bool,
}

#[derive(Debug, Subcommand)]
enum ConfigCommand {
    /// Output the effective configuration
    Show,
    /// Print the resolved config file path
    Path,
    /// Regenerate the default configuration file
    Reset,
}

#[derive(Debug, Clone)]
struct RuntimeContext {
    common: CommonOpts,
    paths: AppPaths,
    config: AppConfig,
}

impl RuntimeContext {
    fn new(common: CommonOpts) -> Result<Self> {
        let mut paths = AppPaths::discover(common.config.clone())?;
        let config = load_or_init_config(&mut paths, &common)?;
        let paths = paths.apply_overrides(&config)?;
        let ctx = Self {
            common,
            paths,
            config,
        };
        ctx.ensure_directories()?;
        ctx.ensure_embedded_templates()?;
        Ok(ctx)
    }

    fn ensure_embedded_templates(&self) -> Result<()> {
        let templates_dir = self.paths.data_dir.join("templates");

        // Check if templates directory is empty or doesn't exist
        let is_empty = !templates_dir.exists()
            || fs::read_dir(&templates_dir)
                .map(|mut entries| entries.next().is_none())
                .unwrap_or(true);

        if !is_empty {
            return Ok(());
        }

        if self.common.dry_run {
            info!(
                "dry-run: would write embedded templates to {}",
                templates_dir.display()
            );
            return Ok(());
        }

        fs::create_dir_all(&templates_dir).with_context(|| {
            format!("creating templates directory {}", templates_dir.display())
        })?;

        for (name, content) in EMBEDDED_TEMPLATES {
            let path = templates_dir.join(format!("{name}.gitignore"));
            fs::write(&path, content).with_context(|| {
                format!("writing embedded template {}", path.display())
            })?;
            debug!("Wrote embedded template: {name}");
        }

        info!(
            "Initialized {} embedded templates in {}",
            EMBEDDED_TEMPLATES.len(),
            templates_dir.display()
        );

        Ok(())
    }

    fn init_logging(&self) -> Result<()> {
        if self.common.quiet {
            log::set_max_level(LevelFilter::Off);
            return Ok(());
        }

        let mut builder =
            env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn"));

        builder.filter_level(self.effective_log_level());

        let force_color = matches!(self.common.color, ColorOption::Always)
            || env::var_os("FORCE_COLOR").is_some();
        let disable_color = self.common.no_color
            || matches!(self.common.color, ColorOption::Never)
            || env::var_os("NO_COLOR").is_some()
            || (!force_color && !io::stderr().is_terminal());

        if disable_color {
            builder.write_style(WriteStyle::Never);
        } else if force_color {
            builder.write_style(WriteStyle::Always);
        } else {
            builder.write_style(WriteStyle::Auto);
        }

        builder.try_init().or_else(|err| {
            if self.common.verbose > 0 {
                eprintln!("logger already initialized: {err}");
            }
            Ok(())
        })
    }

    fn effective_log_level(&self) -> LevelFilter {
        if self.common.trace {
            LevelFilter::Trace
        } else if self.common.debug {
            LevelFilter::Debug
        } else {
            match self.common.verbose {
                0 => LevelFilter::Warn,
                1 => LevelFilter::Info,
                2 => LevelFilter::Debug,
                _ => LevelFilter::Trace,
            }
        }
    }

    fn ensure_directories(&self) -> Result<()> {
        if self.common.dry_run {
            info!(
                "dry-run: would ensure data dir {} and cache dir {}",
                self.paths.data_dir.display(),
                self.paths.cache_dir.display(),
            );
            return Ok(());
        }

        fs::create_dir_all(&self.paths.data_dir).with_context(|| {
            format!("creating data directory {}", self.paths.data_dir.display())
        })?;
        fs::create_dir_all(&self.paths.cache_dir).with_context(|| {
            format!("creating cache directory {}", self.paths.cache_dir.display())
        })?;
        Ok(())
    }
}

#[derive(Debug, Clone)]
struct AppPaths {
    config_file: PathBuf,
    data_dir: PathBuf,
    cache_dir: PathBuf,
}

impl AppPaths {
    fn discover(override_path: Option<PathBuf>) -> Result<Self> {
        let config_file = match override_path {
            Some(path) => {
                let expanded = expand_path(path)?;
                if expanded.is_dir() {
                    expanded.join("config.toml")
                } else {
                    expanded
                }
            }
            None => default_config_dir()?.join("config.toml"),
        };

        if config_file.parent().is_none() {
            return Err(anyhow!("invalid config file path: {config_file:?}"));
        }

        let data_dir = default_data_dir()?;
        let cache_dir = default_cache_dir()?;

        Ok(Self {
            config_file,
            data_dir,
            cache_dir,
        })
    }

    fn apply_overrides(mut self, cfg: &AppConfig) -> Result<Self> {
        if let Some(ref data_override) = cfg.paths.data_dir {
            self.data_dir = expand_str_path(data_override)?;
        }
        if let Some(ref cache_override) = cfg.paths.cache_dir {
            self.cache_dir = expand_str_path(cache_override)?;
        }
        Ok(self)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
struct AppConfig {
    templates: TemplatesConfig,
    detection: DetectionConfig,
    paths: PathsConfig,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            templates: TemplatesConfig::default(),
            detection: DetectionConfig::default(),
            paths: PathsConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
struct TemplatesConfig {
    /// Local directory containing custom .gitignore templates
    template_dir: Option<String>,
    /// Remote URL to fetch templates from (gitignore.io compatible or GitHub raw URL)
    template_url: Option<String>,
    /// Whether to prefer local templates over embedded ones
    prefer_local: bool,
    /// Additional templates to always include
    always_include: Vec<String>,
}

impl Default for TemplatesConfig {
    fn default() -> Self {
        Self {
            template_dir: None,
            template_url: Some("https://www.toptal.com/developers/gitignore/api".to_string()),
            prefer_local: true,
            always_include: vec![],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
struct DetectionConfig {
    /// Maximum depth to scan for detection
    max_depth: usize,
    /// Whether to detect OS-specific patterns
    detect_os: bool,
    /// Whether to detect IDE/editor patterns
    detect_ide: bool,
}

impl Default for DetectionConfig {
    fn default() -> Self {
        Self {
            max_depth: 10,
            detect_os: true,
            detect_ide: true,
        }
    }
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default)]
struct PathsConfig {
    data_dir: Option<String>,
    cache_dir: Option<String>,
}

/// Detects technologies used in a directory
fn detect_technologies(dir: &Path, config: &DetectionConfig, depth: usize) -> Result<BTreeSet<String>> {
    let mut detected = BTreeSet::new();

    let walker = WalkBuilder::new(dir)
        .max_depth(Some(depth.min(config.max_depth)))
        .hidden(false)
        .git_ignore(true)
        .build();

    for entry in walker.flatten() {
        let path = entry.path();
        let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

        // Detect by manifest files
        match file_name {
            "Cargo.toml" => { detected.insert("rust".to_string()); }
            "package.json" => { detected.insert("node".to_string()); }
            "requirements.txt" | "pyproject.toml" | "setup.py" | "Pipfile" | "uv.lock" => {
                detected.insert("python".to_string());
            }
            "go.mod" | "go.sum" => { detected.insert("go".to_string()); }
            "pom.xml" | "build.gradle" => { detected.insert("java".to_string()); }
            "build.gradle.kts" => {
                detected.insert("java".to_string());
                // Check if this is a Kotlin project
                if path.to_string_lossy().contains("kotlin") {
                    detected.insert("kotlin".to_string());
                }
            }
            "CMakeLists.txt" | "Makefile" | "configure.ac" => { detected.insert("cpp".to_string()); }
            "Gemfile" | "Rakefile" => { detected.insert("ruby".to_string()); }
            "Package.swift" => { detected.insert("swift".to_string()); }
            "composer.json" => { detected.insert("php".to_string()); }
            "build.sbt" => { detected.insert("scala".to_string()); }
            "mix.exs" => { detected.insert("elixir".to_string()); }
            "stack.yaml" | "cabal.project" => { detected.insert("haskell".to_string()); }
            "build.zig" => { detected.insert("zig".to_string()); }
            "pubspec.yaml" => { detected.insert("dart".to_string()); }
            "main.tf" | "terraform.tf" => { detected.insert("terraform".to_string()); }
            "playbook.yml" | "ansible.cfg" => { detected.insert("ansible".to_string()); }
            "Dockerfile" | "docker-compose.yml" | "docker-compose.yaml" => {
                detected.insert("docker".to_string());
            }
            _ => {}
        }

        // Detect by file extension
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            match ext {
                "rs" => { detected.insert("rust".to_string()); }
                "py" | "pyw" | "pyi" => { detected.insert("python".to_string()); }
                "js" | "jsx" | "ts" | "tsx" | "mjs" | "cjs" => { detected.insert("node".to_string()); }
                "go" => { detected.insert("go".to_string()); }
                "java" => { detected.insert("java".to_string()); }
                "cs" | "fs" | "vb" => { detected.insert("csharp".to_string()); }
                "c" | "cpp" | "cc" | "cxx" | "h" | "hpp" | "hxx" => { detected.insert("cpp".to_string()); }
                "rb" => { detected.insert("ruby".to_string()); }
                "swift" => { detected.insert("swift".to_string()); }
                "kt" | "kts" => { detected.insert("kotlin".to_string()); }
                "php" => { detected.insert("php".to_string()); }
                "scala" | "sc" => { detected.insert("scala".to_string()); }
                "ex" | "exs" => { detected.insert("elixir".to_string()); }
                "hs" | "lhs" => { detected.insert("haskell".to_string()); }
                "zig" => { detected.insert("zig".to_string()); }
                "dart" => { detected.insert("dart".to_string()); }
                "tf" | "tfvars" => { detected.insert("terraform".to_string()); }
                "csproj" | "sln" | "fsproj" => { detected.insert("csharp".to_string()); }
                _ => {}
            }
        }

        // Detect IDE/editor directories
        if config.detect_ide && path.is_dir() {
            match file_name {
                ".vscode" => { detected.insert("vscode".to_string()); }
                ".idea" => { detected.insert("intellij".to_string()); }
                ".vim" | ".nvim" => { detected.insert("vim".to_string()); }
                ".emacs.d" => { detected.insert("emacs".to_string()); }
                _ => {}
            }
        }
    }

    // Detect OS
    if config.detect_os {
        #[cfg(target_os = "linux")]
        detected.insert("linux".to_string());
        #[cfg(target_os = "macos")]
        detected.insert("macos".to_string());
        #[cfg(target_os = "windows")]
        detected.insert("windows".to_string());
    }

    Ok(detected)
}

/// Template manager for loading and merging templates
struct TemplateManager<'a> {
    config: &'a AppConfig,
    data_dir: &'a Path,
}

impl<'a> TemplateManager<'a> {
    fn new(config: &'a AppConfig, data_dir: &'a Path) -> Self {
        Self { config, data_dir }
    }

    fn list_available(&self) -> Vec<String> {
        let mut templates: BTreeSet<String> = EMBEDDED_TEMPLATES
            .iter()
            .map(|(name, _)| name.to_string())
            .collect();

        // Add templates from custom directory
        if let Some(ref dir) = self.config.templates.template_dir {
            if let Ok(expanded) = expand_str_path(dir) {
                if let Ok(entries) = fs::read_dir(expanded) {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        if let Some(name) = path.file_stem().and_then(|n| n.to_str()) {
                            if path.extension().and_then(|e| e.to_str()) == Some("gitignore") {
                                templates.insert(name.to_string());
                            }
                        }
                    }
                }
            }
        }

        // Add templates from data directory
        let data_templates = self.data_dir.join("templates");
        if let Ok(entries) = fs::read_dir(&data_templates) {
            for entry in entries.flatten() {
                let path = entry.path();
                if let Some(name) = path.file_stem().and_then(|n| n.to_str()) {
                    if path.extension().and_then(|e| e.to_str()) == Some("gitignore") {
                        templates.insert(name.to_string());
                    }
                }
            }
        }

        templates.into_iter().collect()
    }

    fn get_template(&self, name: &str) -> Option<String> {
        let name_lower = name.to_lowercase();

        // Check custom template directory first if prefer_local is true
        if self.config.templates.prefer_local {
            if let Some(content) = self.load_from_custom_dir(&name_lower) {
                return Some(content);
            }
        }

        // Check data directory (synced templates)
        if let Some(content) = self.load_from_data_dir(&name_lower) {
            return Some(content);
        }

        // Check embedded templates
        for (embedded_name, content) in EMBEDDED_TEMPLATES {
            if embedded_name.to_lowercase() == name_lower {
                return Some(content.to_string());
            }
        }

        // Check custom directory if not checked yet
        if !self.config.templates.prefer_local {
            if let Some(content) = self.load_from_custom_dir(&name_lower) {
                return Some(content);
            }
        }

        None
    }

    fn load_from_custom_dir(&self, name: &str) -> Option<String> {
        let dir = self.config.templates.template_dir.as_ref()?;
        let expanded = expand_str_path(dir).ok()?;
        let path = expanded.join(format!("{name}.gitignore"));
        fs::read_to_string(path).ok()
    }

    fn load_from_data_dir(&self, name: &str) -> Option<String> {
        let path = self.data_dir.join("templates").join(format!("{name}.gitignore"));
        fs::read_to_string(path).ok()
    }

    fn merge_templates(&self, templates: &[String]) -> String {
        let mut lines: BTreeSet<String> = BTreeSet::new();
        let mut sections: Vec<(String, Vec<String>)> = Vec::new();

        for template_name in templates {
            if let Some(content) = self.get_template(template_name) {
                let mut section_lines = Vec::new();
                for line in content.lines() {
                    let trimmed = line.trim();
                    if !trimmed.is_empty() && !lines.contains(trimmed) {
                        lines.insert(trimmed.to_string());
                        section_lines.push(line.to_string());
                    }
                }
                if !section_lines.is_empty() {
                    sections.push((template_name.clone(), section_lines));
                }
            } else {
                warn!("Template '{}' not found", template_name);
            }
        }

        let mut output = String::new();
        for (name, section_lines) in sections {
            if !output.is_empty() {
                output.push('\n');
            }
            output.push_str(&format!("# === {} ===\n", name));
            for line in section_lines {
                output.push_str(&line);
                output.push('\n');
            }
        }

        output
    }
}

fn handle_generate(ctx: &RuntimeContext, cmd: GenerateCommand) -> Result<()> {
    let dir = cmd.dir.clone().unwrap_or_else(|| PathBuf::from("."));
    let dir = dir.canonicalize().unwrap_or(dir);

    // Check if in a git repo (unless --force)
    if !cmd.force && !dir.join(".git").exists() {
        // Walk up to find .git
        let mut found_git = false;
        let mut parent = dir.parent();
        while let Some(p) = parent {
            if p.join(".git").exists() {
                found_git = true;
                break;
            }
            parent = p.parent();
        }
        if !found_git {
            return Err(anyhow!(
                "Not in a git repository. Use --force to create .gitignore anyway."
            ));
        }
    }

    // Detect technologies
    let mut templates: BTreeSet<String> = if cmd.no_detect {
        BTreeSet::new()
    } else {
        detect_technologies(&dir, &ctx.config.detection, cmd.depth)?
    };

    // Add explicit templates
    for t in &cmd.add {
        templates.insert(t.to_lowercase());
    }

    // Add always_include templates from config
    for t in &ctx.config.templates.always_include {
        templates.insert(t.to_lowercase());
    }

    if templates.is_empty() {
        if ctx.common.json {
            println!("{}", serde_json::json!({
                "detected": [],
                "message": "No technologies detected and none specified"
            }));
        } else if ctx.common.yaml {
            println!("detected: []\nmessage: No technologies detected and none specified");
        } else {
            println!("No technologies detected and none specified. Use --add to specify templates.");
        }
        return Ok(());
    }

    let template_list: Vec<String> = templates.into_iter().collect();
    let manager = TemplateManager::new(&ctx.config, &ctx.paths.data_dir);
    let content = manager.merge_templates(&template_list);

    // Generate header
    let date = Utc::now().format("%Y-%m-%d");
    let detected_str = template_list.join(",");
    let header = format!(
        "# ---- ignr (detected: {}) @ {} ----\n",
        detected_str, date
    );

    let full_content = format!("{header}\n{content}");

    if cmd.print {
        if ctx.common.json {
            println!("{}", serde_json::json!({
                "detected": template_list,
                "content": full_content
            }));
        } else if ctx.common.yaml {
            println!("detected:");
            for t in &template_list {
                println!("  - {t}");
            }
            println!("content: |");
            for line in full_content.lines() {
                println!("  {line}");
            }
        } else {
            print!("{full_content}");
        }
        return Ok(());
    }

    let gitignore_path = dir.join(".gitignore");

    if ctx.common.dry_run {
        info!("dry-run: would write .gitignore to {}", gitignore_path.display());
        if ctx.common.verbose > 0 {
            println!("Detected: {}", template_list.join(", "));
            println!("Would write to: {}", gitignore_path.display());
        }
        return Ok(());
    }

    // Handle append mode
    let final_content = if cmd.append && gitignore_path.exists() {
        let existing = fs::read_to_string(&gitignore_path)
            .context("reading existing .gitignore")?;
        format!("{existing}\n{full_content}")
    } else if gitignore_path.exists() {
        // Replace managed section
        let existing = fs::read_to_string(&gitignore_path)
            .context("reading existing .gitignore")?;
        
        // Look for existing ignr section and replace it
        if let Some(start) = existing.find("# ---- ignr (detected:") {
            let before = &existing[..start];
            // Find end of ignr section (next non-ignr header or end of file)
            let after_start = &existing[start..];
            let end = after_start
                .find("\n# ----")
                .filter(|&pos| !after_start[pos + 1..].starts_with("--- ignr"))
                .map(|pos| start + pos + 1)
                .unwrap_or(existing.len());
            
            let after = if end < existing.len() { &existing[end..] } else { "" };
            format!("{before}{full_content}{after}")
        } else {
            format!("{existing}\n{full_content}")
        }
    } else {
        full_content
    };

    fs::write(&gitignore_path, final_content)
        .with_context(|| format!("writing .gitignore to {}", gitignore_path.display()))?;

    if !ctx.common.quiet {
        println!("Generated .gitignore with: {}", template_list.join(", "));
    }

    Ok(())
}

fn handle_sync(ctx: &RuntimeContext, cmd: SyncCommand) -> Result<()> {
    let url = cmd.url
        .or_else(|| ctx.config.templates.template_url.clone())
        .ok_or_else(|| anyhow!("No template URL configured. Set templates.template_url in config or use --url"))?;

    let templates_dir = ctx.paths.data_dir.join("templates");

    if ctx.common.dry_run {
        info!("dry-run: would sync templates from {} to {}", url, templates_dir.display());
        return Ok(());
    }

    fs::create_dir_all(&templates_dir)
        .context("creating templates data directory")?;

    // Fetch list of available templates
    let list_url = format!("{}/list", url.trim_end_matches('/'));
    info!("Fetching template list from {}", list_url);

    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .context("building HTTP client")?;

    let response = client.get(&list_url)
        .send()
        .context("fetching template list")?;

    if !response.status().is_success() {
        return Err(anyhow!("Failed to fetch template list: HTTP {}", response.status()));
    }

    let list_text = response.text().context("reading template list")?;
    let templates: Vec<&str> = list_text.lines().collect();

    if !ctx.common.quiet {
        println!("Found {} templates", templates.len());
    }

    let mut synced = 0;
    let mut failed = 0;

    for template in &templates {
        let template_name = template.trim().to_lowercase();
        if template_name.is_empty() {
            continue;
        }

        let template_url = format!("{}/{}", url.trim_end_matches('/'), template_name);
        debug!("Fetching template: {}", template_name);

        match client.get(&template_url).send() {
            Ok(resp) if resp.status().is_success() => {
                if let Ok(content) = resp.text() {
                    let path = templates_dir.join(format!("{template_name}.gitignore"));
                    if fs::write(&path, &content).is_ok() {
                        synced += 1;
                        debug!("Saved: {}", template_name);
                    } else {
                        failed += 1;
                        warn!("Failed to write: {}", template_name);
                    }
                }
            }
            Ok(resp) => {
                failed += 1;
                debug!("HTTP {} for template: {}", resp.status(), template_name);
            }
            Err(e) => {
                failed += 1;
                debug!("Failed to fetch {}: {}", template_name, e);
            }
        }
    }

    if !ctx.common.quiet {
        println!("Synced {} templates ({} failed)", synced, failed);
    }

    Ok(())
}

fn handle_list(ctx: &RuntimeContext) -> Result<()> {
    let manager = TemplateManager::new(&ctx.config, &ctx.paths.data_dir);
    let templates = manager.list_available();

    if ctx.common.json {
        println!("{}", serde_json::to_string_pretty(&templates)?);
    } else if ctx.common.yaml {
        println!("{}", serde_yaml::to_string(&templates)?);
    } else {
        for name in &templates {
            println!("{name}");
        }
    }

    Ok(())
}

fn handle_init(ctx: &RuntimeContext, cmd: InitCommand) -> Result<()> {
    if ctx.paths.config_file.exists() && !(cmd.force || ctx.common.assume_yes) {
        return Err(anyhow!(
            "config already exists at {} (use --force to overwrite)",
            ctx.paths.config_file.display()
        ));
    }

    if ctx.common.dry_run {
        info!(
            "dry-run: would write default config to {}",
            ctx.paths.config_file.display()
        );
        return Ok(());
    }

    write_default_config(&ctx.paths.config_file)?;

    if !ctx.common.quiet {
        println!("Created config at {}", ctx.paths.config_file.display());
    }

    Ok(())
}

fn handle_config(ctx: &RuntimeContext, command: ConfigCommand) -> Result<()> {
    match command {
        ConfigCommand::Show => {
            if ctx.common.json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&ctx.config)
                        .context("serializing config to JSON")?
                );
            } else if ctx.common.yaml {
                println!(
                    "{}",
                    serde_yaml::to_string(&ctx.config).context("serializing config to YAML")?
                );
            } else {
                println!("{:#?}", ctx.config);
            }
            Ok(())
        }
        ConfigCommand::Path => {
            println!("{}", ctx.paths.config_file.display());
            Ok(())
        }
        ConfigCommand::Reset => {
            if ctx.common.dry_run {
                info!(
                    "dry-run: would reset config at {}",
                    ctx.paths.config_file.display()
                );
                return Ok(());
            }
            write_default_config(&ctx.paths.config_file)?;
            if !ctx.common.quiet {
                println!("Reset config at {}", ctx.paths.config_file.display());
            }
            Ok(())
        }
    }
}

fn handle_completions(shell: Shell) -> Result<()> {
    let mut cmd = Cli::command();
    clap_complete::generate(shell, &mut cmd, APP_NAME, &mut io::stdout());
    Ok(())
}

fn load_or_init_config(paths: &mut AppPaths, common: &CommonOpts) -> Result<AppConfig> {
    if !paths.config_file.exists() {
        if common.dry_run {
            info!(
                "dry-run: would create default config at {}",
                paths.config_file.display()
            );
        } else {
            write_default_config(&paths.config_file)?;
        }
    }

    let env_prefix = env_prefix();
    let built = Config::builder()
        .set_default("templates.prefer_local", true)?
        .set_default("detection.max_depth", 10_i64)?
        .set_default("detection.detect_os", true)?
        .set_default("detection.detect_ide", true)?
        .add_source(
            File::from(paths.config_file.as_path())
                .format(FileFormat::Toml)
                .required(false),
        )
        .add_source(Environment::with_prefix(env_prefix.as_str()).separator("__"))
        .build()?;

    let config: AppConfig = built.try_deserialize()?;
    Ok(config)
}

fn write_default_config(path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("creating config directory {parent:?}"))?;
    }

    let config_content = r#"# Configuration for ignr
# Auto-detect languages/tools and generate .gitignore files

[templates]
# Local directory containing custom .gitignore templates
# template_dir = "~/.config/ignr/templates"

# Remote URL to fetch templates from (gitignore.io compatible API)
template_url = "https://www.toptal.com/developers/gitignore/api"

# Whether to prefer local/custom templates over embedded ones
prefer_local = true

# Templates to always include in generated .gitignore
# always_include = ["macos", "vscode"]

[detection]
# Maximum directory depth to scan for technology detection
max_depth = 10

# Whether to auto-detect OS and add OS-specific patterns
detect_os = true

# Whether to detect IDE/editor directories and add patterns
detect_ide = true

[paths]
# Override the data directory (defaults to XDG_DATA_HOME/ignr)
# Synced and embedded templates are stored here
# data_dir = "~/.local/share/ignr"

# Override the cache directory (defaults to XDG_CACHE_HOME/ignr)
# cache_dir = "~/.cache/ignr"
"#;

    fs::write(path, config_content)
        .with_context(|| format!("writing config file to {}", path.display()))
}

fn expand_path(path: PathBuf) -> Result<PathBuf> {
    if let Some(text) = path.to_str() {
        expand_str_path(text)
    } else {
        Ok(path)
    }
}

fn expand_str_path(text: &str) -> Result<PathBuf> {
    let expanded = shellexpand::full(text).context("expanding path")?;
    Ok(PathBuf::from(expanded.to_string()))
}

fn default_config_dir() -> Result<PathBuf> {
    if let Some(dir) = env::var_os("XDG_CONFIG_HOME").filter(|v| !v.is_empty()) {
        let mut path = PathBuf::from(dir);
        path.push(APP_NAME);
        return Ok(path);
    }

    if let Some(mut dir) = dirs::config_dir() {
        dir.push(APP_NAME);
        return Ok(dir);
    }

    dirs::home_dir()
        .map(|home| home.join(".config").join(APP_NAME))
        .ok_or_else(|| anyhow!("unable to determine configuration directory"))
}

fn default_data_dir() -> Result<PathBuf> {
    if let Some(dir) = env::var_os("XDG_DATA_HOME").filter(|v| !v.is_empty()) {
        return Ok(PathBuf::from(dir).join(APP_NAME));
    }

    if let Some(mut dir) = dirs::data_dir() {
        dir.push(APP_NAME);
        return Ok(dir);
    }

    dirs::home_dir()
        .map(|home| home.join(".local/share").join(APP_NAME))
        .ok_or_else(|| anyhow!("unable to determine data directory"))
}

fn default_cache_dir() -> Result<PathBuf> {
    if let Some(dir) = env::var_os("XDG_CACHE_HOME").filter(|v| !v.is_empty()) {
        return Ok(PathBuf::from(dir).join(APP_NAME));
    }

    if let Some(mut dir) = dirs::cache_dir() {
        dir.push(APP_NAME);
        return Ok(dir);
    }

    dirs::home_dir()
        .map(|home| home.join(".cache").join(APP_NAME))
        .ok_or_else(|| anyhow!("unable to determine cache directory"))
}

fn env_prefix() -> String {
    APP_NAME
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_uppercase()
            } else {
                '_'
            }
        })
        .collect()
}

impl fmt::Display for AppPaths {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "config: {}, data: {}, cache: {}",
            self.config_file.display(),
            self.data_dir.display(),
            self.cache_dir.display(),
        )
    }
}
