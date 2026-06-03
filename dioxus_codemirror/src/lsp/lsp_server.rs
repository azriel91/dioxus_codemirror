use crate::lsp::lsp_message::LspMessage;

/// A language server that runs in-page (typically compiled to WASM).
///
/// This is the extension point for connecting a real language server to the
/// [`CodeMirror`] component. A language server speaks the Language Server
/// Protocol -- JSON-RPC messages -- which, over a stream, would be framed with
/// `Content-Length` headers. Here the transport is message-based, so the seam
/// is simply "feed one message in, get zero or more messages out".
///
/// Implement this for your WASM language server, then bridge it to the editor
/// with [`LspBridge::lsp_bridge_from_server`].
///
/// [`CodeMirror`]: crate::code_mirror::CodeMirror
/// [`LspBridge::lsp_bridge_from_server`]: crate::lsp::lsp_bridge::LspBridge::lsp_bridge_from_server
pub trait LspServer {
    /// Handles a JSON-RPC `message` sent by the editor's LSP client to the
    /// server, returning any JSON-RPC messages the server replies with.
    ///
    /// For example, given an `initialize` request the server returns a single
    /// message containing the `InitializeResult`; a notification with no `id`
    /// may return an empty `Vec`.
    fn lsp_message_handle(&mut self, message: LspMessage) -> Vec<LspMessage>;
}
