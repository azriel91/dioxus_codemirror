use serde::Deserialize;

/// Events received from the CodeMirror glue script in Rust (JS -> Rust).
///
/// Each variant is deserialized from a JSON object tagged with a `type` field,
/// e.g. `{ "type": "doc_changed", "doc": "fn main() {}" }`. The glue script
/// produces these via `dioxus.send(..)`.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Evt {
    /// The editor has been created and mounted.
    Ready,
    /// The user changed the document; `doc` is the full new text.
    DocChanged { doc: String },
    /// A JSON-RPC message from the editor's LSP client to the language server.
    ///
    /// `json` is a single serialized JSON-RPC object, e.g.
    /// `{"jsonrpc":"2.0","id":1,"method":"initialize","params":{...}}`.
    LspMessageRecv { json: String },
}
