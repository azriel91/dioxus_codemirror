use serde::{Deserialize, Serialize};

/// Syntax highlighting language for a [`CodeMirror`] editor.
///
/// Serializes to the lowercase name the glue script matches on, e.g.
/// `Language::Yaml` becomes `"yaml"`. Each variant maps to a CodeMirror
/// `@codemirror/lang-*` package loaded on demand.
///
/// [`CodeMirror`]: crate::code_mirror::CodeMirror
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Language {
    /// YAML, e.g. `name: example`. Uses `@codemirror/lang-yaml`.
    Yaml,
    /// Markdown, e.g. `# Heading`. Uses `@codemirror/lang-markdown`.
    Markdown,
}
