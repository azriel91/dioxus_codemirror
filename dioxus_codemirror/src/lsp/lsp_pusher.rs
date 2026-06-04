use futures::channel::mpsc::{UnboundedReceiver, UnboundedSender};

use crate::lsp::lsp_message::LspMessage;

/// Receiver half of the server-to-editor push channel.
///
/// The [`CodeMirror`] component drains this and forwards each [`LspMessage`] to
/// the editor's LSP client via `Cmd::LspMessageSend`. It carries both async
/// replies and server-initiated messages (e.g. `textDocument/publishDiagnostics`
/// diagnostics) uniformly -- anything the server pushes, whenever it pushes it.
///
/// [`CodeMirror`]: crate::code_mirror::CodeMirror
pub type LspMessageRx = UnboundedReceiver<LspMessage>;

/// Sender half of the server-to-editor push channel, handed to an
/// [`LspServerAsync`] so it can emit messages to the editor at any time.
///
/// Unlike the synchronous [`LspServer`] -- whose only output is the `Vec` it
/// *returns* from handling a message -- an async / worker-backed server holds an
/// `LspPusher` and pushes onto it whenever it has something to say: a reply that
/// arrives later (e.g. from a Web Worker), or an unprompted notification like
/// `textDocument/publishDiagnostics`.
///
/// Cloneable, so the server can keep one copy to push from and move others into
/// worker message callbacks.
///
/// [`LspServerAsync`]: crate::lsp::lsp_server_async::LspServerAsync
/// [`LspServer`]: crate::lsp::lsp_server::LspServer
#[derive(Clone)]
pub struct LspPusher {
    /// Sends pushed messages to the [`LspMessageRx`] held by the component.
    message_tx: UnboundedSender<LspMessage>,
}

impl LspPusher {
    /// Returns a new `LspPusher`/[`LspMessageRx`] pair connected by an unbounded
    /// channel.
    ///
    /// The pusher goes to the server (via
    /// [`LspServerAsync::lsp_pusher_set`]); the receiver goes to the
    /// [`LspBridge`], from which the component drains it.
    ///
    /// [`LspServerAsync::lsp_pusher_set`]: crate::lsp::lsp_server_async::LspServerAsync::lsp_pusher_set
    /// [`LspBridge`]: crate::lsp::lsp_bridge::LspBridge
    pub fn lsp_pusher_new() -> (Self, LspMessageRx) {
        let (message_tx, message_rx) = futures::channel::mpsc::unbounded();
        (Self { message_tx }, message_rx)
    }

    /// Pushes `message` to the editor's LSP client.
    ///
    /// Returns `true` if the message was queued, or `false` if the receiving end
    /// is gone (the editor was torn down) -- in which case the server may stop
    /// pushing.
    pub fn lsp_message_push(&self, message: LspMessage) -> bool {
        self.message_tx.unbounded_send(message).is_ok()
    }
}
