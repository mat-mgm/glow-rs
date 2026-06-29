use regex::bytes::Regex;
use std::sync::OnceLock;

static YAML_PATTERN: OnceLock<Regex> = OnceLock::new();

fn yaml_pattern() -> &'static Regex {
    YAML_PATTERN.get_or_init(|| Regex::new(r"(?m)^---\r?\n(\s*\r?\n)?").unwrap())
}

/// Strip YAML front-matter from Markdown content.
pub fn remove_frontmatter(content: &[u8]) -> &[u8] {
    let pat = yaml_pattern();
    let matches: Vec<_> = pat.find_iter(content).collect();
    if matches.len() >= 2 && matches[0].start() == 0 {
        &content[matches[1].end()..]
    } else {
        content
    }
}

static MARKDOWN_EXTENSIONS: &[&str] = &[
    ".md", ".mdown", ".mkdn", ".mkd", ".markdown",
];

/// Returns true if the filename looks like a Markdown file.
/// Files with no extension are treated as Markdown by default.
pub fn is_markdown_file(filename: &str) -> bool {
    let ext = std::path::Path::new(filename)
        .extension()
        .and_then(|e| e.to_str());
    match ext {
        None => true,
        Some(e) => {
            let dot = format!(".{}", e);
            MARKDOWN_EXTENSIONS
                .iter()
                .any(|&m| m.eq_ignore_ascii_case(&dot))
        }
    }
}

/// Wrap source code in a fenced code block with the given language hint.
pub fn wrap_code_block(s: &str, language: &str) -> String {
    format!("```{}\n{}```", language, s)
}

/// Detect whether the terminal has a dark background using the COLORFGBG env
/// variable (set by many terminals: fg;bg where bg < 8 means light).
/// Falls back to dark if undetected.
pub fn has_dark_background() -> bool {
    if let Ok(val) = std::env::var("COLORFGBG") {
        if let Some(bg) = val.split(';').last() {
            if let Ok(n) = bg.trim().parse::<u8>() {
                return n >= 8;
            }
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_remove_frontmatter_present() {
        let input = b"---\ntitle: Test\n---\n# Hello\n";
        let out = remove_frontmatter(input);
        assert_eq!(out, b"# Hello\n");
    }

    #[test]
    fn test_remove_frontmatter_absent() {
        let input = b"# Hello\nNo frontmatter here.\n";
        let out = remove_frontmatter(input);
        assert_eq!(out, input);
    }

    #[test]
    fn test_is_markdown_file() {
        assert!(is_markdown_file("README.md"));
        assert!(is_markdown_file("README"));
        assert!(is_markdown_file("doc.markdown"));
        assert!(!is_markdown_file("main.go"));
        assert!(!is_markdown_file("script.py"));
    }

    #[test]
    fn test_wrap_code_block() {
        let out = wrap_code_block("fn main() {}\n", ".rs");
        assert!(out.starts_with("```.rs\n"));
        assert!(out.ends_with("```"));
    }

    #[test]
    fn test_remove_frontmatter_windows_line_endings() {
        let input = b"---\r\ntitle: Test\r\n---\r\n# Hello\r\n";
        let out = remove_frontmatter(input);
        assert_eq!(out, b"# Hello\r\n");
    }

    #[test]
    fn test_is_markdown_file_extensions() {
        assert!(is_markdown_file("doc.mdown"));
        assert!(is_markdown_file("doc.mkdn"));
        assert!(is_markdown_file("doc.mkd"));
        assert!(!is_markdown_file("image.png"));
        assert!(!is_markdown_file("data.json"));
    }
}
