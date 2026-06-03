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
single long-lived `document::eval` channel (`code_mirror/glue.js`) and loads
CodeMirror at runtime from [esm.sh].

## Usage

```rust
use dioxus::prelude::*;
use dioxus_codemirror::CodeMirror;

#[component]
fn App() -> Element {
    let value = use_signal(|| "fn main() {}".to_string());
    rsx! {
        CodeMirror { value }
        // Editing the editor updates `value`; setting `value` updates the editor.
        pre { "{value}" }
    }
}
```

## Architecture

```
Rust (Dioxus / WASM)                       JS (document::eval, one per editor)
--------------------                       -----------------------------------
CodeMirror component                       glue.js:
  Signal<String> (two-way value)             - import() CodeMirror + lsp-client from esm.sh
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

## Running the example

```sh
dx serve --platform web -p example
```

Then open the served URL and:

1. Type in the editor -- the mirrored text below updates on each keystroke.
2. Click **Set to template** -- the editor's contents are replaced.
3. Watch the **LSP traffic** panel show JSON-RPC flowing both ways.

## Offline / no network

The glue script imports CodeMirror from esm.sh at runtime. To run fully offline,
fetch the bundled modules once (no npm required) and import the local file
instead, e.g.:

```sh
curl -L "https://esm.sh/codemirror@6?bundle&target=es2022" -o assets/codemirror.js
```

then change the `import(...)` URLs in `code_mirror/glue.js` to the vendored asset.

[CodeMirror 6]: https://codemirror.net/
[esm.sh]: https://esm.sh/
[`LspServer`]: dioxus_codemirror/src/lsp/lsp_server.rs
