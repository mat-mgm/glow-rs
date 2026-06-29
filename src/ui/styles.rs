use lipgloss_rs::{light_dark, Color, Style};

/// Resolved styles for a given dark/light context.
/// Mirrors the `var (...)` block in ui/styles.go.
pub struct Styles {
    pub is_dark: bool,
    // Palette
    pub normal_dim:       Color,
    pub gray:             Color,
    pub mid_gray:         Color,
    pub dark_gray:        Color,
    pub bright_gray:      Color,
    pub dim_bright_gray:  Color,
    pub cream:            Color,
    pub yellow_green:     Color,
    pub fuchsia:          Color,
    pub dim_fuchsia:      Color,
    pub dull_fuchsia:     Color,
    pub dim_dull_fuchsia: Color,
    pub green:            Color,
    pub red:              Color,
    pub semi_dim_green:   Color,
    pub dim_green:        Color,
    // Pager-specific palette
    pub mint_green:         Color,
    pub dark_green_pager:   Color,
    pub line_number_fg:     Color,
    pub status_bar_note_fg: Color,
    pub status_bar_bg:      Color,
}

impl Styles {
    pub fn new(is_dark: bool) -> Self {
        Self {
            is_dark,
            normal_dim:       light_dark(is_dark, Color::parse("#A49FA5"), Color::parse("#777777")),
            gray:             light_dark(is_dark, Color::parse("#909090"), Color::parse("#626262")),
            mid_gray:         light_dark(is_dark, Color::parse("#B2B2B2"), Color::parse("#4A4A4A")),
            dark_gray:        light_dark(is_dark, Color::parse("#DDDADA"), Color::parse("#3C3C3C")),
            bright_gray:      light_dark(is_dark, Color::parse("#847A85"), Color::parse("#979797")),
            dim_bright_gray:  light_dark(is_dark, Color::parse("#C2B8C2"), Color::parse("#4D4D4D")),
            cream:            Color::parse("#FFFDF5"),
            yellow_green:     light_dark(is_dark, Color::parse("#04B575"), Color::parse("#ECFD65")),
            fuchsia:          Color::parse("#EE6FF8"),
            dim_fuchsia:      light_dark(is_dark, Color::parse("#F1A8FF"), Color::parse("#99519E")),
            dull_fuchsia:     light_dark(is_dark, Color::parse("#F793FF"), Color::parse("#AD58B4")),
            dim_dull_fuchsia: light_dark(is_dark, Color::parse("#F6C9FF"), Color::parse("#7B4380")),
            green:            Color::parse("#04B575"),
            red:              light_dark(is_dark, Color::parse("#FF4672"), Color::parse("#ED567A")),
            semi_dim_green:   light_dark(is_dark, Color::parse("#35D79C"), Color::parse("#036B46")),
            dim_green:        light_dark(is_dark, Color::parse("#72D2B0"), Color::parse("#0B5137")),
            mint_green:       Color::parse("#89F0CB"),
            dark_green_pager: Color::parse("#1C8760"),
            line_number_fg:   light_dark(is_dark, Color::parse("#656565"), Color::parse("#7D7D7D")),
            status_bar_note_fg: light_dark(is_dark, Color::parse("#656565"), Color::parse("#7D7D7D")),
            status_bar_bg:    light_dark(is_dark, Color::parse("#E6E6E6"), Color::parse("#242424")),
        }
    }

    // --- Render helpers -------------------------------------------------------

    fn r(style: Style, s: &str) -> String { style.render(&[s]) }

    pub fn dim_normal_fg(&self, s: &str) -> String {
        Self::r(Style::new().foreground(self.normal_dim), s)
    }
    pub fn bright_gray_fg(&self, s: &str) -> String {
        Self::r(Style::new().foreground(self.bright_gray), s)
    }
    pub fn dim_bright_gray_fg(&self, s: &str) -> String {
        Self::r(Style::new().foreground(self.dim_bright_gray), s)
    }
    pub fn gray_fg(&self, s: &str) -> String {
        Self::r(Style::new().foreground(self.gray), s)
    }
    pub fn mid_gray_fg(&self, s: &str) -> String {
        Self::r(Style::new().foreground(self.mid_gray), s)
    }
    pub fn dark_gray_fg(&self, s: &str) -> String {
        Self::r(Style::new().foreground(self.dark_gray), s)
    }
    pub fn green_fg(&self, s: &str) -> String {
        Self::r(Style::new().foreground(self.green), s)
    }
    pub fn semi_dim_green_fg(&self, s: &str) -> String {
        Self::r(Style::new().foreground(self.semi_dim_green), s)
    }
    pub fn dim_green_fg(&self, s: &str) -> String {
        Self::r(Style::new().foreground(self.dim_green), s)
    }
    pub fn fuchsia_fg(&self, s: &str) -> String {
        Self::r(Style::new().foreground(self.fuchsia), s)
    }
    pub fn dim_fuchsia_fg(&self, s: &str) -> String {
        Self::r(Style::new().foreground(self.dim_fuchsia), s)
    }
    pub fn dull_fuchsia_fg(&self, s: &str) -> String {
        Self::r(Style::new().foreground(self.dull_fuchsia), s)
    }
    pub fn dim_dull_fuchsia_fg(&self, s: &str) -> String {
        Self::r(Style::new().foreground(self.dim_dull_fuchsia), s)
    }
    pub fn red_fg(&self, s: &str) -> String {
        Self::r(Style::new().foreground(self.red), s)
    }
    pub fn yellow_green_fg(&self, s: &str) -> String {
        Self::r(Style::new().foreground(self.yellow_green), s)
    }

    pub fn logo_style(&self, s: &str) -> String {
        Self::r(
            Style::new()
                .foreground(Color::parse("#ECFD65"))
                .background(self.fuchsia)
                .bold(true),
            s,
        )
    }

    pub fn error_title_style(&self, s: &str) -> String {
        Self::r(
            Style::new()
                .foreground(self.cream)
                .background(self.red)
                .padding(&[0, 1]),
            s,
        )
    }

    pub fn subtle_style(&self, s: &str) -> String {
        Self::r(
            Style::new().foreground(light_dark(
                self.is_dark, Color::parse("#9B9B9B"), Color::parse("#5C5C5C"),
            )),
            s,
        )
    }

    pub fn tab_style(&self, s: &str) -> String {
        Self::r(Style::new().foreground(self.gray), s)
    }

    pub fn selected_tab_style(&self, s: &str) -> String {
        Self::r(
            Style::new().foreground(light_dark(
                self.is_dark, Color::parse("#333333"), Color::parse("#979797"),
            )),
            s,
        )
    }

    pub fn status_bar_scroll_pos_style(&self, s: &str) -> String {
        Self::r(
            Style::new()
                .foreground(light_dark(
                    self.is_dark, Color::parse("#949494"), Color::parse("#5A5A5A"),
                ))
                .background(self.status_bar_bg),
            s,
        )
    }

    pub fn status_bar_note_style(&self, s: &str) -> String {
        Self::r(
            Style::new()
                .foreground(self.status_bar_note_fg)
                .background(self.status_bar_bg),
            s,
        )
    }

    pub fn status_bar_help_style(&self, s: &str) -> String {
        Self::r(
            Style::new()
                .foreground(self.status_bar_note_fg)
                .background(light_dark(
                    self.is_dark, Color::parse("#DCDCDC"), Color::parse("#323232"),
                )),
            s,
        )
    }

    pub fn status_bar_message_style(&self, s: &str) -> String {
        Self::r(
            Style::new()
                .foreground(self.mint_green)
                .background(self.dark_green_pager),
            s,
        )
    }

    pub fn status_bar_message_help_style(&self, s: &str) -> String {
        Self::r(
            Style::new()
                .foreground(Color::parse("#B6FFE4"))
                .background(self.green),
            s,
        )
    }

    pub fn help_view_style(&self, s: &str) -> String {
        Self::r(
            Style::new()
                .foreground(self.status_bar_note_fg)
                .background(light_dark(
                    self.is_dark, Color::parse("#f2f2f2"), Color::parse("#1B1B1B"),
                )),
            s,
        )
    }

    pub fn line_number_style(&self, s: &str) -> String {
        Self::r(Style::new().foreground(self.line_number_fg), s)
    }

    pub fn stash_spinner_style(&self) -> Style {
        Style::new().foreground(self.gray)
    }

    pub fn stash_input_prompt_style(&self) -> Style {
        Style::new().foreground(self.yellow_green).margin_right(1)
    }

    pub fn stash_input_cursor_style(&self) -> Style {
        Style::new().foreground(self.fuchsia).margin_right(1)
    }
}
