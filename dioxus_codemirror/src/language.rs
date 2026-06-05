use serde::{Deserialize, Serialize};

/// Syntax highlighting language for a [`CodeMirror`] editor.
///
/// Serializes to the lowercase name the glue script looks up in the bundled
/// `languages` map, e.g. `Language::Yaml` becomes `"yaml"`. Each variant maps
/// to a CodeMirror `@codemirror/lang-*` package.
///
/// A language is only bundled when its matching `lang-*` Cargo feature is
/// enabled on `dioxus_codemirror` (e.g. `Language::Css` needs `features =
/// ["lang-css"]`). The defaults are [`Language::Yaml`] and
/// [`Language::Markdown`]; selecting a language whose feature is disabled falls
/// back to plain text (with a console warning) rather than failing. See the
/// crate's `[features]` for the full list, or `lang-all` to bundle every one.
///
/// [`CodeMirror`]: crate::code_mirror::CodeMirror
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Language {
    /// YAML, e.g. `name: example`. Uses `@codemirror/lang-yaml` (`lang-yaml`).
    Yaml,
    /// Markdown, e.g. `# Heading`. Uses `@codemirror/lang-markdown`
    /// (`lang-markdown`).
    Markdown,
    /// JavaScript, e.g. `const x = 1;`. Uses `@codemirror/lang-javascript`
    /// (`lang-javascript`).
    Javascript,
    /// CSS, e.g. `a { color: red; }`. Uses `@codemirror/lang-css` (`lang-css`).
    Css,
    /// HTML, e.g. `<p>hi</p>`. Uses `@codemirror/lang-html` (`lang-html`).
    Html,
}
