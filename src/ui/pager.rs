use std::fmt::Write as FmtWrite;
use std::path::Path;
use std::time::{Duration, Instant};

use bubbles_rs::viewport;
use bubbletea_rs::{batch, cmd, message::msg, Cmd, KeyCode, KeyPressMsg, Msg, WindowSizeMsg};
use glamour_rs::{Style as GlamourStyle, TermRenderer};

use super::config::TuiConfig;
use super::markdown::Markdown;
use super::styles::Styles;
use crate::utils::{is_markdown_file, wrap_code_block};

const STATUS_BAR_HEIGHT: usize = 1;
const LINE_NUMBER_WIDTH: usize = 4;
const STATUS_MESSAGE_TIMEOUT: Duration = Duration::from_secs(3);

// --- Custom messages ----------------------------------------------------------

/// Glamour has finished rendering the content.
pub struct ContentRenderedMsg(pub String);

/// File on disk has changed — reload it.
pub struct ReloadMsg;

/// Status message display timeout has expired.
pub struct StatusMessageTimeoutMsg;

/// Editor process has finished.
pub struct EditorFinishedMsg(pub Option<String>);

/// Error message from the pager.
pub struct PagerErrMsg(pub String);

// --- Model --------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PagerState {
    Browse,
    StatusMessage,
}

pub struct PagerModel {
    pub viewport: viewport::Model,
    pub state: PagerState,
    pub show_help: bool,
    pub current_doc: Markdown,
    pub status_message: String,
    status_message_deadline: Option<Instant>,
    pub width: usize,
    pub height: usize,
    pub cfg: TuiConfig,
}

impl PagerModel {
    pub fn new(cfg: TuiConfig) -> Self {
        let mut vp = viewport::Model::new();
        vp.set_width(0);
        vp.set_height(0);
        Self {
            viewport: vp,
            state: PagerState::Browse,
            show_help: false,
            current_doc: Markdown::new(
                String::new(),
                String::new(),
                std::time::SystemTime::now(),
            ),
            status_message: String::new(),
            status_message_deadline: None,
            width: 0,
            height: 0,
            cfg,
        }
    }

    pub fn set_size(&mut self, w: usize, h: usize) {
        self.width = w;
        self.height = h;
        self.viewport.set_width(w);
        self.viewport.set_height(h.saturating_sub(STATUS_BAR_HEIGHT));
    }

    pub fn unload(&mut self) {
        self.show_help = false;
        self.state = PagerState::Browse;
        self.viewport.set_content("");
        self.viewport.set_y_offset(0);
    }

    pub fn show_status_message_cmd(&mut self, message: &str) -> Option<Cmd> {
        self.state = PagerState::StatusMessage;
        self.status_message = message.to_owned();
        self.status_message_deadline = Some(Instant::now() + STATUS_MESSAGE_TIMEOUT);
        Some(cmd(async { Some(msg(StatusMessageTimeoutMsg)) }))
    }

    fn check_status_timeout(&mut self) {
        if let Some(dl) = self.status_message_deadline {
            if Instant::now() >= dl {
                self.state = PagerState::Browse;
                self.status_message_deadline = None;
            }
        }
    }

    pub fn update(&mut self, message: &Msg) -> Option<Cmd> {
        self.check_status_timeout();

        let mut cmds: Vec<Option<Cmd>> = Vec::new();

        if message.is::<StatusMessageTimeoutMsg>() {
            self.state = PagerState::Browse;
            return None;
        }

        if let Some(rendered) = message.downcast_ref::<ContentRenderedMsg>() {
            self.viewport.set_content(&rendered.0);
            return None;
        }

        if message.is::<ReloadMsg>() || message.is::<EditorFinishedMsg>() {
            cmds.push(Some(self.load_current_doc_cmd()));
            return batch(cmds);
        }

        if let Some(size) = message.downcast_ref::<WindowSizeMsg>() {
            self.set_size(size.width as usize, size.height as usize);
            cmds.push(Some(self.render_glamour_cmd()));
        }

        if let Some(key) = message.downcast_ref::<KeyPressMsg>() {
            match key.0.code {
                KeyCode::Char('q') | KeyCode::Esc => {
                    if self.state != PagerState::Browse {
                        self.state = PagerState::Browse;
                        return None;
                    }
                    // Parent model handles returning to stash.
                }
                KeyCode::Home | KeyCode::Char('g') => {
                    self.viewport.goto_top();
                }
                KeyCode::End | KeyCode::Char('G') => {
                    self.viewport.goto_bottom();
                }
                KeyCode::Char('d') => {
                    self.viewport.half_page_down();
                }
                KeyCode::Char('u') => {
                    self.viewport.half_page_up();
                }
                KeyCode::Char('c') => {
                    let body = self.current_doc.body.clone();
                    if let Ok(mut ctx) = arboard::Clipboard::new() {
                        let _ = ctx.set_text(&body);
                    }
                    cmds.push(self.show_status_message_cmd("Copied contents"));
                }
                KeyCode::Char('e') => {
                    if !self.current_doc.local_path.is_empty() {
                        let path = self.current_doc.local_path.clone();
                        let lineno = (self.viewport.total_line_count() as f64
                            * self.viewport.scroll_percent())
                            as usize;
                        cmds.push(Some(open_editor_cmd(path, lineno)));
                    }
                }
                KeyCode::Char('r') => {
                    cmds.push(Some(self.load_current_doc_cmd()));
                }
                KeyCode::Char('?') => {
                    self.show_help = !self.show_help;
                }
                _ => {}
            }
        }

        cmds.push(self.viewport.update(message));

        batch(cmds)
    }

    pub fn view(&self, styles: &Styles) -> String {
        let mut b = String::new();
        let _ = write!(b, "{}\n", self.viewport.view());
        self.status_bar_view(&mut b, styles);
        if self.show_help {
            let _ = write!(b, "\n{}", self.help_view(styles));
        }
        b
    }

    fn status_bar_view(&self, b: &mut String, styles: &Styles) {
        let show_msg = self.state == PagerState::StatusMessage;

        let logo = styles.logo_style(" glow ");

        let percent = self.viewport.scroll_percent().clamp(0.0, 1.0);
        let scroll_pct = format!(" {:3.0}% ", percent * 100.0);
        let scroll_pct = if show_msg {
            styles.status_bar_message_style(&scroll_pct)
        } else {
            styles.status_bar_scroll_pos_style(&scroll_pct)
        };

        let help_note = if show_msg {
            styles.status_bar_message_help_style(" ? Help ")
        } else {
            styles.status_bar_help_style(" ? Help ")
        };

        let note_raw = if show_msg {
            format!(" {} ", self.status_message)
        } else {
            format!(" {} ", self.current_doc.note)
        };
        let note = if show_msg {
            styles.status_bar_message_style(&note_raw)
        } else {
            styles.status_bar_note_style(&note_raw)
        };

        let _ = write!(b, "{}{}{}{}", logo, note, scroll_pct, help_note);
    }

    fn help_view(&self, styles: &Styles) -> String {
        let s = "\n\
k/↑      up                  g/home  go to top\n\
j/↓      down                G/end   go to bottom\n\
b/pgup   page up             c       copy contents\n\
f/pgdn   page down           e       edit this document\n\
u        ½ page up           r       reload this document\n\
d        ½ page down         esc     back to files\n\
                             q       quit\n";
        styles.help_view_style(s)
    }

    pub fn render_glamour_cmd(&self) -> Cmd {
        let body = self.current_doc.body.clone();
        let note = self.current_doc.note.clone();
        let vp_width = self.viewport.width();
        let cfg = self.cfg.clone();

        cmd(async move {
            let s = tokio::task::spawn_blocking(move || {
                glamour_render(&body, &note, vp_width, &cfg)
            })
            .await
            .unwrap_or_default();
            Some(msg(ContentRenderedMsg(s)))
        })
    }

    fn load_current_doc_cmd(&self) -> Cmd {
        let path = self.current_doc.local_path.clone();
        cmd(async move {
            match tokio::fs::read_to_string(&path).await {
                Ok(body) => Some(msg(ContentRenderedMsg(body))),
                Err(e) => Some(msg(PagerErrMsg(e.to_string()))),
            }
        })
    }
}

// --- Helpers ------------------------------------------------------------------

fn select_glamour_style(style_name: &str, is_dark: bool) -> GlamourStyle {
    match style_name {
        "dark" => GlamourStyle::Dark,
        "light" => GlamourStyle::Light,
        "dracula" => GlamourStyle::DraculaDark,
        "tokyo-night" => GlamourStyle::TokyoNight,
        "ascii" => GlamourStyle::Ascii,
        "notty" => GlamourStyle::Notty,
        _ => {
            if is_dark { GlamourStyle::Dark } else { GlamourStyle::Light }
        }
    }
}

fn glamour_render(body: &str, note: &str, vp_width: usize, cfg: &TuiConfig) -> String {
    if !cfg.glamour_enabled {
        return body.to_owned();
    }

    let is_code = !is_markdown_file(note);
    let is_dark = crate::utils::has_dark_background();
    let style = select_glamour_style(&cfg.glamour_style, is_dark);

    let max_width = cfg.glamour_max_width as usize;
    let width = if is_code { 0 } else { vp_width.min(max_width) };

    let renderer = match TermRenderer::new(style) {
        Ok(r) => r.with_word_wrap(width),
        Err(_) => return body.to_owned(),
    };

    let content = if is_code {
        let ext = Path::new(note)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");
        wrap_code_block(body, ext)
    } else {
        body.to_owned()
    };

    let out = match renderer.render(&content) {
        Ok(s) => s,
        Err(_) => return body.to_owned(),
    };

    if cfg.show_line_numbers || is_code {
        out.lines()
            .enumerate()
            .map(|(i, line)| format!("{:>width$} {}\n", i + 1, line, width = LINE_NUMBER_WIDTH))
            .collect()
    } else {
        out
    }
}

fn open_editor_cmd(path: String, _lineno: usize) -> Cmd {
    let editor = std::env::var("EDITOR")
        .or_else(|_| std::env::var("VISUAL"))
        .unwrap_or_else(|_| "vi".into());
    cmd(async move {
        let fields = shlex::split(&editor).unwrap_or_else(|| vec!["vi".into()]);
        if fields.is_empty() {
            return Some(msg(EditorFinishedMsg(None)));
        }
        // Run in blocking task to avoid blocking the async runtime.
        let result = tokio::task::spawn_blocking(move || {
            std::process::Command::new(&fields[0])
                .args(&fields[1..])
                .arg(&path)
                .stdin(std::process::Stdio::inherit())
                .stdout(std::process::Stdio::inherit())
                .stderr(std::process::Stdio::inherit())
                .status()
                .err()
                .map(|e| e.to_string())
        })
        .await
        .unwrap_or(None);
        Some(msg(EditorFinishedMsg(result)))
    })
}
