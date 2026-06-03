use dioxus::prelude::*;

use crate::lsp::{lsp_message::LspMessage, lsp_server::LspServer};

/// Connects the [`CodeMirror`] editor's LSP client to a language server.
///
/// When the editor's LSP client emits a JSON-RPC message for the server,
/// `on_message_to_server` is called with it and returns the server's reply
/// messages, which the component forwards straight back to the editor. The
/// exchange happens synchronously within the component's message loop, so there
/// is no round-trip latency through the render cycle.
///
/// Construct one directly, or from an [`LspServer`] with
/// [`LspBridge::lsp_bridge_from_server`].
///
/// [`CodeMirror`]: crate::code_mirror::CodeMirror
#[derive(Clone, PartialEq)]
pub struct LspBridge {
    /// File URI the LSP client operates on, e.g. `"file:///main.rs"`.
    pub uri: String,
    /// Handles a JSON-RPC message from the editor and returns the server's
    /// reply messages (often empty, e.g. for notifications).
    pub on_message_to_server: Callback<LspMessage, Vec<LspMessage>>,
}

impl LspBridge {
    /// Builds an [`LspBridge`] that drives an in-page [`LspServer`].
    ///
    /// Each message from the editor is handed to `server`, whose replies are
    /// returned to the editor. `uri` is the file URI, e.g. `"file:///main.rs"`.
    ///
    /// Call this from a component body (it allocates a [`Callback`] in the
    /// current scope).
    /// Must be called unconditionally from a component body -- it uses
    /// [`use_callback`] so the callback persists across re-renders (a plain
    /// `Callback::new` would be invalidated on the next render and then fail
    /// when the component's long-lived message loop calls it).
    pub fn lsp_bridge_from_server<S>(uri: impl Into<String>, mut server: Signal<S>) -> Self
    where
        S: LspServer + 'static,
    {
        let on_message_to_server =
            use_callback(move |message: LspMessage| server.write().lsp_message_handle(message));

        Self {
            uri: uri.into(),
            on_message_to_server,
        }
    }
}
