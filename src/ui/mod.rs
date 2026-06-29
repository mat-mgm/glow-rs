pub mod config;
pub mod keys;
pub mod markdown;
pub mod pager;
pub mod sort;
pub mod stash;
pub mod stashhelp;
pub mod stashitem;
pub mod styles;

use bubbletea_rs::{
    batch, cmd,
    command::quit,
    message::msg,
    Cmd, KeyCode, KeyPressMsg, Model, Msg, View, WindowSizeMsg,
};

use config::TuiConfig;
use markdown::Markdown;
use pager::{ContentRenderedMsg, PagerModel};
use stash::{FetchedMarkdownMsg, FilterState, StashModel};
use styles::Styles;

pub type BoxError = Box<dyn std::error::Error + Send + Sync>;

// --- App state ---------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq)]
enum AppState {
    ShowStash,
    ShowDocument,
}

// --- File search result message ----------------------------------------------

/// All local markdown files found in the directory search.
struct AllFilesFoundMsg(Vec<(String, String, std::time::SystemTime)>);

// --- App model ---------------------------------------------------------------

pub struct AppModel {
    state: AppState,
    pager: PagerModel,
    stash: StashModel,
    styles: Styles,
    cfg: TuiConfig,
    width: usize,
    height: usize,
    fatal_err: Option<String>,
}

impl AppModel {
    fn new(cfg: TuiConfig) -> Self {
        let is_dark = crate::utils::has_dark_background();
        let styles = Styles::new(is_dark);
        let pager = PagerModel::new(cfg.clone());
        let stash = StashModel::new(cfg.clone(), &styles);
        AppModel {
            state: AppState::ShowStash,
            pager,
            stash,
            styles,
            cfg,
            width: 0,
            height: 0,
            fatal_err: None,
        }
    }

    fn unload_document(&mut self) -> Option<Cmd> {
        self.state = AppState::ShowStash;
        self.pager.unload();
        None
    }
}

impl Model for AppModel {
    fn init(&mut self) -> Option<Cmd> {
        let spinner_tick = self.stash.spinner.tick();

        if self.state == AppState::ShowDocument {
            let content = self.pager.current_doc.body.clone();
            let doc_path = self.pager.current_doc.local_path.clone();

            if !content.is_empty() {
                return batch(vec![
                    Some(spinner_tick),
                    Some(cmd(async move {
                        Some(msg(ContentRenderedMsg(content)))
                    })),
                ]);
            }
            return batch(vec![
                Some(spinner_tick),
                Some(cmd(async move {
                    match tokio::fs::read_to_string(&doc_path).await {
                        Ok(body) => Some(msg(ContentRenderedMsg(body))),
                        Err(e) => Some(msg(pager::PagerErrMsg(e.to_string()))),
                    }
                })),
            ]);
        }

        // Stash mode: start local file search.
        let path = self.cfg.path.clone();
        let show_all = self.cfg.show_all_files;
        batch(vec![
            Some(spinner_tick),
            Some(cmd(async move {
                let cwd = resolve_cwd(&path);
                let cwd_clone = cwd.clone();
                let files = tokio::task::spawn_blocking(move || {
                    find_markdown_files(&cwd_clone, show_all)
                        .into_iter()
                        .map(|(p, t)| {
                            let note = strip_abs_path(&p, &cwd_clone);
                            (p, note, t)
                        })
                        .collect::<Vec<_>>()
                })
                .await
                .unwrap_or_default();
                Some(msg(AllFilesFoundMsg(files)))
            })),
        ])
    }

    fn update(&mut self, message: Msg) -> Option<Cmd> {
        if self.fatal_err.is_some() {
            if message.is::<KeyPressMsg>() {
                return Some(quit());
            }
            return None;
        }

        let mut cmds: Vec<Option<Cmd>> = Vec::new();

        // Global key handling.
        if let Some(key) = message.downcast_ref::<KeyPressMsg>() {
            match key.0.code {
                KeyCode::Char('q') | KeyCode::Char('Q') => {
                    if self.stash.filter_state != FilterState::Filtering {
                        return Some(quit());
                    }
                }
                KeyCode::Esc => {
                    if self.state == AppState::ShowDocument
                        || self.stash.view_state == stash::StashViewState::LoadingDocument
                    {
                        cmds.push(self.unload_document());
                        return batch(cmds);
                    }
                }
                KeyCode::Left | KeyCode::Char('h') | KeyCode::Delete => {
                    if self.state == AppState::ShowDocument {
                        cmds.push(self.unload_document());
                        return batch(cmds);
                    }
                }
                KeyCode::Char('r') => {
                    if self.state == AppState::ShowStash
                        && self.stash.filter_state != FilterState::Filtering
                    {
                        self.stash.markdowns.clear();
                        self.stash.loaded = false;
                        let path = self.cfg.path.clone();
                        let show_all = self.cfg.show_all_files;
                        return Some(cmd(async move {
                            let cwd = resolve_cwd(&path);
                            let cwd_clone = cwd.clone();
                            let files = tokio::task::spawn_blocking(move || {
                                find_markdown_files(&cwd_clone, show_all)
                                    .into_iter()
                                    .map(|(p, t)| {
                                        let note = strip_abs_path(&p, &cwd_clone);
                                        (p, note, t)
                                    })
                                    .collect::<Vec<_>>()
                            })
                            .await
                            .unwrap_or_default();
                            Some(msg(AllFilesFoundMsg(files)))
                        }));
                    }
                }
                _ => {}
            }
        }

        // Window resize.
        if let Some(size) = message.downcast_ref::<WindowSizeMsg>() {
            self.width = size.width as usize;
            self.height = size.height as usize;
            self.stash.set_size(self.width, self.height);
            self.pager.set_size(self.width, self.height);
        }

        // File search result: populate stash.
        if let Some(all) = message.downcast_ref::<AllFilesFoundMsg>() {
            for (path, note, modtime) in &all.0 {
                let md = Markdown::new(path.clone(), note.clone(), *modtime);
                self.stash.add_markdown(md);
            }
            self.stash.loaded = true;
            return None;
        }

        // FetchedMarkdownMsg: open doc in pager.
        if let Some(fetched) = message.downcast_ref::<FetchedMarkdownMsg>() {
            self.pager.current_doc = fetched.0.clone();
            cmds.push(Some(self.pager.render_glamour_cmd()));
        }

        // ContentRenderedMsg: switch view to pager.
        if message.is::<ContentRenderedMsg>() {
            self.state = AppState::ShowDocument;
        }

        // Delegate to sub-models.
        match self.state {
            AppState::ShowStash => {
                cmds.push(self.stash.update(&message));
            }
            AppState::ShowDocument => {
                cmds.push(self.pager.update(&message));
            }
        }

        batch(cmds)
    }

    fn view(&self) -> View {
        if let Some(ref err) = self.fatal_err {
            return View::new(format!(
                "\n   ERROR\n\n   {}\n\n   press any key to exit",
                err
            ));
        }
        let content = match self.state {
            AppState::ShowDocument => self.pager.view(&self.styles),
            AppState::ShowStash => self.stash.view(&self.styles),
        };
        View::new(content)
    }
}

// --- File finder -------------------------------------------------------------

fn resolve_cwd(path: &str) -> String {
    if path.is_empty() {
        std::env::current_dir()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| ".".into())
    } else {
        path.to_owned()
    }
}

fn find_markdown_files(dir: &str, show_all: bool) -> Vec<(String, std::time::SystemTime)> {
    use ignore::WalkBuilder;
    let mut results = Vec::new();
    let mut builder = WalkBuilder::new(dir);
    builder.hidden(!show_all);
    for entry in builder.build().flatten() {
        let path = entry.path();
        if path.is_file() && crate::utils::is_markdown_file(path.to_str().unwrap_or("")) {
            let modtime = entry
                .metadata()
                .ok()
                .and_then(|m| m.modified().ok())
                .unwrap_or(std::time::SystemTime::now());
            results.push((path.to_string_lossy().to_string(), modtime));
        }
    }
    results
}

// --- Public entry point ------------------------------------------------------

/// Launch the TUI program.
pub fn new_program(cfg: TuiConfig, content: String) -> Result<(), BoxError> {
    let mut app = AppModel::new(cfg.clone());
    let path = cfg.path.clone();

    if !content.is_empty() {
        app.state = AppState::ShowDocument;
        app.pager.current_doc = Markdown::new(
            String::new(),
            "(stdin)".into(),
            std::time::SystemTime::now(),
        );
        app.pager.current_doc.body = content;
    } else if !path.is_empty() {
        match std::fs::metadata(&path) {
            Err(e) => return Err(Box::new(e)),
            Ok(m) if m.is_file() => {
                let cwd = std::env::current_dir()
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_default();
                let note = strip_abs_path(&path, &cwd);
                let modtime = m.modified().unwrap_or(std::time::SystemTime::now());
                app.state = AppState::ShowDocument;
                app.pager.current_doc = Markdown::new(path, note, modtime);
            }
            _ => {}
        }
    }

    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?
        .block_on(async {
            bubbletea_rs::Program::new(app)
                .run()
                .await
                .map(|_| ())
                .map_err(|e| Box::new(e) as BoxError)
        })
}

fn strip_abs_path(full_path: &str, cwd: &str) -> String {
    let fp = std::fs::canonicalize(full_path)
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|_| full_path.to_owned());
    let cp = std::fs::canonicalize(cwd)
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|_| cwd.to_owned());
    let prefix = format!("{}/", cp);
    fp.strip_prefix(&prefix).unwrap_or(&fp).to_owned()
}
