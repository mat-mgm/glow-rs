use std::fmt::Write as FmtWrite;

use fuzzy_matcher::{skim::SkimMatcherV2, FuzzyMatcher};
use lipgloss_rs::Style;

use super::markdown::Markdown;
use super::stash::StashModel;
use super::styles::Styles;

const VERTICAL_LINE: &str = "│";
pub fn stash_item_view(
    b: &mut String,
    m: &StashModel,
    index: usize,
    md: &Markdown,
    styles: &Styles,
) {
    let truncate_to = m.width.saturating_sub(super::stash::STASH_VIEW_HORIZONTAL_PADDING * 2);
    let title = truncate_string(&md.note, truncate_to);
    let date = md.relative_time();

    let is_selected = index == m.cursor();
    let is_filtering = m.filter_state == super::stash::FilterState::Filtering;
    let single_filtered = is_filtering && m.visible_markdowns_count() == 1;

    let (gutter, title_styled, date_styled) = if (is_selected && !is_filtering) || single_filtered {
        // Selected item.
        let gutter = styles.dull_fuchsia_fg(VERTICAL_LINE);
        let title_s = if m.current_section_key() == super::stash::SectionKey::Filter
            || single_filtered
        {
            style_filtered_text(
                &title,
                &m.filter_input.value(),
                styles,
            )
        } else {
            styles.fuchsia_fg(&title)
        };
        let date_s = styles.dim_fuchsia_fg(&date);
        (gutter, title_s, date_s)
    } else {
        // Unselected item.
        let gutter = " ".to_owned();
        let title_s = if is_filtering && m.filter_input.value().is_empty() {
            styles.dim_normal_fg(&title)
        } else {
            let s = lipgloss_rs::Style::new()
                .foreground(lipgloss_rs::light_dark(
                    styles.is_dark,
                    lipgloss_rs::Color::parse("#1a1a1a"),
                    lipgloss_rs::Color::parse("#dddddd"),
                ));
            style_filtered_text_with_style(&title, &m.filter_input.value(), s, styles)
        };
        let date_s = styles.gray_fg(&date);
        (gutter, title_s, date_s)
    };

    let _ = write!(b, "{} {}\n", gutter, title_styled);
    let _ = write!(b, "{} {}", gutter, date_styled);
}

fn truncate_string(s: &str, max_width: usize) -> String {
    let chars: Vec<char> = s.chars().collect();
    let width: usize = chars.iter().map(|c| char_width(*c)).sum();
    if width <= max_width {
        return s.to_owned();
    }
    let ellipsis_width = 1; // '…' is 1 cell wide
    let mut remaining = max_width.saturating_sub(ellipsis_width);
    let mut result = String::new();
    for c in &chars {
        let w = char_width(*c);
        if remaining < w { break; }
        result.push(*c);
        remaining -= w;
    }
    result.push('…');
    result
}

fn char_width(c: char) -> usize {
    unicode_width::UnicodeWidthChar::width(c).unwrap_or(1)
}

/// Style text with fuzzy match highlights.
pub fn style_filtered_text(haystack: &str, needles: &str, styles: &Styles) -> String {
    let default_style = lipgloss_rs::Style::new()
        .foreground(styles.fuchsia);
    style_filtered_text_with_style(haystack, needles, default_style, styles)
}

fn style_filtered_text_with_style(
    haystack: &str,
    needles: &str,
    default_style: Style,
    _styles: &Styles,
) -> String {
    if needles.is_empty() {
        return default_style.render(&[haystack]);
    }

    let matcher = SkimMatcherV2::default();
    let Some((_, indices)) = matcher.fuzzy_indices(haystack, needles) else {
        return default_style.render(&[haystack]);
    };

    let underline_style = default_style.clone().underline(true);
    let mut result = String::new();
    for (i, c) in haystack.chars().enumerate() {
        if indices.contains(&i) {
            result.push_str(&underline_style.render(&[&c.to_string()]));
        } else {
            result.push_str(&default_style.render(&[&c.to_string()]));
        }
    }
    result
}

// --- Extension on StashModel for use by stashitem ----------------------------

impl StashModel {
    pub fn visible_markdowns_count(&self) -> usize {
        self.visible_markdowns().len()
    }

    pub fn current_section_key(&self) -> super::stash::SectionKey {
        self.current_section_ref().key
    }
}
