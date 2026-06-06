use dioxus_codemirror::{LspMessage, LspServer};
use serde_json::{json, Value};

/// A stand-in language server that returns canned JSON-RPC responses.
///
/// It exists to demonstrate the LSP transport plumbing end-to-end without a
/// real language server: requests with an `id` receive a response, while
/// notifications are logged and acknowledged with no reply. Swap this out for a
/// real WASM [`LspServer`] to get genuine language features.
#[derive(Clone, Default, PartialEq, Eq)]
pub struct MockLspServer {
    /// Log of messages received (`-->`) and sent (`<--`), newest last.
    pub log: Vec<String>,
}

impl LspServer for MockLspServer {
    fn lsp_message_handle(&mut self, message: LspMessage) -> Vec<LspMessage> {
        self.log.push(format!("--> {}", message.json()));

        let request: Value = serde_json::from_str(message.json()).unwrap_or(Value::Null);
        let method = request.get("method").and_then(Value::as_str);

        // Only requests (which carry an `id`) get a response; notifications do not.
        let Some(id) = request.get("id").filter(|id| !id.is_null()).cloned() else {
            return Vec::new();
        };

        let result = match method {
            Some("initialize") => json!({
                "capabilities": {
                    "textDocumentSync": 1,
                    "hoverProvider": true
                },
                "serverInfo": { "name": "mock-lsp", "version": "0.1.0" }
            }),
            Some("textDocument/hover") => json!({
                "contents": {
                    "kind": "markdown",
                    "value": "**mock hover** -- response from the in-page LSP"
                }
            }),
            _ => Value::Null,
        };

        let response = json!({ "jsonrpc": "2.0", "id": id, "result": result }).to_string();
        self.log.push(format!("<-- {response}"));

        vec![LspMessage::new(response)]
    }
}
