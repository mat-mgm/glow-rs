pub mod config;
pub mod keys;
pub mod markdown;
pub mod pager;
pub mod sort;
pub mod stash;
pub mod stashhelp;
pub mod stashitem;
pub mod styles;

use config::TuiConfig;

pub type BoxError = Box<dyn std::error::Error + Send + Sync>;

/// Launch the TUI program. Placeholder until Phase 6.
pub fn new_program(_cfg: TuiConfig, _content: String) -> Result<(), BoxError> {
    eprintln!("glow: TUI mode is not yet implemented");
    Ok(())
}
