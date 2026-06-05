use dioxus_codemirror::{LspMessage, LspPusher, LspServerAsync};
use serde_json::{json, Value};

/// A stand-in async language server that *pushes* its messages back.
///
/// Unlike [`MockLspServer`], which returns its replies synchronously, this one
/// holds an [`LspPusher`] and pushes everything onto it: request responses, and
/// -- crucially -- unprompted `textDocument/publishDiagnostics` notifications
/// it emits when the document opens or changes. That demonstrates the
/// server-push path end to end, the same way a Web Worker-backed server (whose
/// replies genuinely arrive later) would behave. Swap this out for a real WASM
/// [`LspServerAsync`] to get genuine language features.
///
/// [`MockLspServer`]: crate::mock_lsp_server::MockLspServer
#[derive(Clone, Default)]
pub struct MockLspServerAsync {
    /// Channel to push messages to the editor; set once via [`lsp_pusher_set`].
    ///
    /// [`lsp_pusher_set`]: LspServerAsync::lsp_pusher_set
    pusher: Option<LspPusher>,
    /// Log of messages received (`-->`) and pushed (`<--`), newest last.
    pub log: Vec<String>,
}

impl PartialEq for MockLspServerAsync {
    fn eq(&self, other: &Self) -> bool {
        // The pusher has no meaningful equality; the log drives UI updates.
        self.log == other.log
    }
}

impl MockLspServerAsync {
    /// Pushes `message` to the editor and logs it, if the pusher is connected.
    fn lsp_message_push(&mut self, message: LspMessage) {
        self.log.push(format!("<-- {}", message.json()));
        if let Some(pusher) = self.pusher.as_ref() {
            pusher.lsp_message_push(message);
        }
    }

    /// Returns a canned `publishDiagnostics` notification for `uri`.
    ///
    /// A real server would compute these; here it always flags the first two
    /// characters of the first line, e.g. to show a lint squiggle.
    fn lsp_diagnostics_publish(uri: &str) -> LspMessage {
        let notification = json!({
            "jsonrpc": "2.0",
            "method": "textDocument/publishDiagnostics",
            "params": {
                "uri": uri,
                "diagnostics": [{
                    "range": {
                        "start": { "line": 0, "character": 0 },
                        "end": { "line": 0, "character": 2 }
                    },
                    "severity": 2,
                    "source": "mock-lsp-async",
                    "message": "mock diagnostic -- pushed by the async LSP server"
                }]
            }
        });

        LspMessage::new(notification.to_string())
    }
}

impl LspServerAsync for MockLspServerAsync {
    fn lsp_pusher_set(&mut self, pusher: LspPusher) {
        self.pusher = Some(pusher);
    }

    fn lsp_message_handle(&mut self, message: LspMessage) {
        self.log.push(format!("--> {}", message.json()));

        let request: Value = serde_json::from_str(message.json()).unwrap_or(Value::Null);
        let method = request.get("method").and_then(Value::as_str);

        // Requests (which carry an `id`) get a response pushed back.
        if let Some(id) = request.get("id").filter(|id| !id.is_null()).cloned() {
            let result = match method {
                Some("initialize") => json!({
                    "capabilities": {
                        "textDocumentSync": 1,
                        "hoverProvider": true
                    },
                    "serverInfo": { "name": "mock-lsp-async", "version": "0.1.0" }
                }),
                Some("textDocument/hover") => json!({
                    "contents": {
                        "kind": "markdown",
                        "value": "**mock hover** -- pushed by the async LSP server"
                    }
                }),
                _ => Value::Null,
            };

            let response = json!({ "jsonrpc": "2.0", "id": id, "result": result }).to_string();
            self.lsp_message_push(LspMessage::new(response));
        }

        // When the document opens or changes, push diagnostics out-of-band --
        // the case the synchronous transport cannot express.
        if matches!(
            method,
            Some("textDocument/didOpen") | Some("textDocument/didChange")
        ) && let Some(uri) = request
            .get("params")
            .and_then(|params| params.get("textDocument"))
            .and_then(|text_document| text_document.get("uri"))
            .and_then(Value::as_str)
        {
            let diagnostics = Self::lsp_diagnostics_publish(uri);
            self.lsp_message_push(diagnostics);
        }
    }
}
