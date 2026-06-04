use serde::{Deserialize, Serialize};

/// Optional CodeMirror editor features, each toggled independently.
///
/// Field names mirror the CodeMirror extension they enable, for API parity --
/// e.g. [`EditorFeatures::allow_multiple_selections`] enables
/// `EditorState.allowMultipleSelections`, and
/// [`EditorFeatures::highlight_selection_matches`] enables
/// `highlightSelectionMatches`. All default to off; pass an instance to
/// [`CodeMirrorProps::features`].
///
/// Construct with [`EditorFeatures::default`] and the chained builder methods,
/// e.g.:
///
/// ```
/// # use dioxus_codemirror::EditorFeatures;
/// let features = EditorFeatures::default()
///     .allow_multiple_selections()
///     .highlight_selection_matches()
///     .tab_size(2);
/// ```
///
/// [`CodeMirrorProps::features`]: crate::code_mirror::CodeMirrorProps::features
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct EditorFeatures {
    /// Allow multiple selections / cursors (e.g. via `Alt`-click), mapping to
    /// `EditorState.allowMultipleSelections`. Secondary selections are drawn by
    /// the `drawSelection` extension that the base setup already includes.
    pub allow_multiple_selections: bool,
    /// Highlight other occurrences of the current word/selection, and bind
    /// `Mod-d` to select the next occurrence -- mapping to
    /// `highlightSelectionMatches` plus the `selectNextOccurrence` command.
    pub highlight_selection_matches: bool,
    /// Highlight the line the primary cursor is on, mapping to
    /// `highlightActiveLine`.
    pub highlight_active_line: bool,
    /// Highlight the bracket matching the one next to the cursor, mapping to
    /// `bracketMatching`.
    pub bracket_matching: bool,
    /// Auto-insert closing brackets and quotes, mapping to `closeBrackets`.
    pub close_brackets: bool,
    /// Allow rectangular (block) selection via `Alt`-drag, mapping to
    /// `rectangularSelection` plus `crosshairCursor`.
    pub rectangular_selection: bool,
    /// Re-indent lines as you type, mapping to `indentOnInput`.
    pub indent_on_input: bool,
    /// Render whitespace characters visibly, mapping to `highlightWhitespace`.
    pub highlight_whitespace: bool,
    /// Wrap long lines instead of scrolling horizontally, mapping to
    /// `EditorView.lineWrapping`.
    pub line_wrapping: bool,
    /// Make the document read-only, mapping to `EditorState.readOnly`.
    pub read_only: bool,
    /// Width of a tab in spaces, mapping to `EditorState.tabSize`, e.g.
    /// `Some(2)`. `None` keeps CodeMirror's default.
    pub tab_size: Option<u8>,
}

impl EditorFeatures {
    /// Enables [`Self::allow_multiple_selections`].
    pub fn allow_multiple_selections(mut self) -> Self {
        self.allow_multiple_selections = true;
        self
    }

    /// Enables [`Self::highlight_selection_matches`].
    pub fn highlight_selection_matches(mut self) -> Self {
        self.highlight_selection_matches = true;
        self
    }

    /// Enables [`Self::highlight_active_line`].
    pub fn highlight_active_line(mut self) -> Self {
        self.highlight_active_line = true;
        self
    }

    /// Enables [`Self::bracket_matching`].
    pub fn bracket_matching(mut self) -> Self {
        self.bracket_matching = true;
        self
    }

    /// Enables [`Self::close_brackets`].
    pub fn close_brackets(mut self) -> Self {
        self.close_brackets = true;
        self
    }

    /// Enables [`Self::rectangular_selection`].
    pub fn rectangular_selection(mut self) -> Self {
        self.rectangular_selection = true;
        self
    }

    /// Enables [`Self::indent_on_input`].
    pub fn indent_on_input(mut self) -> Self {
        self.indent_on_input = true;
        self
    }

    /// Enables [`Self::highlight_whitespace`].
    pub fn highlight_whitespace(mut self) -> Self {
        self.highlight_whitespace = true;
        self
    }

    /// Enables [`Self::line_wrapping`].
    pub fn line_wrapping(mut self) -> Self {
        self.line_wrapping = true;
        self
    }

    /// Enables [`Self::read_only`].
    pub fn read_only(mut self) -> Self {
        self.read_only = true;
        self
    }

    /// Sets [`Self::tab_size`] to `tab_size` spaces, e.g. `2`.
    pub fn tab_size(mut self, tab_size: u8) -> Self {
        self.tab_size = Some(tab_size);
        self
    }
}
