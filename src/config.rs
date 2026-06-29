use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub const DEFAULT_CONFIG: &str = r#"# style name or JSON path (default "auto")
style: "auto"
# enable mouse support (TUI-mode only)
mouse: false
# use pager to display markdown
pager: false
# word-wrap at width (0 = terminal width)
width: 80
# show all files, including hidden and ignored (TUI-mode only)
all: true
# show line numbers (TUI-mode only)
show_line_numbers: false
# preserve newlines in output
preserve_new_lines: false
"#;

/// Application configuration (mirrors Glow's viper config).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    #[serde(default = "default_style")]
    pub style: String,
    #[serde(default)]
    pub pager: bool,
    #[serde(default)]
    pub tui: bool,
    #[serde(default)]
    pub mouse: bool,
    #[serde(default = "default_width")]
    pub width: u32,
    #[serde(default = "default_true")]
    pub all: bool,
    #[serde(default)]
    pub show_line_numbers: bool,
    #[serde(default)]
    pub preserve_new_lines: bool,
}

fn default_style() -> String { "auto".into() }
fn default_width() -> u32 { 80 }
fn default_true() -> bool { true }

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            style: "auto".into(),
            pager: false,
            tui: false,
            mouse: false,
            width: 80,
            all: true,
            show_line_numbers: false,
            preserve_new_lines: false,
        }
    }
}

/// Return the platform config directory for glow (XDG on Linux).
pub fn config_dir() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("glow"))
}

/// Return the path to the default config file.
pub fn config_file_path() -> Option<PathBuf> {
    config_dir().map(|d| d.join("glow.yml"))
}

/// Load config from the default location, returning defaults if not found.
pub fn load_config() -> AppConfig {
    let Some(path) = config_file_path() else {
        return AppConfig::default();
    };
    let Ok(content) = std::fs::read_to_string(&path) else {
        return AppConfig::default();
    };
    match serde_yaml::from_str(&content) {
        Ok(cfg) => cfg,
        Err(e) => {
            eprintln!("glow: could not parse config file {}: {}", path.display(), e);
            AppConfig::default()
        }
    }
}

/// Ensure the default config file exists, creating it if necessary.
pub fn ensure_config_file() -> std::io::Result<PathBuf> {
    let path = config_file_path().ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::NotFound, "could not find config dir")
    })?;
    if !path.exists() {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&path, DEFAULT_CONFIG)?;
    }
    Ok(path)
}
