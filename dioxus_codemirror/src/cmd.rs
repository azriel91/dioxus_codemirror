use serde::Serialize;

use crate::language::Language;

/// Commands sent from Rust to the CodeMirror glue script (Rust -> JS).
///
/// Each variant serializes to a JSON object tagged with a `type` field, e.g.
/// `{ "type": "doc_set", "doc": "fn main() {}" }`. The `type` string is the
/// constant the glue script matches on, so the protocol stays in sync with the
/// JavaScript dispatcher at compile time on the Rust side.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Cmd {
    /// First message sent to the glue script, carrying the editor configuration.
    ///
    /// `mount_id`: the DOM id of the element the editor mounts into, e.g.
    /// `"cm-editor-0"`. `cm_base`: base URL of the vendored CodeMirror assets,
    /// e.g. `"/assets/codemirror"`. `doc`: the initial document text.
    /// `line_numbers`: whether to show the line-number gutter. `language`:
    /// syntax highlighting language, e.g. `Some(Language::Yaml)`, or `None` for
    /// plain text. The remaining flags toggle optional CodeMirror features (see
    /// the matching [`CodeMirrorProps`] fields). `lsp_uri`: the file URI to
    /// attach the LSP client for, e.g. `Some("file:///main.rs")`, or `None` to
    /// disable LSP.
    ///
    /// [`CodeMirrorProps`]: crate::code_mirror::CodeMirrorProps
    Init {
        mount_id: String,
        cm_base: String,
        doc: String,
        line_numbers: bool,
        language: Option<Language>,
        allow_multiple_selections: bool,
        highlight_selection_matches: bool,
        highlight_active_line: bool,
        bracket_matching: bool,
        close_brackets: bool,
        rectangular_selection: bool,
        indent_on_input: bool,
        highlight_whitespace: bool,
        line_wrapping: bool,
        read_only: bool,
        tab_size: Option<u8>,
        lsp_uri: Option<String>,
    },
    /// Replace the editor's document with `doc`, e.g. when the bound data
    /// changes elsewhere on the page.
    DocSet { doc: String },
    /// A JSON-RPC message from the language server to the editor's LSP client.
    ///
    /// `json` is a single serialized JSON-RPC object, e.g.
    /// `{"jsonrpc":"2.0","id":1,"result":{...}}`.
    LspMessageSend { json: String },
    /// Tear down the editor (e.g. on component unmount).
    Destroy,
}
