# Glow → Rust Port

## Overview
A port of [Glow](https://github.com/charmbracelet/glow) (`github.com/charmbracelet/glow/v2`,
Charm's terminal Markdown reader CLI) to Rust, built on top of the sibling
`glamour-rs`, `bubbletea-rs`, `lipgloss-rs`, and `bubbles-rs` crates.

* **Purpose**: Render Markdown in the terminal — either directly to stdout, via
  a pager, or inside a full-screen TUI file browser — from local files, stdin,
  or HTTP/GitHub/GitLab URLs.
* **Context**: Go source of truth lives at `../charm/glow/` (~3.5k LOC across
  the root package and a `ui/` sub-package). Dependencies:
  * `glamour` / `lipgloss` / `bubbletea` / `bubbles` → covered by sibling crates.
  * `spf13/cobra` + `spf13/viper` → `clap` (CLI args) + `serde`/`toml` (config).
  * `mvdan.cc/sh/v3/shell` → `shlex` crate (PAGER command splitting).
  * `muesli/gitcha` (git-aware file finder) → `ignore` crate (respects `.gitignore`).
  * `sahilm/fuzzy` → `fuzzy-matcher` crate.
  * `go-humanize` → `humantime` crate.
  * `fsnotify` → `notify` crate (live file reloading in the pager).
  * `atotto/clipboard` → `arboard` crate.
  * `muesli/go-app-paths` → `dirs` crate (XDG/platform config paths).
  * `net/http` → `ureq` crate (synchronous HTTP; no async needed for URL fetch).
  * `golang.org/x/term` → `crossterm` (already inside `bubbletea-rs`).
  * `caarlos0/env` → read `GLAMOUR_STYLE` and debug flags from env directly.
* **Scope**:
  * **In**: CLI mode (stdin/file/URL → rendered Markdown on stdout), pager mode
    (`PAGER` env var), TUI mode (full-screen file browser + Markdown pager),
    GitHub and GitLab README auto-fetch, YAML config file
    (`~/.config/glow/glow.yml`), `glow config` subcommand (open config in
    `$EDITOR`), fuzzy filtering, file watching (live reload), line numbers.
  * **Out (for now)**: man page generation (`glow man`), shell completion
    registration, Windows console setup.
* **References**:
  * Go source: `../charm/glow/`
  * Sibling crates: `../glamour-rs`, `../bubbletea-rs`, `../lipgloss-rs`,
    `../bubbles-rs`
  * `SOURCES.md` — stack porting status

## Status
Current status: in-progress
Start date: 2026-06-29
Last updated: 2026-06-29
Priority: normal

## Goals
* **Functional parity** with Glow v2: all three modes (CLI/pager/TUI) work;
  sources (file/stdin/URL/GitHub/GitLab) resolve correctly; built-in Glamour
  styles and custom JSON styles apply.
* **Idiomatic Rust CLI**: `clap` argument parsing, `serde`/`toml` config, no
  global mutable state.
* **TUI parity**: stash (file browser) and pager sub-models, fuzzy filtering,
  pagination, status bar, help overlay, clipboard copy, live file watching.

Success criteria:
* `glow README.md` renders to styled ANSI output.
* `glow -` (stdin) works.
* `glow --pager README.md` pipes through `$PAGER`.
* `glow --tui` (or no args) opens the TUI file browser.
* `cargo build`, `cargo test`, and `cargo clippy` are warning-free.

Constraints / priorities:
* Build on sibling crates — do not modify them.
* Prefer `ureq` over `reqwest` for synchronous HTTP (avoids async complexity
  in the CLI path).
* The TUI is the most complex part; implement CLI first, then TUI incrementally.

## Architecture
* **Structure**: binary crate `glow-rs` with a `ui` module.
  ```
  src/
    main.rs         — clap CLI, config loading, execute/runTUI dispatch
    config.rs       — Config struct, YAML/TOML serde, default config
    source.rs       — source resolution (file/stdin/URL/dir/GitHub/GitLab)
    url.rs          — GitHub/GitLab README URL helpers
    utils.rs        — frontmatter strip, IsMarkdownFile, WrapCodeBlock, GlamourStyle
    ui/
      mod.rs        — NewProgram, Model, top-level update/view
      config.rs     — TuiConfig struct
      styles.rs     — color definitions, style functions
      markdown.rs   — Markdown struct, normalize, relativeTime
      sort.rs       — sortMarkdowns
      pager.rs      — PagerModel (viewport-based reader, status bar, help)
      stash.rs      — StashModel (file browser, paginator, spinner, textinput)
      stashitem.rs  — stashItemView, styleFilteredText
      stashhelp.rs  — help overlay for stash view
      keys.rs       — key constants
  ```
* **Key design decisions**:
  * CLI path is synchronous; TUI path uses `bubbletea-rs` async runtime.
  * Config is loaded once at startup from YAML (mirroring Viper's behaviour);
    CLI flags override config values.
  * `Source` enum (Stdin, File, Http, Dir) unifies source resolution.
  * Glamour style is resolved via `glamour_rs::Style`; `GlamourStyle` helper
    (mirroring `utils/utils.go`) selects code-block style when the source is
    not a Markdown file.
  * File watching in the pager uses the `notify` crate; watcher events are
    forwarded to the bubbletea runtime as `Msg::Custom`.
* **Resources**:
  * `glamour-rs` (path), `bubbletea-rs` (path), `lipgloss-rs` (path),
    `bubbles-rs` (path)
  * `clap` — CLI argument parsing
  * `serde` + `serde_yaml` — config file
  * `ureq` — HTTP fetching
  * `shlex` — PAGER command splitting
  * `ignore` — gitignore-aware directory walking
  * `fuzzy-matcher` — fuzzy search in stash view
  * `humantime` — relative time formatting
  * `notify` — file system watching
  * `arboard` — clipboard
  * `dirs` — XDG config dir

## Development Guidelines
* Environment: Use Nix flakes (`nix develop`) to setup the dependencies. Enter the nix shell once and perform all development inside it.
* Version Control: Use `git` to track changes. Commit every time a new phase or feature is implemented and verified to work as expected.
* Workflow: After each phase implementation is done, wait for explicit user confirmation before marking the verification boxes in the roadmap and committing.
* Style: Use suckless coding style and robust coding practices.
* Warnings: Always address and fix compiler warnings.
* Commit convention: `Phase N: …`, authored as `dev <dev@localhost>`, no AI attribution.

## Roadmap

### General conditions
Go source of truth: `../charm/glow/`. Sibling crates are frozen — do not
modify them. One commit per phase.

Status legend: `[ ]` todo · `[~]` in-progress · `[✓]` done · `[x]` blocked ·
`[?]` optional · `[!]` critical.

### Phase 0: Scaffolding [ ]
**Description**: Create the `glow-rs` binary crate, wire dependencies, set up Nix dev shell.

**Tasks**
- [ ] `Cargo.toml` — binary crate with path deps on all four sibling crates;
      add `clap`, `serde`, `serde_yaml`, `ureq`, `shlex`, `ignore`,
      `fuzzy-matcher`, `humantime`, `notify`, `arboard`, `dirs`, `toml`.
- [ ] `src/main.rs` skeleton with empty `main()`.
- [ ] Module stubs: `config.rs`, `source.rs`, `url.rs`, `utils.rs`,
      `ui/mod.rs`, and sub-modules.
- [ ] `flake.nix` dev shell (adapt from `glamour-rs/flake.nix`).
- [ ] `.gitignore`.

**Checks**
- [ ] `cargo build` green on skeleton.
- [ ] `cargo clippy` warning-free.

### Phase 1: Source resolution and CLI render mode [ ]
**Description**: Port the core CLI path: resolve a source (stdin/file/URL/dir),
strip frontmatter, render with `glamour-rs`, and print to stdout.

**Tasks**
- [ ] `source.rs` — `Source` struct (reader + URL string); `source_from_arg()`;
      directory walk finding README files.
- [ ] `utils.rs` — `remove_frontmatter()`, `is_markdown_file()`,
      `wrap_code_block()`, `glamour_style()`.
- [ ] `config.rs` — `AppConfig` struct (style, width, pager, tui, mouse, all,
      showLineNumbers, preserveNewLines); `load_config()` reading from default
      XDG paths via `dirs`; `default_config_content()`.
- [ ] `main.rs` — `clap` CLI definition mirroring Go flags; `validate_options()`;
      `execute_cli()` (read source → strip frontmatter → render → print).
- [ ] Pager mode: detect `pager` flag; split `$PAGER` with `shlex`; pipe output.

**Checks**
- [ ] `glow README.md` prints styled Markdown to stdout.
- [ ] `echo "# Hi" | glow -` works.
- [ ] `glow --pager README.md` pipes through `less`.
- [ ] `cargo clippy` warning-free.

**Dependencies**: Phase 0.

### Phase 2: HTTP sources (URL/GitHub/GitLab) [ ]
**Description**: Port the HTTP and GitHub/GitLab README auto-fetch paths.

**Tasks**
- [ ] `url.rs` — `readme_url()`, `github_readme_url()`, `gitlab_readme_url()`;
      `find_github_readme()`, `find_gitlab_readme()` using `ureq`.
- [ ] Wire URL resolution into `source_from_arg()`.
- [ ] `glow github://user/repo` and `glow gitlab://user/repo` resolve to raw
      README content.
- [ ] `glow https://...` fetches and renders any HTTP Markdown URL.

**Checks**
- [ ] `glow github://charmbracelet/glamour` (or mocked) resolves correctly.
- [ ] HTTP 404 returns a clear error.

**Dependencies**: Phase 1.

### Phase 3: TUI types, styles, and foundation [ ]
**Description**: Port the types and styles that both sub-models share.

**Tasks**
- [ ] `ui/config.rs` — `TuiConfig` struct (ShowAllFiles, ShowLineNumbers,
      GlamourMaxWidth, GlamourStyle, EnableMouse, PreserveNewLines, Path,
      HighPerformancePager, GlamourEnabled).
- [ ] `ui/styles.rs` — adaptive color constants and style-fn closures
      mirroring `styles.go`.
- [ ] `ui/markdown.rs` — `Markdown` struct; `build_filter_value()`;
      `relative_time()` using `humantime`.
- [ ] `ui/sort.rs` — `sort_markdowns()`.
- [ ] `ui/keys.rs` — key string constants.
- [ ] `ui/mod.rs` — `AppModel` top-level struct (`state`, `stash`, `pager`,
      `common`); `new_program()`.

**Checks**
- [ ] Types compile; `cargo test` green.

**Dependencies**: Phase 0.

### Phase 4: Pager sub-model [ ]
**Description**: Port the viewport-based Markdown pager (document reader).

**Tasks**
- [ ] `ui/pager.rs` — `PagerModel` with `bubbles_rs::viewport::Model`;
      `pager_state` (browse/statusMessage); `update()` handling scroll keys,
      `g`/`G` (top/bottom), `c` (clipboard copy), `e` (open editor), `?` (help).
- [ ] Status bar: scroll position, filename, help hint, status message with timeout.
- [ ] Line number rendering (optional, controlled by `showLineNumbers`).
- [ ] File watching: `notify` watcher sends reload events; on `reloadMsg`,
      re-render content and update viewport.
- [ ] Help overlay: key-binding reference panel.
- [ ] Glamour render on `contentRenderedMsg`.

**Checks**
- [ ] Pager renders a document; scroll keys work.
- [ ] Status bar shows correct filename and scroll position.
- [ ] Clipboard copy sends content to arboard.

**Dependencies**: Phase 3.

### Phase 5: Stash sub-model (file browser) [ ]
**Description**: Port the file listing view with fuzzy filtering and pagination.

**Tasks**
- [ ] `ui/stash.rs` — `StashModel` with `bubbles_rs::spinner::Model`,
      `paginator::Model`, `textinput::Model`; sections (documents, filter);
      cursor and page state; `stash_state` (ready/loadingDocument/showingError).
- [ ] Background file search: walk directory using `ignore` crate (respects
      `.gitignore`); send `FoundLocalFileMsg` events to the runtime.
- [ ] Fuzzy filter: `fuzzy-matcher` crate; filter/apply/clear states.
- [ ] Pagination: `bubbles_rs::paginator` computes page bounds.
- [ ] `ui/stashitem.rs` — `stash_item_view()`, `style_filtered_text()`.
- [ ] `ui/stashhelp.rs` — help overlay for stash view.

**Checks**
- [ ] TUI opens with local Markdown files listed.
- [ ] Arrow keys navigate; Enter opens a file in the pager.
- [ ] `/` activates filter; typing narrows results; Esc clears.
- [ ] Pagination works when there are more files than fit on screen.

**Dependencies**: Phase 3.

### Phase 6: TUI orchestration and full integration [ ]
**Description**: Connect stash and pager into the top-level model; wire TUI run
path from main; end-to-end validation.

**Tasks**
- [ ] `ui/mod.rs` — `AppModel::update()` dispatches to stash/pager based on
      `state`; handles `initLocalFileSearchMsg`, `foundLocalFileMsg`,
      `localFileSearchFinished`, `statusMessageTimeoutMsg`.
- [ ] `ui/mod.rs` — `AppModel::view()` delegates to stash or pager view.
- [ ] `main.rs` `run_tui()` — builds `TuiConfig` from CLI flags + env;
      calls `ui::new_program()`.
- [ ] Dark/light theme detection from `termenv`/`crossterm` for auto style.
- [ ] `glow config` subcommand: write default config if missing; open `$EDITOR`.

**Checks**
- [ ] `glow` (no args) opens TUI; selecting a file shows it in the pager; `Esc`
      returns to file list; `q` quits.
- [ ] `glow --tui path/to/dir` opens TUI scoped to that directory.
- [ ] `glow --tui file.md` opens file directly in pager.
- [ ] `glow config` opens the config file in `$EDITOR`.

**Dependencies**: Phases 4, 5.

### Phase 7: Polish, tests, and final validation [ ]
**Description**: Clippy sweep, unit tests, examples, documentation.

**Tasks**
- [ ] Unit tests: `remove_frontmatter`, `is_markdown_file`, `wrap_code_block`,
      `source_from_arg` (local paths), `relative_time`.
- [ ] `cargo build`, `cargo test`, `cargo clippy` all green, no warnings.
- [ ] README with usage examples.

**Checks**
- [ ] All tests pass.
- [ ] No clippy warnings.
- [ ] End-to-end CLI and TUI smoke test.

**Dependencies**: Phases 1–6.
