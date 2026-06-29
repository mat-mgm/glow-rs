/// TUI-specific configuration — mirrors Glow's `ui.Config`.
#[derive(Debug, Clone)]
pub struct TuiConfig {
    pub show_all_files: bool,
    pub show_line_numbers: bool,
    pub glamour_max_width: u32,
    pub glamour_style: String,
    pub enable_mouse: bool,
    pub preserve_new_lines: bool,
    /// Working directory or file path to open.
    pub path: String,
    /// Enable high-performance (buffered) pager rendering.
    pub high_performance_pager: bool,
    /// Enable glamour rendering in the pager.
    pub glamour_enabled: bool,
}

impl Default for TuiConfig {
    fn default() -> Self {
        Self {
            show_all_files: true,
            show_line_numbers: false,
            glamour_max_width: 80,
            glamour_style: "auto".into(),
            enable_mouse: false,
            preserve_new_lines: false,
            path: String::new(),
            high_performance_pager: true,
            glamour_enabled: true,
        }
    }
}
