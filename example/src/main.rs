mod mock_lsp_server;
mod mock_lsp_server_async;

use dioxus::prelude::*;
use dioxus_codemirror::{CodeMirror, Language, LspBridge, Theme};

use crate::{mock_lsp_server::MockLspServer, mock_lsp_server_async::MockLspServerAsync};

/// Styling for the editors and the demo page.
///
/// The page follows the OS color scheme (`prefers-color-scheme`) to match the
/// CodeMirror component, which themes itself for light and dark automatically.
/// The editor border reuses the component's `--dxcm-border` variable so it
/// flips alongside the editor chrome.
const STYLE: &str = r#"
:root { color-scheme: light dark; }
body {
  font-family: sans-serif; max-width: 52rem; margin: 2rem auto; padding: 0 1rem;
  background: #ffffff; color: #1f2328;
}
.cm-editor { height: 12rem; border: 1px solid var(--dxcm-border, #ddd); font-size: 14px; }
.cm-scroller { overflow: auto; }
.dioxus-codemirror { margin-bottom: .5rem; }
section { margin-top: 1.5rem; }
pre { background: #f5f5f5; padding: .5rem; white-space: pre-wrap; word-break: break-all; }
button { padding: .4rem .8rem; }

@media (prefers-color-scheme: dark) {
  body { background: #0d1117; color: #e6edf3; }
  pre { background: #161b22; }
}
"#;

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    let value_plain = use_signal(|| "Edit me -- plain text, fully editable.\n".to_string());
    let value_yaml =
        use_signal(|| "name: example\nversion: 0.1.0\nitems:\n  - one\n  - two\n".to_string());
    let value_markdown =
        use_signal(|| "# Title\n\nSome **bold** and _italic_ text.\n\n- a\n- b\n".to_string());
    // Drives the YAML editor's `theme` prop below. `Theme::Auto` (the default)
    // follows the OS color scheme; the buttons pin a palette per editor.
    let mut theme_yaml = use_signal(|| Theme::Auto);
    let mut value_lsp = use_signal(|| "fn main() {\n    println!(\"hello\");\n}\n".to_string());

    let value_lsp_async =
        use_signal(|| "fn main() {\n    println!(\"diagnostics\");\n}\n".to_string());

    // The in-page mock language server. Its replies are returned synchronously
    // and forwarded straight back to the editor's LSP client.
    let mock = use_signal(MockLspServer::default);
    let lsp = LspBridge::lsp_bridge_from_server("file:///main.rs", mock);
    let lsp_log = mock.read().log.clone();

    // The async mock language server. It pushes its replies -- and unprompted
    // diagnostics -- back over a channel rather than returning them.
    let mock_async = use_signal(MockLspServerAsync::default);
    let lsp_async = LspBridge::lsp_bridge_from_server_async("file:///async.rs", mock_async);
    let lsp_async_log = mock_async.read().log.clone();

    rsx! {
        style { dangerous_inner_html: STYLE }
        h1 { "dioxus_codemirror example" }

        section {
            h2 { "1. Plain editable text + features" }
            p {
                "No line numbers, no language. Several CodeMirror features are on: "
                "Alt-click for multiple cursors, Mod-d to select the next match of "
                "the current word, matching-bracket highlighting, auto-closing "
                "brackets, and line wrapping."
            }
            CodeMirror {
                value: value_plain,
                allow_multiple_selections: true,
                highlight_selection_matches: true,
                highlight_active_line: true,
                bracket_matching: true,
                close_brackets: true,
                line_wrapping: true,
            }
            pre { "{value_plain}" }
        }

        section {
            h2 { "2. YAML with line numbers" }
            p {
                "The editor themes itself for light and dark automatically. Use the "
                "buttons to override the OS color scheme for this editor."
            }
            div {
                button { onclick: move |_| theme_yaml.set(Theme::Auto), "Auto" }
                button { onclick: move |_| theme_yaml.set(Theme::Light), "Light" }
                button { onclick: move |_| theme_yaml.set(Theme::Dark), "Dark" }
            }
            CodeMirror {
                value: value_yaml,
                line_numbers: true,
                language: Language::Yaml,
                theme: theme_yaml(),
            }
        }

        section {
            h2 { "3. Markdown with line numbers" }
            CodeMirror {
                value: value_markdown,
                line_numbers: true,
                language: Language::Markdown,
            }
        }

        section {
            h2 { "4. Set value externally + LSP" }
            CodeMirror { value: value_lsp, line_numbers: true, lsp }
            button {
                onclick: move |_| {
                    value_lsp.set("// replaced from outside the editor\nlet answer = 42;\n".to_string());
                },
                "Set to template"
            }
            p {
                "JSON-RPC exchanged with the in-page mock language server "
                "(--> to server, <-- to editor):"
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

        section {
            h2 { "5. Async LSP + server-pushed diagnostics" }
            p {
                "This editor uses an async bridge. The server pushes its replies "
                "back over a channel, and emits "
                code { "textDocument/publishDiagnostics" }
                " unprompted when the document opens or changes -- the case the "
                "synchronous transport cannot express."
            }
            CodeMirror { value: value_lsp_async, line_numbers: true, lsp: lsp_async }
            p {
                "JSON-RPC exchanged with the in-page async mock language server "
                "(--> to server, <-- pushed to editor):"
            }
            pre {
                if lsp_async_log.is_empty() {
                    "(waiting for the editor's LSP client to connect...)"
                } else {
                    for line in lsp_async_log.iter() {
                        "{line}\n"
                    }
                }
            }
        }
    }
}
