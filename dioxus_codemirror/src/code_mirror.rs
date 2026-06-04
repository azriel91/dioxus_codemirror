use std::sync::atomic::{AtomicU64, Ordering};

use dioxus::{
    document::{Eval, eval},
    prelude::*,
};
use futures::StreamExt;

use crate::{
    cmd::Cmd,
    editor_features::EditorFeatures,
    evt::Evt,
    language::Language,
    lsp::{lsp_bridge::LspBridge, lsp_message::LspMessage},
    theme::Theme,
};

/// Source counter for unique editor mount ids, e.g. `cm-editor-0`.
static EDITOR_ID_NEXT: AtomicU64 = AtomicU64::new(0);

/// Vendored CodeMirror modules, served as a folder so sibling imports between
/// the modules resolve. Refresh with `cargo run -p xtask -- vendor`.
const CM_ASSETS: Asset = asset!("/assets/codemirror", AssetOptions::folder());

/// Properties for the [`CodeMirror`] component.
#[derive(Props, Clone, PartialEq)]
pub struct CodeMirrorProps {
    /// Two-way bound document text.
    ///
    /// Editing in the browser writes the new text here; writing to it from
    /// elsewhere on the page replaces the editor's contents.
    pub value: Signal<String>,
    /// Show a line-number gutter. Defaults to `false`.
    #[props(default)]
    pub line_numbers: bool,
    /// Syntax highlighting language, e.g. `Language::Yaml`. Defaults to plain
    /// text (`None`).
    #[props(default)]
    pub language: Option<Language>,
    /// Optional CodeMirror features to enable, e.g.
    /// `EditorFeatures::default().allow_multiple_selections()`. Defaults to all
    /// off.
    #[props(default)]
    pub features: EditorFeatures,
    /// Color theme, e.g. `Theme::Dark`. Defaults to [`Theme::Auto`], which
    /// follows the operating system's `prefers-color-scheme`.
    #[props(default)]
    pub theme: Theme,
    /// Optional language server connection. When `Some`, an LSP client is
    /// attached for [`LspBridge::uri`].
    #[props(default)]
    pub lsp: Option<LspBridge>,
    /// Called once, after the editor has been created and mounted.
    #[props(default)]
    pub on_ready: Option<EventHandler<()>>,
}

/// A CodeMirror 6 editor wrapped as a Dioxus web component.
///
/// Drives the editor through a single long-lived `document::eval` channel (see
/// `code_mirror/glue.js`), exchanging typed [`Cmd`]/[`Evt`] messages so the
/// editor needs no JavaScript build step.
#[component]
pub fn CodeMirror(props: CodeMirrorProps) -> Element {
    let CodeMirrorProps {
        mut value,
        line_numbers,
        language,
        features,
        theme,
        lsp,
        on_ready,
    } = props;

    let mount_id = use_hook(|| {
        format!("cm-editor-{}", EDITOR_ID_NEXT.fetch_add(1, Ordering::Relaxed))
    });

    // The glue script's evaluator handle, shared with the doc-set effect once
    // the editor is created.
    let mut eval_handle = use_signal(|| None::<Eval>);
    // Last document text synced with the editor. Used to break the
    // edit -> signal -> doc_set -> edit echo loop on the Rust side.
    let mut doc_synced = use_signal(String::new);

    // === Create the editor and pump JS events (Evt) into Rust === //
    let mount_id_future = mount_id.clone();
    let lsp_push = lsp.clone();
    use_future(move || {
        let mount_id = mount_id_future.clone();
        let lsp = lsp.clone();
        async move {
            let mut evaluator = eval(include_str!("code_mirror/glue.js"));

            let init = Cmd::Init {
                mount_id,
                cm_base: CM_ASSETS.to_string(),
                doc: value.peek().clone(),
                line_numbers,
                language,
                features,
                lsp_uri: lsp.as_ref().map(|lsp| lsp.uri.clone()),
            };
            if evaluator.send(init).is_err() {
                return;
            }
            doc_synced.set(value.peek().clone());
            eval_handle.set(Some(evaluator));

            loop {
                match evaluator.recv::<Evt>().await {
                    Ok(Evt::Ready) => {
                        if let Some(on_ready) = on_ready.as_ref() {
                            on_ready.call(());
                        }
                    }
                    Ok(Evt::DocChanged { doc }) => {
                        // Record before writing `value` so the doc_set effect
                        // sees them equal and does not echo back to the editor.
                        doc_synced.set(doc.clone());
                        value.set(doc);
                    }
                    Ok(Evt::LspMessageRecv { json }) => {
                        // Hand the message to the server and forward its replies
                        // straight back to the editor's LSP client.
                        if let Some(lsp) = lsp.as_ref() {
                            let replies = lsp.on_message_to_server.call(LspMessage::new(json));
                            for reply in replies {
                                let _ = evaluator.send(Cmd::LspMessageSend {
                                    json: reply.json_into(),
                                });
                            }
                        }
                    }
                    // Channel closed (component unmounted) or a decode error.
                    Err(_) => break,
                }
            }
        }
    });

    // === Forward server-pushed LSP messages into the editor === //
    // An async bridge (see `LspBridge::lsp_bridge_from_server_async`) lets the
    // server push replies and unprompted messages -- e.g.
    // `textDocument/publishDiagnostics` -- at any time. Drain them here and hand
    // each to the editor's LSP client, the same way prompted replies are. The
    // synchronous bridge has no receiver, so this loop ends immediately.
    use_future(move || {
        let lsp_push = lsp_push.clone();
        async move {
            let Some(mut messages_pushed_rx) =
                lsp_push.as_ref().and_then(LspBridge::messages_pushed_rx_take)
            else {
                return;
            };

            while let Some(message) = messages_pushed_rx.next().await {
                // The editor exists by the time the server pushes (pushes are
                // driven by editor messages, which require a mounted editor); if
                // it does not yet, the message predates the LSP client and is
                // dropped.
                if let Some(evaluator) = eval_handle.peek().as_ref() {
                    let _ = evaluator.send(Cmd::LspMessageSend {
                        json: message.json_into(),
                    });
                }
            }
        }
    });

    // === Push external value changes into the editor (Cmd::DocSet) === //
    use_effect(move || {
        let value_current = value.read().clone();
        if value_current != *doc_synced.peek()
            && let Some(evaluator) = eval_handle.peek().as_ref()
            && evaluator
                .send(Cmd::DocSet {
                    doc: value_current.clone(),
                })
                .is_ok()
        {
            doc_synced.set(value_current);
        }
    });

    rsx! {
        div {
            id: "{mount_id}",
            class: "dioxus-codemirror",
            "data-theme": theme.theme_attr(),
        }
    }
}
