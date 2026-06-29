use std::fmt::Write as FmtWrite;
use std::time::Instant;

use bubbles_rs::{paginator, spinner, textinput};
use bubbletea_rs::{batch, cmd, message::msg, Cmd, KeyCode, KeyPressMsg, Msg, WindowSizeMsg};
use fuzzy_matcher::{skim::SkimMatcherV2, FuzzyMatcher};

use super::config::TuiConfig;
use super::markdown::Markdown;
use super::sort::sort_markdowns;
use super::stashitem::stash_item_view;
use super::styles::Styles;

const STASH_INDENT: usize = 1;
const STASH_VIEW_ITEM_HEIGHT: usize = 3;
const STASH_VIEW_TOP_PADDING: usize = 5;
const STASH_VIEW_BOTTOM_PADDING: usize = 3;
pub const STASH_VIEW_HORIZONTAL_PADDING: usize = 6;

// --- Custom messages ----------------------------------------------------------

/// A markdown document has been loaded from disk.
pub struct FetchedMarkdownMsg(pub Markdown);

/// The fuzzy-filter pass has completed.
pub struct FilteredMarkdownMsg(pub Vec<usize>); // indices into markdowns vec

/// The local file search has finished.
pub struct LocalFileSearchFinishedMsg;

/// A local markdown file was found.
pub struct FoundLocalFileMsg {
    pub path: String,
    pub note: String,
    pub modtime: std::time::SystemTime,
}

/// Error from the stash.
pub struct StashErrMsg(pub String);

// --- Section types -----------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SectionKey {
    Documents,
    Filter,
}

#[derive(Clone)]
pub struct Section {
    pub key: SectionKey,
    pub paginator: paginator::Model,
    pub cursor: usize,
}

impl Section {
    fn new(key: SectionKey) -> Self {
        Self {
            key,
            paginator: new_stash_paginator(),
            cursor: 0,
        }
    }
}

fn new_stash_paginator() -> paginator::Model {
    paginator::Model::new()
        .with_per_page(10)
}

// --- Filter state ------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum FilterState {
    Unfiltered,
    Filtering,
    FilterApplied,
}

// --- View state --------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum StashViewState {
    Ready,
    LoadingDocument,
    ShowingError,
}

// --- Status message ----------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum StatusMessageType {
    Normal,
    Subtle,
    Error,
}

#[derive(Clone)]
pub struct StatusMessage {
    pub kind: StatusMessageType,
    pub text: String,
}

impl StatusMessage {
    pub fn normal(text: &str) -> Self {
        Self { kind: StatusMessageType::Normal, text: text.to_owned() }
    }
    pub fn error(text: &str) -> Self {
        Self { kind: StatusMessageType::Error, text: text.to_owned() }
    }
    pub fn render(&self, styles: &Styles) -> String {
        match self.kind {
            StatusMessageType::Normal => styles.green_fg(&self.text),
            StatusMessageType::Subtle => styles.dim_green_fg(&self.text),
            StatusMessageType::Error => styles.red_fg(&self.text),
        }
    }
}

pub struct StatusMessageTimeoutMsg;

// --- Model -------------------------------------------------------------------

pub struct StashModel {
    pub spinner: spinner::Model,
    pub filter_input: textinput::Model,
    pub view_state: StashViewState,
    pub filter_state: FilterState,
    pub show_full_help: bool,
    pub show_status_message: bool,
    pub status_message: Option<StatusMessage>,
    status_message_deadline: Option<Instant>,

    pub sections: Vec<Section>,
    pub section_index: usize,

    pub loaded: bool,
    pub markdowns: Vec<Markdown>,
    pub filtered_indices: Vec<usize>,

    pub width: usize,
    pub height: usize,
    pub cfg: TuiConfig,

    pub err: Option<String>,
}

impl StashModel {
    pub fn new(cfg: TuiConfig, styles: &Styles) -> Self {
        let sp = spinner::Model::new()
            .with_spinner(spinner::line())
            .with_style(styles.stash_spinner_style());

        let mut fi = textinput::Model::new();
        fi.prompt = "Find: ".into();

        Self {
            spinner: sp,
            filter_input: fi,
            view_state: StashViewState::Ready,
            filter_state: FilterState::Unfiltered,
            show_full_help: false,
            show_status_message: false,
            status_message: None,
            status_message_deadline: None,
            sections: vec![Section::new(SectionKey::Documents)],
            section_index: 0,
            loaded: false,
            markdowns: Vec::new(),
            filtered_indices: Vec::new(),
            width: 0,
            height: 0,
            cfg,
            err: None,
        }
    }

    pub fn set_size(&mut self, w: usize, h: usize) {
        self.width = w;
        self.height = h;
        self.filter_input.set_width(w.saturating_sub(STASH_VIEW_HORIZONTAL_PADDING * 2 + 6));
        self.update_pagination();
    }

    pub fn current_section_ref(&self) -> &Section {
        &self.sections[self.section_index]
    }

    pub fn cursor(&self) -> usize {
        self.current_section_ref().cursor
    }

    fn set_cursor(&mut self, i: usize) {
        self.sections[self.section_index].cursor = i;
    }

    fn per_page(&self) -> usize {
        self.current_section_ref().paginator.per_page
    }

    fn total_pages(&self) -> usize {
        self.current_section_ref().paginator.total_pages
    }

    fn page(&self) -> usize {
        self.current_section_ref().paginator.page
    }

    fn set_page(&mut self, p: usize) {
        self.sections[self.section_index].paginator.page = p;
    }

    pub fn visible_markdowns(&self) -> Vec<&Markdown> {
        if self.filter_state == FilterState::Filtering
            || self.current_section_ref().key == SectionKey::Filter
        {
            self.filtered_indices.iter().map(|&i| &self.markdowns[i]).collect()
        } else {
            self.markdowns.iter().collect()
        }
    }

    fn selected_markdown(&self) -> Option<&Markdown> {
        let per_page = self.per_page();
        let page = self.page();
        let i = page * per_page + self.cursor();
        let visible = self.visible_markdowns();
        visible.get(i).copied()
    }

    pub fn loading_done(&self) -> bool { self.loaded }

    fn should_spin(&self) -> bool {
        !self.loading_done() || self.view_state == StashViewState::LoadingDocument
    }

    pub fn filter_applied(&self) -> bool {
        self.filter_state != FilterState::Unfiltered
    }

    fn update_pagination(&mut self) {
        let available_height = self.height
            .saturating_sub(STASH_VIEW_TOP_PADDING + STASH_VIEW_BOTTOM_PADDING);
        let per_page = (available_height / STASH_VIEW_ITEM_HEIGHT).max(1);
        let total = self.visible_markdowns().len().max(1);
        let sec = &mut self.sections[self.section_index];
        sec.paginator.per_page = per_page;
        sec.paginator.set_total_pages(total);
        if sec.paginator.page >= sec.paginator.total_pages {
            sec.paginator.page = sec.paginator.total_pages.saturating_sub(1);
        }
    }

    fn reset_filtering(&mut self) {
        self.filter_state = FilterState::Unfiltered;
        self.filter_input.reset();
        self.filtered_indices.clear();
        sort_markdowns(&mut self.markdowns);
        if self.sections.last().map(|s| s.key) == Some(SectionKey::Filter) {
            self.sections.pop();
        }
        if self.section_index >= self.sections.len() {
            self.section_index = 0;
        }
        self.update_pagination();
    }

    fn hide_status_message(&mut self) {
        self.show_status_message = false;
        self.status_message = None;
        self.status_message_deadline = None;
    }

    fn move_cursor_up(&mut self) {
        if self.cursor() == 0 && self.page() == 0 {
            return;
        }
        if self.cursor() > 0 {
            let c = self.cursor();
            self.set_cursor(c - 1);
        } else {
            self.sections[self.section_index].paginator.prev_page();
            let items = self.current_section_ref().paginator.items_on_page(self.visible_markdowns().len());
            self.set_cursor(items.saturating_sub(1));
        }
    }

    fn move_cursor_down(&mut self) {
        let visible = self.visible_markdowns().len();
        let items_on_page = self.current_section_ref().paginator.items_on_page(visible);
        let cursor = self.cursor();
        if cursor + 1 < items_on_page {
            self.set_cursor(cursor + 1);
        } else if !self.current_section_ref().paginator.on_last_page() {
            self.sections[self.section_index].paginator.next_page();
            self.set_cursor(0);
        }
    }

    pub fn add_markdown(&mut self, md: Markdown) {
        self.markdowns.push(md);
        if !self.filter_applied() {
            sort_markdowns(&mut self.markdowns);
        }
        self.update_pagination();
    }

    fn run_filter(&self) -> Vec<usize> {
        let query = self.filter_input.value();
        if query.is_empty() {
            return (0..self.markdowns.len()).collect();
        }
        let matcher = SkimMatcherV2::default();
        let mut scored: Vec<(i64, usize)> = self.markdowns.iter()
            .enumerate()
            .filter_map(|(i, md)| {
                matcher.fuzzy_match(&md.filter_value, &query).map(|s| (s, i))
            })
            .collect();
        scored.sort_by(|a, b| b.0.cmp(&a.0));
        scored.into_iter().map(|(_, i)| i).collect()
    }

    pub fn open_markdown_cmd(&mut self, md: Markdown) -> Option<Cmd> {
        self.view_state = StashViewState::LoadingDocument;
        let path = md.local_path.clone();
        let note = md.note.clone();
        let modtime = md.modtime;
        let tick = self.spinner.tick();
        let load = load_local_markdown_cmd(path, note, modtime);
        batch(vec![Some(load), Some(tick)])
    }

    // --- Update ---------------------------------------------------------------

    pub fn update(&mut self, message: &Msg) -> Option<Cmd> {
        let mut cmds: Vec<Option<Cmd>> = Vec::new();

        // Status timeout.
        if let Some(dl) = self.status_message_deadline {
            if Instant::now() >= dl {
                self.hide_status_message();
            }
        }

        if message.is::<StatusMessageTimeoutMsg>() {
            self.hide_status_message();
            return None;
        }

        if message.is::<LocalFileSearchFinishedMsg>() {
            self.loaded = true;
            return None;
        }

        if let Some(found) = message.downcast_ref::<FoundLocalFileMsg>() {
            let md = Markdown::new(found.path.clone(), found.note.clone(), found.modtime);
            self.add_markdown(md);
            return None;
        }

        if let Some(FetchedMarkdownMsg(_md)) = message.downcast_ref::<FetchedMarkdownMsg>() {
            self.view_state = StashViewState::Ready;
            // Signal to parent to switch to pager mode.
            return None;
        }

        if message.is::<StashErrMsg>() {
            self.view_state = StashViewState::ShowingError;
            return None;
        }

        if let Some(size) = message.downcast_ref::<WindowSizeMsg>() {
            self.set_size(size.width as usize, size.height as usize);
            return None;
        }

        // Spinner tick.
        if self.should_spin() {
            cmds.push(self.spinner.update(message));
        }

        if self.filter_state == FilterState::Filtering {
            cmds.push(self.handle_filtering(message));
        } else {
            match self.view_state {
                StashViewState::Ready => {
                    cmds.push(self.handle_browsing(message));
                }
                StashViewState::ShowingError => {
                    if message.is::<KeyPressMsg>() {
                        self.view_state = StashViewState::Ready;
                    }
                }
                _ => {}
            }
        }

        batch(cmds)
    }

    fn handle_browsing(&mut self, message: &Msg) -> Option<Cmd> {
        let mut cmds: Vec<Option<Cmd>> = Vec::new();
        let num_docs = self.visible_markdowns().len();

        if let Some(key) = message.downcast_ref::<KeyPressMsg>() {
            match key.0.code {
                KeyCode::Up | KeyCode::Char('k') => self.move_cursor_up(),
                KeyCode::Down | KeyCode::Char('j') => self.move_cursor_down(),
                KeyCode::Home | KeyCode::Char('g') => {
                    self.set_page(0);
                    self.set_cursor(0);
                }
                KeyCode::End | KeyCode::Char('G') => {
                    let tp = self.total_pages();
                    self.set_page(tp.saturating_sub(1));
                    let items = self.current_section_ref().paginator.items_on_page(num_docs);
                    self.set_cursor(items.saturating_sub(1));
                }
                KeyCode::Esc => {
                    if self.filter_applied() {
                        self.reset_filtering();
                    }
                }
                KeyCode::Tab | KeyCode::Char('L') => {
                    self.section_index = (self.section_index + 1) % self.sections.len();
                    self.update_pagination();
                }
                KeyCode::BackTab | KeyCode::Char('H') => {
                    if self.section_index == 0 {
                        self.section_index = self.sections.len().saturating_sub(1);
                    } else {
                        self.section_index -= 1;
                    }
                    self.update_pagination();
                }
                KeyCode::Char('e') => {
                    if let Some(md) = self.selected_markdown() {
                        let path = md.local_path.clone();
                        if !path.is_empty() {
                            cmds.push(Some(open_editor_stash_cmd(path)));
                        }
                    }
                }
                KeyCode::Enter => {
                    self.hide_status_message();
                    if num_docs > 0 {
                        if let Some(md) = self.selected_markdown().cloned() {
                            cmds.push(self.open_markdown_cmd(md));
                        }
                    }
                }
                KeyCode::Char('/') => {
                    self.hide_status_message();
                    for md in &mut self.markdowns {
                        md.build_filter_value();
                    }
                    self.filtered_indices = (0..self.markdowns.len()).collect();
                    self.set_page(0);
                    self.set_cursor(0);
                    self.filter_state = FilterState::Filtering;
                    self.filter_input.cursor_end();
                    cmds.push(self.filter_input.focus());
                    cmds.push(Some(bubbles_rs::cursor::blink()));
                }
                KeyCode::Char('?') => {
                    self.show_full_help = !self.show_full_help;
                    self.update_pagination();
                }
                _ => {}
            }
        }

        // Update paginator.
        cmds.push(self.sections[self.section_index].paginator.update(message));

        // Clamp cursor to page.
        let items_on_page = self.current_section_ref()
            .paginator
            .items_on_page(self.visible_markdowns().len());
        if self.cursor() >= items_on_page && items_on_page > 0 {
            self.set_cursor(items_on_page - 1);
        }

        batch(cmds)
    }

    fn handle_filtering(&mut self, message: &Msg) -> Option<Cmd> {
        let mut cmds: Vec<Option<Cmd>> = Vec::new();

        if let Some(key) = message.downcast_ref::<KeyPressMsg>() {
            match key.0.code {
                KeyCode::Esc => {
                    self.reset_filtering();
                }
                KeyCode::Enter | KeyCode::Tab | KeyCode::BackTab => {
                    self.hide_status_message();
                    let visible_len = self.visible_markdowns().len();
                    let first_md = if visible_len == 1 {
                        self.visible_markdowns().first().cloned().cloned()
                    } else {
                        None
                    };
                    if visible_len == 0 {
                        self.view_state = StashViewState::Ready;
                        self.reset_filtering();
                    } else if let Some(md) = first_md {
                        self.view_state = StashViewState::Ready;
                        self.reset_filtering();
                        cmds.push(self.open_markdown_cmd(md));
                    } else {
                        if self.sections.last().map(|s| s.key) != Some(SectionKey::Filter) {
                            self.sections.push(Section::new(SectionKey::Filter));
                        }
                        self.section_index = self.sections.len() - 1;
                        self.filter_input.blur();
                        self.filter_state = if self.filter_input.value().is_empty() {
                            self.reset_filtering();
                            FilterState::Unfiltered
                        } else {
                            FilterState::FilterApplied
                        };
                    }
                }
                _ => {}
            }
        }

        // Update filter textinput.
        let old_val = self.filter_input.value();
        cmds.push(self.filter_input.update(message));
        let new_val = self.filter_input.value();

        if new_val != old_val {
            self.filtered_indices = self.run_filter();
        }

        self.update_pagination();
        batch(cmds)
    }

    // --- View -----------------------------------------------------------------

    pub fn view(&self, styles: &Styles) -> String {
        let s = match self.view_state {
            StashViewState::ShowingError => {
                let err = self.err.as_deref().unwrap_or("unknown error");
                return format!("\n   {}\n\n   {}", styles.error_title_style("ERROR"), err);
            }
            StashViewState::LoadingDocument => {
                format!(" {} Loading document...", self.spinner.view())
            }
            StashViewState::Ready => self.ready_view(styles),
        };
        format!("\n{}", indent(&s, STASH_INDENT))
    }

    fn ready_view(&self, styles: &Styles) -> String {
        let load_indicator = if self.should_spin() {
            self.spinner.view()
        } else {
            " ".to_owned()
        };

        let logo_or_filter = self.logo_or_filter_view(styles);
        let header = self.header_view(styles);
        let populated = self.populated_view(styles);
        let populated_height = populated.lines().count() + 2;

        let pagination = self.pagination_view(styles);

        let avail_height = self.height.saturating_sub(
            STASH_VIEW_TOP_PADDING + populated_height + STASH_VIEW_BOTTOM_PADDING,
        );
        let blank_lines = "\n".repeat(avail_height);

        let help = self.help_view(styles);

        format!(
            "{}{}\n\n  {}\n\n{}\n\n{}  {}\n\n{}",
            load_indicator,
            logo_or_filter,
            header,
            populated,
            blank_lines,
            pagination,
            help
        )
    }

    fn logo_or_filter_view(&self, styles: &Styles) -> String {
        let mut s = " ".to_owned();
        if self.show_status_message && self.filter_state == FilterState::Filtering {
            if let Some(ref sm) = self.status_message {
                s += &sm.render(styles);
            }
        } else if self.filter_state == FilterState::Filtering {
            s += &self.filter_input.view();
        } else {
            s += &styles.logo_style(" Glow ");
            if self.show_status_message {
                if let Some(ref sm) = self.status_message {
                    s += "  ";
                    s += &sm.render(styles);
                }
            }
        }
        s
    }

    fn header_view(&self, styles: &Styles) -> String {
        if self.filter_state == FilterState::Filtering {
            let count = self.markdowns.len();
            if count == 0 {
                return styles.gray_fg("Nothing found.");
            }
            return styles.gray_fg(&format!("{} local", count));
        }

        let divider_bar = styles.dark_gray_fg(" │ ");
        let sections: Vec<String> = self.sections.iter().enumerate().map(|(i, sec)| {
            let s = match sec.key {
                SectionKey::Documents => format!("{} documents", self.markdowns.len()),
                SectionKey::Filter => format!(
                    "{} \"{}\"",
                    self.filtered_indices.len(),
                    self.filter_input.value()
                ),
            };
            if self.section_index == i && self.sections.len() > 1 {
                styles.selected_tab_style(&s)
            } else {
                styles.tab_style(&s)
            }
        }).collect();

        sections.join(&divider_bar)
    }

    fn populated_view(&self, styles: &Styles) -> String {
        let visible = self.visible_markdowns();
        let mut b = String::new();

        if visible.is_empty() {
            let f = |s: &str| format!("  {}", styles.gray_fg(s));
            let msg = match self.current_section_ref().key {
                SectionKey::Documents => {
                    if self.loading_done() { f("No files found.") }
                    else { f("Looking for local files...") }
                }
                SectionKey::Filter => return String::new(),
            };
            return msg;
        }

        let (start, end) = self.current_section_ref().paginator.get_slice_bounds(visible.len());
        let docs = &visible[start..end];

        for (i, md) in docs.iter().enumerate() {
            stash_item_view(&mut b, self, i, md, styles);
            if i != docs.len() - 1 {
                let _ = write!(b, "\n\n");
            }
        }

        // Pad with blank lines if the page isn't full.
        let items_on_page = self.current_section_ref().paginator.items_on_page(visible.len());
        if items_on_page < self.per_page() {
            let n = (self.per_page() - items_on_page) * STASH_VIEW_ITEM_HEIGHT;
            for _ in 0..n {
                b.push('\n');
            }
        }

        b
    }

    fn pagination_view(&self, _styles: &Styles) -> String {
        let p = &self.current_section_ref().paginator;
        if p.total_pages > 1 {
            p.view()
        } else {
            String::new()
        }
    }

    fn help_view(&self, styles: &Styles) -> String {
        if self.show_full_help {
            let s = "\
k/↑     up           /     filter\n\
j/↓     down         ?     close help\n\
enter   open         q     quit\n\
esc     clear filter\n";
            return styles.subtle_style(s);
        }
        styles.subtle_style("/ filter  • ? help  • q quit")
    }
}

// --- Helpers ------------------------------------------------------------------

fn indent(s: &str, n: usize) -> String {
    let prefix = " ".repeat(n);
    s.lines()
        .map(|line| format!("{}{}", prefix, line))
        .collect::<Vec<_>>()
        .join("\n")
}

fn load_local_markdown_cmd(path: String, note: String, modtime: std::time::SystemTime) -> Cmd {
    cmd(async move {
        match tokio::fs::read_to_string(&path).await {
            Ok(body) => {
                Some(msg(FetchedMarkdownMsg(Markdown {
                    local_path: path,
                    filter_value: String::new(),
                    body,
                    note,
                    modtime,
                })))
            }
            Err(e) => Some(msg(StashErrMsg(e.to_string()))),
        }
    })
}

fn open_editor_stash_cmd(path: String) -> Cmd {
    let editor = std::env::var("EDITOR")
        .or_else(|_| std::env::var("VISUAL"))
        .unwrap_or_else(|_| "vi".into());
    cmd(async move {
        let fields = shlex::split(&editor).unwrap_or_else(|| vec!["vi".into()]);
        if !fields.is_empty() {
            let _ = tokio::task::spawn_blocking(move || {
                std::process::Command::new(&fields[0])
                    .args(&fields[1..])
                    .arg(&path)
                    .status()
            })
            .await;
        }
        None
    })
}
