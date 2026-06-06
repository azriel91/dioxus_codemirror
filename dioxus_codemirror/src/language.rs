use serde::{Deserialize, Serialize};

/// Syntax highlighting language for a [`CodeMirror`] editor.
///
/// Serializes to the lowercase name the glue script looks up in the bundled
/// `languages` map, e.g. `Language::Yaml` becomes `"yaml"`. Each variant maps
/// to a CodeMirror `@codemirror/lang-*` package.
///
/// Every language is currently bundled regardless of the enabled `lang-*` Cargo
/// features: the whole vendored superset is served because Dioxus cannot yet
/// serve a build-script-generated, per-feature asset folder (see
/// <https://github.com/DioxusLabs/dioxus/issues/4426> and
/// [`code_mirror`](crate::code_mirror)). Selecting any variant works; should the
/// looked-up name be missing from the `languages` map, the glue falls back to
/// plain text with a console warning rather than failing.
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
