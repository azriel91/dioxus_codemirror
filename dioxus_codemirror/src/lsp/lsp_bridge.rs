use std::{cell::RefCell, rc::Rc};

use dioxus::prelude::*;

use crate::lsp::{
    lsp_message::LspMessage, lsp_pusher::LspMessageRx, lsp_server::LspServer,
    lsp_server_async::LspServerAsync,
};

/// Connects the [`CodeMirror`] editor's LSP client to a language server.
///
/// There are two transport flavours, picked by which constructor you call:
///
/// 1. **Synchronous** -- [`LspBridge::lsp_bridge_from_server`]. Each message
///    from the editor is handed to the server and the messages it *returns* are
///    forwarded straight back, within the component's message loop, so there is
///    no round-trip latency through the render cycle. Covers requests and
///    notifications, but not server-initiated messages.
/// 2. **Async / worker-capable** -- [`LspBridge::lsp_bridge_from_server_async`].
///    Messages from the editor are handed to the server fire-and-forget; the
///    server pushes replies and unprompted messages (e.g.
///    `textDocument/publishDiagnostics` diagnostics) back over a channel at any
///    time, which the component drains into the editor. This is the path for a
///    server running in a Web Worker, or one that emits diagnostics after a
///    processing step.
///
/// [`CodeMirror`]: crate::code_mirror::CodeMirror
#[derive(Clone)]
pub struct LspBridge {
    /// File URI the LSP client operates on, e.g. `"file:///main.rs"`.
    pub uri: String,
    /// Handles a JSON-RPC message from the editor and returns the server's
    /// synchronous reply messages (often empty -- e.g. for notifications, or for
    /// an async server whose replies arrive later via [`messages_pushed_rx`]).
    ///
    /// [`messages_pushed_rx`]: LspBridge::messages_pushed_rx
    pub on_message_to_server: Callback<LspMessage, Vec<LspMessage>>,
    /// Receiver of server-pushed messages, drained once by the component.
    ///
    /// `Some` for an async bridge; `None` for a synchronous one (which has no
    /// out-of-band push path). Wrapped in `Rc<RefCell<Option<_>>>` so the bridge
    /// stays [`Clone`] (the receiver is not) while letting the component take the
    /// receiver exactly once.
    messages_pushed_rx: Rc<RefCell<Option<LspMessageRx>>>,
}

impl PartialEq for LspBridge {
    fn eq(&self, other: &Self) -> bool {
        // The receiver cell is compared by identity: two bridges are the same
        // bridge only if they share the same push channel. `Callback`s and the
        // `Rc` both have stable identity across re-renders (they come from
        // `use_callback` / `use_hook`), so equal bridges stay equal -- which is
        // what the prop diff relies on to avoid needless re-renders.
        self.uri == other.uri
            && self.on_message_to_server == other.on_message_to_server
            && Rc::ptr_eq(&self.messages_pushed_rx, &other.messages_pushed_rx)
    }
}

impl LspBridge {
    /// Builds a synchronous [`LspBridge`] that drives an in-page [`LspServer`].
    ///
    /// Each message from the editor is handed to `server`, whose returned
    /// replies are forwarded back to the editor. `uri` is the file URI, e.g.
    /// `"file:///main.rs"`. There is no server-push path -- for that, see
    /// [`LspBridge::lsp_bridge_from_server_async`].
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

        // No push channel for the synchronous transport.
        let messages_pushed_rx = use_hook(|| Rc::new(RefCell::new(None)));

        Self {
            uri: uri.into(),
            on_message_to_server,
            messages_pushed_rx,
        }
    }

    /// Builds an async [`LspBridge`] that drives an in-page [`LspServerAsync`].
    ///
    /// A push channel is created and its [`LspPusher`] handed to `server` once
    /// (via [`LspServerAsync::lsp_pusher_set`]); the component drains the
    /// receiving end and forwards whatever the server pushes -- async replies and
    /// server-initiated diagnostics alike -- to the editor. Messages from the
    /// editor are delivered to the server fire-and-forget, so handling them never
    /// blocks the component's message loop.
    ///
    /// `uri` is the file URI, e.g. `"file:///main.rs"`.
    ///
    /// Must be called unconditionally from a component body -- it uses
    /// [`use_hook`]/[`use_callback`] so the channel and callback persist across
    /// re-renders.
    ///
    /// [`LspPusher`]: crate::lsp::lsp_pusher::LspPusher
    pub fn lsp_bridge_from_server_async<S>(uri: impl Into<String>, mut server: Signal<S>) -> Self
    where
        S: LspServerAsync + 'static,
    {
        // Create the push channel once and give the server its pusher.
        let messages_pushed_rx = use_hook(|| {
            let (pusher, message_rx) = crate::lsp::lsp_pusher::LspPusher::lsp_pusher_new();
            server.write().lsp_pusher_set(pusher);
            Rc::new(RefCell::new(Some(message_rx)))
        });

        // Editor messages are fire-and-forget: the server pushes any reply via
        // the pusher, so nothing is returned synchronously here.
        let on_message_to_server = use_callback(move |message: LspMessage| {
            server.write().lsp_message_handle(message);
            Vec::new()
        });

        Self {
            uri: uri.into(),
            on_message_to_server,
            messages_pushed_rx,
        }
    }

    /// Takes the server-push receiver, leaving `None` behind.
    ///
    /// Returns `Some` at most once (on the first call for an async bridge),
    /// `None` thereafter or for a synchronous bridge. The component calls this to
    /// claim the receiver for its drain loop.
    pub(crate) fn messages_pushed_rx_take(&self) -> Option<LspMessageRx> {
        self.messages_pushed_rx.borrow_mut().take()
    }
}
