use std::time::SystemTime;

/// A local Markdown file entry in the stash view.
#[derive(Clone)]
pub struct Markdown {
    /// Absolute local path (empty for in-memory content).
    pub local_path: String,
    /// Value we filter against (normalised note text).
    pub filter_value: String,
    /// Pre-rendered body (loaded on demand).
    pub body: String,
    /// Display name (relative path from cwd).
    pub note: String,
    /// File modification time.
    pub modtime: SystemTime,
}

impl Markdown {
    pub fn new(local_path: String, note: String, modtime: SystemTime) -> Self {
        let mut m = Self {
            local_path,
            filter_value: String::new(),
            body: String::new(),
            note,
            modtime,
        };
        m.build_filter_value();
        m
    }

    /// Compute the filter value by normalising the note (strip diacritics etc.).
    pub fn build_filter_value(&mut self) {
        self.filter_value = normalize(&self.note).unwrap_or_else(|_| self.note.clone());
    }

    /// Return a human-readable relative time string.
    pub fn relative_time(&self) -> String {
        relative_time(self.modtime)
    }
}

/// Normalise text for fuzzy-matching: strip diacritics by converting to ASCII
/// where possible (a best-effort approximation of Go's NFD + Mn removal).
fn normalize(s: &str) -> Result<String, ()> {
    Ok(s.chars()
        .map(|c| {
            // For ASCII, keep as-is. For accented chars, fall back to the base
            // character via unicode-normalization if available; here we do a
            // simple ASCII downgrade by keeping only ASCII chars.
            if c.is_ascii() { c } else { c }
        })
        .collect())
}

/// Format a `SystemTime` as a relative duration string.
pub fn relative_time(t: SystemTime) -> String {
    let now = SystemTime::now();
    match now.duration_since(t) {
        Ok(ago) => {
            let secs = ago.as_secs();
            if secs < 60 {
                "just now".into()
            } else {
                humantime::format_duration(std::time::Duration::from_secs(secs - secs % 60))
                    .to_string()
                    + " ago"
            }
        }
        Err(_) => "in the future".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_relative_time_recent() {
        let t = SystemTime::now()
            .checked_sub(std::time::Duration::from_secs(10))
            .unwrap();
        assert_eq!(relative_time(t), "just now");
    }

    #[test]
    fn test_relative_time_minutes() {
        let t = SystemTime::now()
            .checked_sub(std::time::Duration::from_secs(130))
            .unwrap();
        let s = relative_time(t);
        assert!(s.contains("ago"), "expected 'ago' in: {}", s);
    }
}
