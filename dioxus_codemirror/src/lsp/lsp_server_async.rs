use crate::lsp::{lsp_message::LspMessage, lsp_pusher::LspPusher};

/// An async / worker-capable language server.
///
/// This is the extension point for language servers whose work does *not*
/// complete within the call that receives a message -- for example one that
/// forwards requests to a Web Worker (or a real WASM server running off the
/// main thread) and gets answers back later, or one that emits diagnostics
/// spontaneously after a processing step.
///
/// Where the synchronous [`LspServer`] *returns* its replies, an
/// `LspServerAsync` pushes them: it is handed an [`LspPusher`] once via
/// [`lsp_pusher_set`], then for each message from the editor it does whatever
/// async work it needs and pushes any resulting messages -- replies and
/// unprompted notifications alike -- onto the pusher whenever they are ready.
/// This is what makes **server-pushed diagnostics**
/// (`textDocument/publishDiagnostics`) possible: nothing has to be returned in
/// response to a specific request.
///
/// Implement this for your worker-backed language server, then bridge it to the
/// editor with [`LspBridge::lsp_bridge_from_server_async`].
///
/// [`LspServer`]: crate::lsp::lsp_server::LspServer
/// [`lsp_pusher_set`]: LspServerAsync::lsp_pusher_set
/// [`LspBridge::lsp_bridge_from_server_async`]: crate::lsp::lsp_bridge::LspBridge::lsp_bridge_from_server_async
pub trait LspServerAsync {
    /// Hands the server an [`LspPusher`] to emit messages to the editor.
    ///
    /// Called once, before any [`lsp_message_handle`] call. Keep the pusher (it
    /// is [`Clone`]) so later async work -- a worker reply, a diagnostics run
    /// -- can push onto it at any time.
    ///
    /// [`lsp_message_handle`]: LspServerAsync::lsp_message_handle
    fn lsp_pusher_set(&mut self, pusher: LspPusher);

    /// Handles a JSON-RPC `message` sent by the editor's LSP client.
    ///
    /// Unlike [`LspServer::lsp_message_handle`], this returns nothing: the
    /// server pushes any reply -- now or later, from this thread or a worker
    /// callback -- via the [`LspPusher`] it was given in [`lsp_pusher_set`].
    ///
    /// [`LspServer::lsp_message_handle`]: crate::lsp::lsp_server::LspServer::lsp_message_handle
    /// [`lsp_pusher_set`]: LspServerAsync::lsp_pusher_set
    fn lsp_message_handle(&mut self, message: LspMessage);
}
