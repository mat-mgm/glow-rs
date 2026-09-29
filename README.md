# glow-rs

A native Rust port of [Glow](https://github.com/charmbracelet/glow), the terminal Markdown reader CLI and TUI.

## Features

- Multiple reading modes:
  - Direct CLI printing to stdout with styled ANSI output.
  - Paged mode using the system pager (`$PAGER` or `less -r`).
  - Full-screen interactive TUI with file navigation, fuzzy search, and live preview.
- Flexible input sources:
  - Local files and directories.
  - Standard input (pipes or `glow -`).
  - Web URLs (HTTP/HTTPS).
  - GitHub and GitLab repository READMEs (`github.com/owner/repo` or `gitlab.com/owner/repo`).
- Live file reloading: watch files for changes and automatically refresh the display.
- Stylesheet customization:
  - Built-in themes (dark, light, dracula, tokyonight, ascii, notty).
  - Custom JSON stylesheets via `GLAMOUR_STYLE` or `--style`.
- Configuration file support (`~/.config/glow/glow.yml`).
- Line numbering and word wrapping.

## Installation

### From Source

```bash
cargo build --release
```

The resulting binary will be located in `target/release/glow`.

## Usage

### Reading Files and Input

```bash
# Render a local file directly
glow README.md

# Read from standard input
curl -s https://raw.githubusercontent.com/.../README.md | glow -

# View with system pager
glow -p README.md

# View a GitHub repository README
glow github.com/charmbracelet/glow
```

### Interactive TUI Mode

Launch the interactive file browser and Markdown viewer:

```bash
# Launch TUI in current directory
glow

# Or explicitly launch TUI
glow --tui
```

In TUI mode:
- Use arrow keys or `j`/`k` to navigate.
- Press `/` to fuzzy search.
- Press `Enter` to open and view a file.
- Press `Esc` or `q` to go back or quit.

### Command Line Options

```text
Usage: glow [OPTIONS] [SOURCE] [COMMAND]

Arguments:
  [SOURCE]  Source: file, directory, URL, stdin (-), or github://user/repo

Options:
  -p, --pager[=<PAGER>]  Display with pager
  -t, --tui[=<TUI>]      Display with TUI
  -s, --style <STYLE>    Style name (dark, light, dracula, tokyonight, ascii, notty) or JSON path
  -w, --width <WIDTH>    Word-wrap at column width (0 for terminal width)
  -a, --all              Show hidden files in TUI
  -l, --line-numbers     Display line numbers
      --config <CONFIG>  Config file path
  -h, --help             Print help
  -V, --version          Print version

Commands:
  config  Open config file in $EDITOR
```

## Configuration

Glow can be configured via a YAML file placed at `~/.config/glow/glow.yml`:

```yaml
style: "dark"
mouse: true
pager: false
width: 80
showAllFiles: false
```

You can open the configuration file directly in your default editor:

```bash
glow config
```

## License

This project is licensed under the MIT License.
