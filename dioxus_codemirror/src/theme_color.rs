use serde::{Deserialize, Serialize};

/// CSS colour overrides for a single theme palette entry, per colour scheme.
///
/// Each field, when `Some`, overrides that entry's built-in source colour for
/// the matching scheme; `None` keeps the default. The value is any CSS colour,
/// e.g. `"#0d1117"`, `"oklch(92.9% 0.013 255.508)"`, or `"rgb(13 17 23)"`, and
/// is emitted verbatim into a CSS custom-property declaration.
///
/// See [`ThemeColors`] for the full palette.
///
/// [`ThemeColors`]: crate::theme_colors::ThemeColors
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ThemeColor {
    /// Colour for the light scheme, e.g. `Some("#ffffff".to_string())`. `None`
    /// keeps the default light colour.
    pub light: Option<String>,
    /// Colour for the dark scheme, e.g. `Some("#0d1117".to_string())`. `None`
    /// keeps the default dark colour.
    pub dark: Option<String>,
}

impl ThemeColor {
    /// Returns a `ThemeColor` overriding both schemes, e.g.
    /// `ThemeColor::new("#ffffff", "#0d1117")`.
    pub fn new(light: impl Into<String>, dark: impl Into<String>) -> Self {
        Self {
            light: Some(light.into()),
            dark: Some(dark.into()),
        }
    }

    /// Returns a `ThemeColor` overriding only the light scheme, e.g.
    /// `ThemeColor::light_only("oklch(92.9% 0.013 255.508)")`.
    pub fn light_only(light: impl Into<String>) -> Self {
        Self {
            light: Some(light.into()),
            dark: None,
        }
    }

    /// Returns a `ThemeColor` overriding only the dark scheme, e.g.
    /// `ThemeColor::dark_only("#0d1117")`.
    pub fn dark_only(dark: impl Into<String>) -> Self {
        Self {
            light: None,
            dark: Some(dark.into()),
        }
    }
}
