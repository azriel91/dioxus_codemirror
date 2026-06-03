# dioxus_codemirror

A Dioxus **web** component that wraps the [CodeMirror 6] editor, for use in
Dioxus web applications.

It supports:

1. **Reacting to edits** -- a two-way bound `Signal<String>`.
2. **Setting the value externally** -- writing the bound signal replaces the
   editor's contents (with an echo-loop guard so the two stay in sync).
3. **Connecting to an in-page LSP server** -- a transport bridge to a language
   server running in the page (e.g. compiled to WASM).

No JavaScript build step is required: the component drives CodeMirror through a
single long-lived `document::eval` channel (`code_mirror/glue.js`). CodeMirror
itself is **vendored** as a Dioxus folder asset (`dioxus_codemirror/assets/codemirror/`),
so there is no runtime CDN dependency.

## Usage

```rust
use dioxus::prelude::*;
use dioxus_codemirror::{CodeMirror, Language};

#[component]
fn App() -> Element {
    let value = use_signal(|| "fn main() {}".to_string());
    rsx! {
        // Editing the editor updates `value`; setting `value` updates the editor.
        CodeMirror { value }
        pre { "{value}" }

        // Optional line-number gutter and syntax highlighting.
        CodeMirror { value, line_numbers: true, language: Language::Yaml }
    }
}
```

The editor is always editable. Props:

* `value: Signal<String>` -- two-way bound document text (required).
* `line_numbers: bool` -- show a line-number gutter (default `false`).
* `language: Language` -- syntax highlighting, `Language::Yaml` or
  `Language::Markdown` (default plain text).
* `lsp: LspBridge` -- connect an in-page language server (optional).

## Architecture

```
Rust (Dioxus / WASM)                       JS (document::eval, one per editor)
--------------------                       -----------------------------------
CodeMirror component                       glue.js:
  Signal<String> (two-way value)             - injects a module script: imports vendored CodeMirror
  use_future: recv loop  <-- Evt (JSON) --   - EditorView + updateListener -> dioxus.send(Evt)
  use_effect: value set  --- Cmd (JSON) -->   - command loop: await dioxus.recv() -> apply Cmd
  Cmd enum (serialize)                         - LSP Transport bridges to/from Rust via Cmd/Evt
  Evt enum (deserialize)
```

Messages are typed Rust enums (`Cmd` outbound, `Evt` inbound) serialized to JSON
with a `type` tag -- the tag is the exact string the glue script dispatches on,
so the wire protocol is compile-time-checked on the Rust side.

## LSP

`CodeMirror` takes an optional `LspBridge`, which connects the editor's
`@codemirror/lsp-client` to an [`LspServer`] -- the extension point a real
in-page language server implements (`fn lsp_message_handle(message) -> replies`).
The `example` ships a `MockLspServer` that returns canned JSON-RPC responses, so
the round trip is demonstrable without a real server. Replace it with your WASM
language server to get genuine language features.

### Limitation: only synchronous, prompted replies are forwarded

`lsp_message_handle` is request/response: each message from the editor is handed
to the server, and the messages it *returns* are forwarded straight back. This
covers requests (e.g. `initialize`, hover, completion) and notifications.

It does **not** yet support **server-initiated, unprompted messages** -- a
server pushing `textDocument/publishDiagnostics` (lint/error squiggles) some time
*after* a processing step, rather than as the return value of handling a request.
Supporting that needs an async path: a channel the server can push onto at any
time, drained into `Cmd::LspMessageSend`. A sketch:

* give `LspServer` a way to emit messages out-of-band (e.g. take a
  `Callback<LspMessage>` / channel sender on construction, or add a `poll`
  method), and
* in `code_mirror.rs`, forward those emissions to the editor from the message
  loop (alongside the existing `Evt::LspMessageRecv` handling) -- for example by
  `select!`ing over both the eval channel and the server's outbound channel.

Until then, diagnostics must be returned in response to a message the editor
sends (e.g. piggy-backed on the reply to a `didChange`/`didOpen`-triggered
request), not pushed spontaneously.

## Running the example

```sh
dx serve --platform web -p example
```

The example shows four editors:

1. **Plain editable text** -- type to edit; the mirrored text updates live.
2. **YAML** with line numbers and highlighting.
3. **Markdown** with line numbers and highlighting.
4. **Set value externally + LSP** -- the **Set to template** button replaces the
   contents, and the panel shows JSON-RPC flowing both ways to the mock server.

## Vendored CodeMirror assets

CodeMirror and its dependencies are vendored into
`dioxus_codemirror/assets/codemirror/` -- one ES module file per npm package,
with each package's imports rewritten to its siblings so the tree is self
contained and the core `@codemirror/state`/`view` modules load exactly once
(CodeMirror requires a single instance of each). The folder is exposed as a
Dioxus asset; the glue script imports the entry files from it.

### Refreshing / upgrading versions

The `xtask` crate fetches the modules from [esm.sh] (no npm required) and
rewrites their imports:

```sh
cargo run -p xtask -- vendor
```

Pinned versions live in `xtask/src/main.rs` (`package_spec`). Note the meta
`codemirror` package must be pinned to an exact version -- esm.sh resolves
`codemirror@6` to an unrelated CodeMirror 5 lineage. After changing a version
there, re-run the command and commit the regenerated `assets/codemirror/`.

[CodeMirror 6]: https://codemirror.net/
[esm.sh]: https://esm.sh/
[`LspServer`]: dioxus_codemirror/src/lsp/lsp_server.rs
