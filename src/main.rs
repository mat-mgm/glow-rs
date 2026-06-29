mod config;
mod source;
mod url;
mod utils;
pub mod ui;

use clap::{Parser, Subcommand};
use glamour_rs::{Style, TermRenderer};
use std::io::{self, Read, Write};
use std::process;

use config::{ensure_config_file, load_config, AppConfig};
use source::{base_url_from_source, fetch_url, is_http_url, is_url, source_from_arg, stdin_is_pipe, Source};
use url::readme_url;
use utils::{has_dark_background, is_markdown_file, wrap_code_block};

type BoxError = Box<dyn std::error::Error + Send + Sync>;

/// Render markdown on the CLI, with pizzazz!
#[derive(Parser, Debug)]
#[command(
    name = "glow",
    about = "Render markdown on the CLI, with pizzazz!",
    version
)]
struct Cli {
    /// Source: file, directory, URL, stdin (-), or github://user/repo
    source: Option<String>,

    /// Config file path
    #[arg(long, global = true)]
    config: Option<String>,

    /// Display with pager
    #[arg(short = 'p', long)]
    pager: Option<bool>,

    /// Display with TUI
    #[arg(short = 't', long)]
    tui: Option<bool>,

    /// Style name or JSON path
    #[arg(short = 's', long)]
    style: Option<String>,

    /// Word-wrap at width (0 = terminal width)
    #[arg(short = 'w', long)]
    width: Option<u32>,

    /// Show all files including hidden (TUI-mode only)
    #[arg(short = 'a', long)]
    all: Option<bool>,

    /// Show line numbers (TUI-mode only)
    #[arg(short = 'l', long = "line-numbers")]
    line_numbers: Option<bool>,

    /// Preserve newlines in output
    #[arg(short = 'n', long = "preserve-new-lines")]
    preserve_new_lines: Option<bool>,

    /// Enable mouse wheel (TUI-mode only)
    #[arg(short = 'm', long, hide = true)]
    mouse: Option<bool>,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Edit the glow config file
    Config,
}

fn main() {
    let cli = Cli::parse();
    if let Err(e) = run(cli) {
        eprintln!("glow: {}", e);
        process::exit(1);
    }
}

fn run(cli: Cli) -> Result<(), BoxError> {
    // Handle subcommands first.
    if let Some(Commands::Config) = cli.command {
        return cmd_config();
    }

    // Load file config and apply CLI overrides.
    let mut cfg = load_config();
    apply_overrides(&mut cfg, &cli);

    // Validate style.
    validate_style(&cfg.style)?;

    // Detect terminal width if not set.
    if cfg.width == 0 {
        cfg.width = terminal_width().unwrap_or(80).min(120) as u32;
    }

    let source_arg = cli.source;

    // If stdin is a pipe, read from it.
    if stdin_is_pipe() && source_arg.as_deref() != Some("-") {
        let mut buf = Vec::new();
        io::stdin().read_to_end(&mut buf)?;
        let src = Source { content: buf, url: String::new() };
        return execute_cli(&cfg, src, &mut io::stdout());
    }

    match source_arg.as_deref() {
        // No arg: TUI on current directory.
        None => run_tui(&cfg, "", ""),

        // Explicit source arg.
        Some(arg) => {
            // Resolve source (may be URL, file, dir, GitHub, etc.)
            let src = resolve_source(arg)?;

            // If tui flag is set, or the source is a directory, use TUI.
            if cfg.tui {
                let path = if !is_url(&src.url) { src.url.clone() } else { String::new() };
                let content = String::from_utf8_lossy(&src.content).into_owned();
                return run_tui(&cfg, &path, &content);
            }

            // Check if source is a directory (url is an abs path to a dir).
            // If the arg was a dir path, source_from_arg already found a README,
            // so we don't need a separate check here.

            execute_cli(&cfg, src, &mut io::stdout())
        }
    }
}

/// Resolve a source argument into a Source struct.
fn resolve_source(arg: &str) -> Result<Source, BoxError> {
    // GitHub / GitLab pseudo-protocol or plain GitHub/GitLab URL.
    if let Some(result) = readme_url(arg) {
        return result;
    }

    // Plain HTTP(S) URL.
    if is_http_url(arg)
        || (arg.contains("://")
            && !arg.starts_with("github://")
            && !arg.starts_with("gitlab://"))
    {
        return fetch_url(arg);
    }

    // Local file / directory / stdin.
    source_from_arg(arg)
}

fn execute_cli(
    cfg: &AppConfig,
    src: Source,
    w: &mut dyn Write,
) -> Result<(), BoxError> {
    let content_bytes = utils::remove_frontmatter(&src.content);
    let base_url = base_url_from_source(&src.url).unwrap_or_default();

    let is_code = !is_markdown_file(&src.url);
    let style = resolve_glamour_style(&cfg.style, is_code);

    let renderer = TermRenderer::new(style)?
        .with_word_wrap(cfg.width as usize);

    let raw_content = String::from_utf8_lossy(content_bytes);
    let content = if is_code {
        let ext = std::path::Path::new(&src.url)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");
        wrap_code_block(&raw_content, ext)
    } else {
        raw_content.into_owned()
    };
    let _ = base_url; // glamour-rs doesn't expose base_url yet

    let out = renderer.render(&content)?;

    if cfg.pager {
        return pipe_to_pager(&out);
    }

    write!(w, "{}", out)?;
    Ok(())
}

/// Select the appropriate glamour style for the given style name.
fn resolve_glamour_style(style: &str, _is_code: bool) -> Style {
    match style {
        "dark" => Style::Dark,
        "light" => Style::Light,
        "dracula" => Style::DraculaDark,
        "tokyo-night" => Style::TokyoNight,
        "ascii" => Style::Ascii,
        "notty" => Style::Notty,
        "auto" | "" => {
            if has_dark_background() { Style::Dark } else { Style::Light }
        }
        _ => {
            if has_dark_background() { Style::Dark } else { Style::Light }
        }
    }
}

fn pipe_to_pager(content: &str) -> Result<(), BoxError> {
    let pager_cmd = std::env::var("PAGER").unwrap_or_else(|_| "less -r".into());
    let fields = shlex::split(&pager_cmd).ok_or("invalid PAGER command")?;
    if fields.is_empty() { return Err("empty PAGER command".into()); }

    let mut child = process::Command::new(&fields[0])
        .args(&fields[1..])
        .stdin(process::Stdio::piped())
        .spawn()?;

    if let Some(stdin) = child.stdin.take() {
        let mut stdin = stdin;
        write!(stdin, "{}", content)?;
    }
    child.wait()?;
    Ok(())
}

fn validate_style(style: &str) -> Result<(), BoxError> {
    match style {
        "auto" | "dark" | "light" | "dracula" | "tokyo-night" | "ascii" | "notty" => Ok(()),
        _ => {
            // Check if it's a JSON file path.
            if std::path::Path::new(style).exists() {
                Ok(())
            } else {
                Err(format!("specified style does not exist: {}", style).into())
            }
        }
    }
}

fn apply_overrides(cfg: &mut AppConfig, cli: &Cli) {
    if let Some(v) = &cli.style { cfg.style = v.clone(); }
    if let Some(v) = cli.width { cfg.width = v; }
    if let Some(v) = cli.pager { cfg.pager = v; }
    if let Some(v) = cli.tui { cfg.tui = v; }
    if let Some(v) = cli.mouse { cfg.mouse = v; }
    if let Some(v) = cli.all { cfg.all = v; }
    if let Some(v) = cli.line_numbers { cfg.show_line_numbers = v; }
    if let Some(v) = cli.preserve_new_lines { cfg.preserve_new_lines = v; }
}

fn terminal_width() -> Option<usize> {
    use crossterm::terminal;
    terminal::size().ok().map(|(w, _)| w as usize)
}

fn cmd_config() -> Result<(), BoxError> {
    let path = ensure_config_file()?;
    let editor = std::env::var("EDITOR")
        .or_else(|_| std::env::var("VISUAL"))
        .unwrap_or_else(|_| "vi".into());
    let fields = shlex::split(&editor).ok_or("invalid EDITOR command")?;
    if fields.is_empty() { return Err("empty EDITOR command".into()); }

    process::Command::new(&fields[0])
        .args(&fields[1..])
        .arg(&path)
        .stdin(process::Stdio::inherit())
        .stdout(process::Stdio::inherit())
        .stderr(process::Stdio::inherit())
        .status()?;
    Ok(())
}

fn run_tui(
    cfg: &AppConfig,
    path: &str,
    content: &str,
) -> Result<(), BoxError> {
    let tui_cfg = ui::config::TuiConfig {
        show_all_files: cfg.all,
        show_line_numbers: cfg.show_line_numbers,
        glamour_max_width: cfg.width,
        glamour_style: cfg.style.clone(),
        enable_mouse: cfg.mouse,
        preserve_new_lines: cfg.preserve_new_lines,
        path: path.to_owned(),
        high_performance_pager: std::env::var("GLOW_HIGH_PERFORMANCE_PAGER")
            .map(|v| v != "0" && v != "false")
            .unwrap_or(true),
        glamour_enabled: std::env::var("GLOW_ENABLE_GLAMOUR")
            .map(|v| v != "0" && v != "false")
            .unwrap_or(true),
    };
    ui::new_program(tui_cfg, content.to_owned())?;
    Ok(())
}
