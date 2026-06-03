use dioxus::prelude::*;

use crate::lsp::{lsp_message::LspMessage, lsp_server::LspServer};

/// Connects the [`CodeMirror`] editor's LSP client to a language server.
///
/// The bridge carries messages in both directions:
///
/// * `on_message_to_server` is called with each JSON-RPC message the editor's
///   LSP client emits towards the server.
/// * `messages_to_client` is a signal the editor watches; pushing an
///   [`LspMessage`] onto it forwards that message to the editor's LSP client.
///
/// Construct one directly, or from an [`LspServer`] with
/// [`LspBridge::lsp_bridge_from_server`].
///
/// [`CodeMirror`]: crate::code_mirror::CodeMirror
#[derive(Clone, PartialEq)]
pub struct LspBridge {
    /// File URI the LSP client operates on, e.g. `"file:///main.rs"`.
    pub uri: String,
    /// Invoked with each JSON-RPC message the editor sends to the server.
    pub on_message_to_server: Callback<LspMessage>,
    /// Messages destined for the editor's LSP client; new entries are
    /// forwarded to the editor as they are appended.
    pub messages_to_client: ReadSignal<Vec<LspMessage>>,
}

impl LspBridge {
    /// Builds an [`LspBridge`] that drives an in-page [`LspServer`].
    ///
    /// Messages from the editor are handed to `server`, and any messages it
    /// returns are appended to `messages_to_client` for delivery back to the
    /// editor. `uri` is the file URI, e.g. `"file:///main.rs"`.
    ///
    /// Call this from a component body (it allocates a [`Callback`] in the
    /// current scope).
    pub fn lsp_bridge_from_server<S>(
        uri: impl Into<String>,
        mut server: Signal<S>,
        mut messages_to_client: Signal<Vec<LspMessage>>,
    ) -> Self
    where
        S: LspServer + 'static,
    {
        let on_message_to_server = Callback::new(move |message: LspMessage| {
            let responses = server.write().lsp_message_handle(message);
            if !responses.is_empty() {
                messages_to_client.write().extend(responses);
            }
        });

        Self {
            uri: uri.into(),
            on_message_to_server,
            messages_to_client: messages_to_client.into(),
        }
    }
}
