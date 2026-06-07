use std::fmt::Write;

use crate::theme_color::ThemeColor;

/// Per-editor CSS colour overrides for the [`CodeMirror`] theme palette.
///
/// Each field overrides one entry of the built-in palette for the light and/or
/// dark scheme (see [`ThemeColor`]); unset fields keep the defaults. Overrides
/// are emitted as inline CSS custom properties on the editor's mount element
/// (see [`Self::style_attr`]), so they apply to that editor only and win over
/// the shared default stylesheet.
///
/// # Example
///
/// ```
/// use dioxus_codemirror::{ThemeColor, ThemeColors};
///
/// let theme_colors = ThemeColors {
///     bg: ThemeColor::new("#ffffff", "#0d1117"),
///     syntax_keyword: ThemeColor::dark_only("#ff7b72"),
///     ..Default::default()
/// };
/// ```
///
/// [`CodeMirror`]: crate::code_mirror::CodeMirror
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ThemeColors {
    /// Editor background. CSS var `--dxcm-bg`.
    pub bg: ThemeColor,
    /// Editor foreground (default text). CSS var `--dxcm-fg`.
    pub fg: ThemeColor,
    /// Text cursor colour. CSS var `--dxcm-caret`.
    pub caret: ThemeColor,
    /// Selection background (unfocused). CSS var `--dxcm-selection`.
    pub selection: ThemeColor,
    /// Selection background (focused). CSS var `--dxcm-selection-focused`.
    pub selection_focused: ThemeColor,
    /// Other selection-match occurrences. CSS var `--dxcm-selection-match`.
    pub selection_match: ThemeColor,
    /// Active selection-match occurrence. CSS var
    /// `--dxcm-selection-match-main`.
    pub selection_match_main: ThemeColor,
    /// Gutter background. CSS var `--dxcm-gutter-bg`.
    pub gutter_bg: ThemeColor,
    /// Gutter foreground (line numbers). CSS var `--dxcm-gutter-fg`.
    pub gutter_fg: ThemeColor,
    /// Rendered-whitespace dots. CSS var `--dxcm-highlight-space`.
    pub highlight_space: ThemeColor,
    /// Active line background. CSS var `--dxcm-active-line`.
    pub active_line: ThemeColor,
    /// Active line gutter background. CSS var `--dxcm-active-line-gutter-bg`.
    pub active_line_gutter_bg: ThemeColor,
    /// Borders / focus outline. CSS var `--dxcm-border`.
    pub border: ThemeColor,
    /// Tooltip background. CSS var `--dxcm-tooltip-bg`.
    pub tooltip_bg: ThemeColor,
    /// Tooltip foreground. CSS var `--dxcm-tooltip-fg`.
    pub tooltip_fg: ThemeColor,
    /// Selected tooltip item background. CSS var `--dxcm-tooltip-selected-bg`.
    pub tooltip_selected_bg: ThemeColor,
    /// Selected tooltip item foreground. CSS var `--dxcm-tooltip-selected-fg`.
    pub tooltip_selected_fg: ThemeColor,
    /// Completion-info panel background. CSS var `--dxcm-tooltip-info-bg`.
    pub tooltip_info_bg: ThemeColor,
    /// Syntax: keywords. CSS var `--dxcm-syntax-keyword`.
    pub syntax_keyword: ThemeColor,
    /// Syntax: strings. CSS var `--dxcm-syntax-string`.
    pub syntax_string: ThemeColor,
    /// Syntax: comments. CSS var `--dxcm-syntax-comment`.
    pub syntax_comment: ThemeColor,
    /// Syntax: numbers. CSS var `--dxcm-syntax-number`.
    pub syntax_number: ThemeColor,
    /// Syntax: functions. CSS var `--dxcm-syntax-function`.
    pub syntax_function: ThemeColor,
    /// Syntax: types. CSS var `--dxcm-syntax-type`.
    pub syntax_type: ThemeColor,
    /// Syntax: constants. CSS var `--dxcm-syntax-constant`.
    pub syntax_constant: ThemeColor,
    /// Syntax: operators. CSS var `--dxcm-syntax-operator`.
    pub syntax_operator: ThemeColor,
    /// Syntax: properties. CSS var `--dxcm-syntax-property`.
    pub syntax_property: ThemeColor,
    /// Syntax: headings. CSS var `--dxcm-syntax-heading`.
    pub syntax_heading: ThemeColor,
    /// Syntax: links. CSS var `--dxcm-syntax-link`.
    pub syntax_link: ThemeColor,
    /// Syntax: invalid tokens. CSS var `--dxcm-syntax-invalid`.
    pub syntax_invalid: ThemeColor,
}

impl ThemeColors {
    /// Returns the palette entries paired with their CSS-variable suffix (the
    /// hyphenated `--dxcm-<suffix>` name), in palette order.
    ///
    /// Single source of truth mapping each field to its CSS variable, e.g.
    /// `("syntax-keyword", &self.syntax_keyword)`. The suffixes must match the
    /// `THEME_PALETTE` names in `code_mirror/glue.js`.
    fn entries(&self) -> [(&'static str, &ThemeColor); 30] {
        [
            ("bg", &self.bg),
            ("fg", &self.fg),
            ("caret", &self.caret),
            ("selection", &self.selection),
            ("selection-focused", &self.selection_focused),
            ("selection-match", &self.selection_match),
            ("selection-match-main", &self.selection_match_main),
            ("gutter-bg", &self.gutter_bg),
            ("gutter-fg", &self.gutter_fg),
            ("highlight-space", &self.highlight_space),
            ("active-line", &self.active_line),
            ("active-line-gutter-bg", &self.active_line_gutter_bg),
            ("border", &self.border),
            ("tooltip-bg", &self.tooltip_bg),
            ("tooltip-fg", &self.tooltip_fg),
            ("tooltip-selected-bg", &self.tooltip_selected_bg),
            ("tooltip-selected-fg", &self.tooltip_selected_fg),
            ("tooltip-info-bg", &self.tooltip_info_bg),
            ("syntax-keyword", &self.syntax_keyword),
            ("syntax-string", &self.syntax_string),
            ("syntax-comment", &self.syntax_comment),
            ("syntax-number", &self.syntax_number),
            ("syntax-function", &self.syntax_function),
            ("syntax-type", &self.syntax_type),
            ("syntax-constant", &self.syntax_constant),
            ("syntax-operator", &self.syntax_operator),
            ("syntax-property", &self.syntax_property),
            ("syntax-heading", &self.syntax_heading),
            ("syntax-link", &self.syntax_link),
            ("syntax-invalid", &self.syntax_invalid),
        ]
    }

    /// Returns the inline `style` attribute value declaring the overridden
    /// source CSS variables, e.g.
    /// `"--dxcm-light-bg: #ffffff; --dxcm-dark-bg: #0d1117;"`, or `None` when
    /// no overrides are set.
    ///
    /// Only source variables (`--dxcm-light-<name>` / `--dxcm-dark-<name>`) are
    /// emitted; the active aliases set by the shared stylesheet then resolve to
    /// these, so scheme switching still works.
    pub fn style_attr(&self) -> Option<String> {
        let mut style = String::new();
        for (name, theme_color) in self.entries() {
            if let Some(light) = theme_color.light.as_deref() {
                let _ = write!(style, "--dxcm-light-{name}: {light}; ");
            }
            if let Some(dark) = theme_color.dark.as_deref() {
                let _ = write!(style, "--dxcm-dark-{name}: {dark}; ");
            }
        }
        let style = style.trim_end().to_string();
        (!style.is_empty()).then_some(style)
    }
}
