use serde::{Deserialize, Serialize};

/// Color theme for a [`CodeMirror`] editor.
///
/// Drives the `data-theme` attribute on the editor's mount element, which the
/// vendored stylesheet keys off to choose its light or dark CSS variables (see
/// `code_mirror/glue.js`). Token colors track the same variables, so the whole
/// editor -- chrome and syntax highlighting -- switches together.
///
/// [`CodeMirror`]: crate::code_mirror::CodeMirror
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Theme {
    /// Follow the operating system's `prefers-color-scheme`. The default.
    #[default]
    Auto,
    /// Always use the light palette, e.g. when forcing light on a dark OS.
    Light,
    /// Always use the dark palette, e.g. when forcing dark on a light OS.
    Dark,
}

impl Theme {
    /// The `data-theme` attribute value the stylesheet matches on, e.g.
    /// `Theme::Dark` becomes `"dark"`.
    pub fn theme_attr(self) -> &'static str {
        match self {
            Theme::Auto => "auto",
            Theme::Light => "light",
            Theme::Dark => "dark",
        }
    }
}
