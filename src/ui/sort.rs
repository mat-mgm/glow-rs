use super::markdown::Markdown;

/// Sort markdowns alphabetically by note (display name).
pub fn sort_markdowns(mds: &mut Vec<Markdown>) {
    mds.sort_by(|a, b| a.note.cmp(&b.note));
}
