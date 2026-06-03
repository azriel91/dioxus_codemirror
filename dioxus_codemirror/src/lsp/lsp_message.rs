/// A single JSON-RPC message exchanged between the editor's LSP client and a
/// language server.
///
/// The message is the serialized JSON-RPC object as a string, without
/// `Content-Length` framing -- the `@codemirror/lsp-client` transport is
/// message-based, so each `LspMessage` is exactly one JSON-RPC object, e.g.
/// `{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LspMessage {
    json: String,
}

impl LspMessage {
    /// Returns a new `LspMessage` wrapping the given JSON-RPC `json` string.
    pub fn new(json: impl Into<String>) -> Self {
        Self { json: json.into() }
    }

    /// Returns the JSON-RPC message as a string slice.
    pub fn json(&self) -> &str {
        &self.json
    }

    /// Consumes the message, returning the owned JSON-RPC string.
    pub fn json_into(self) -> String {
        self.json
    }
}
