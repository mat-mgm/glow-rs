use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};

pub type BoxError = Box<dyn std::error::Error + Send + Sync>;

/// A resolved Markdown source.
pub struct Source {
    pub content: Vec<u8>,
    /// Absolute path or URL of the source (used for glamour base URL and
    /// determining whether the content is Markdown or code).
    pub url: String,
}

static README_NAMES: &[&str] = &[
    "README.md", "README", "Readme.md", "Readme", "readme.md", "readme",
];

/// Parse a CLI argument and return a Source with its content loaded.
pub fn source_from_arg(arg: &str) -> Result<Source, BoxError> {
    // stdin
    if arg == "-" {
        let mut buf = Vec::new();
        io::stdin().read_to_end(&mut buf)?;
        return Ok(Source { content: buf, url: String::new() });
    }

    // GitHub / GitLab pseudo-protocol — handled by caller (url.rs)
    // HTTP(S) URLs — handled by caller (url.rs)
    // Let the caller wrap HTTP resolution; here we handle local paths.

    // empty arg → current directory
    let path = if arg.is_empty() { "." } else { arg };

    let meta = fs::metadata(path);

    if let Ok(m) = meta {
        if m.is_dir() {
            return source_from_dir(path);
        }
    }

    // regular file
    let abs = PathBuf::from(path).canonicalize()?;
    let content = fs::read(&abs)?;
    Ok(Source { content, url: abs.to_string_lossy().into_owned() })
}

/// Walk a directory and return the first README-style file found.
fn source_from_dir(dir: &str) -> Result<Source, BoxError> {
    for entry in walkdir::WalkDir::new(dir).min_depth(1).max_depth(1) {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy();
        for &readme in README_NAMES {
            if name.eq_ignore_ascii_case(readme) {
                let abs = entry.path().canonicalize()?;
                let content = fs::read(&abs)?;
                return Ok(Source {
                    content,
                    url: abs.to_string_lossy().into_owned(),
                });
            }
        }
    }
    Err("missing markdown source".into())
}

/// Detect whether stdin is a pipe or redirected file.
///
/// Mirrors Go's `stdinIsPipe()`: returns true when stdin is NOT a character
/// device (i.e. an actual pipe or regular file) OR has data waiting (size > 0).
/// Terminals and /dev/null are char devices with size 0 → returns false.
pub fn stdin_is_pipe() -> bool {
    unsafe {
        let mut st: libc::stat = std::mem::zeroed();
        if libc::fstat(libc::STDIN_FILENO, &mut st) == 0 {
            let is_char_device = (st.st_mode & libc::S_IFMT as u32) == libc::S_IFCHR as u32;
            return !is_char_device || st.st_size > 0;
        }
    }
    // Fallback.
    use std::io::IsTerminal;
    !io::stdin().is_terminal()
}

/// Fetch content from an HTTP/HTTPS URL.
pub fn fetch_url(url: &str) -> Result<Source, BoxError> {
    let response = ureq::get(url).call()?;
    if response.status() != 200 {
        return Err(format!("HTTP status {}", response.status()).into());
    }
    let mut buf = Vec::new();
    response.into_reader().read_to_end(&mut buf)?;
    Ok(Source { content: buf, url: url.to_owned() })
}

/// Return true if path starts with http:// or https://.
pub fn is_http_url(path: &str) -> bool {
    path.starts_with("http://") || path.starts_with("https://")
}

/// Return true if the string looks like any scheme URL.
pub fn is_url(s: &str) -> bool {
    s.contains("://")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_source_from_arg_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.md");
        fs::write(&path, b"# Hello\n").unwrap();
        let src = source_from_arg(path.to_str().unwrap()).unwrap();
        assert_eq!(src.content, b"# Hello\n");
    }

    #[test]
    fn test_source_from_arg_dir_readme() {
        let dir = tempfile::tempdir().unwrap();
        let readme = dir.path().join("README.md");
        fs::write(&readme, b"# Readme\n").unwrap();
        let src = source_from_arg(dir.path().to_str().unwrap()).unwrap();
        assert_eq!(src.content, b"# Readme\n");
    }

    #[test]
    fn test_is_http_url() {
        assert!(is_http_url("https://example.com/file.md"));
        assert!(is_http_url("http://example.com/file.md"));
        assert!(!is_http_url("file.md"));
        assert!(!is_http_url("github://user/repo"));
    }

    #[test]
    fn test_source_from_arg_missing_file() {
        let result = source_from_arg("/nonexistent/path/to/file.md");
        assert!(result.is_err());
    }

    #[test]
    fn test_source_from_arg_dir_no_readme() {
        let dir = tempfile::tempdir().unwrap();
        let result = source_from_arg(dir.path().to_str().unwrap());
        assert!(result.is_err());
    }
}

/// Compute a base URL from an absolute source URL for glamour image resolution.
pub fn base_url_from_source(url: &str) -> Option<String> {
    if !is_url(url) { return None; }
    let parsed = ::url::Url::parse(url).ok()?;
    let mut base = parsed.clone();
    base.set_path(
        Path::new(parsed.path())
            .parent()
            .and_then(|p| p.to_str())
            .unwrap_or("/"),
    );
    Some(format!("{}/", base))
}
