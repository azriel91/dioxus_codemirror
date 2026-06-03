mod mock_lsp_server;

use dioxus::prelude::*;
use dioxus_codemirror::{CodeMirror, LspBridge, LspMessage};

use crate::mock_lsp_server::MockLspServer;

/// Styling for the editor and the demo page.
const STYLE: &str = r#"
body { font-family: sans-serif; max-width: 52rem; margin: 2rem auto; padding: 0 1rem; }
.cm-editor { height: 18rem; border: 1px solid #ddd; font-size: 14px; }
.cm-scroller { overflow: auto; }
.dioxus-codemirror { margin-bottom: 1rem; }
section { margin-top: 1.5rem; }
pre { background: #f5f5f5; padding: .5rem; white-space: pre-wrap; word-break: break-all; }
button { padding: .4rem .8rem; }
"#;

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    let mut value = use_signal(|| "fn main() {\n    println!(\"hello\");\n}\n".to_string());

    // The in-page mock language server, and the queue of messages it sends back
    // to the editor's LSP client.
    let mock = use_signal(MockLspServer::default);
    let lsp_inbound = use_signal(Vec::<LspMessage>::new);
    let lsp = LspBridge::lsp_bridge_from_server("file:///main.rs", mock, lsp_inbound);

    let lsp_log = mock.read().log.clone();

    rsx! {
        style { dangerous_inner_html: STYLE }
        h1 { "dioxus_codemirror example" }

        CodeMirror { value, lsp }

        section {
            h2 { "1. React to edits" }
            p { "The editor's text, mirrored live from the bound signal:" }
            pre { "{value}" }
        }

        section {
            h2 { "2. Set the value externally" }
            button {
                onclick: move |_| {
                    value.set("// replaced from outside the editor\nlet answer = 42;\n".to_string());
                },
                "Set to template"
            }
        }

        section {
            h2 { "3. LSP traffic" }
            p {
                "JSON-RPC exchanged with the in-page mock language server "
                "(--> to server, <-- to editor). Hover over the code to trigger a request."
            }
            pre {
                if lsp_log.is_empty() {
                    "(waiting for the editor's LSP client to connect...)"
                } else {
                    for line in lsp_log.iter() {
                        "{line}\n"
                    }
                }
            }
        }
    }
}
