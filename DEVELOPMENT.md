# Development

This document covers the internals of `dioxus_codemirror`: how the component
talks to CodeMirror, the message protocol, the in-page LSP bridge, the vendored
assets, and how to add a language. For user-facing usage, see
[`README.md`](README.md).


## Running the example

Install `dx` (the Dioxus build tool):

```sh
cargo install cargo-binstall
cargo binstall dioxus-cli
```

Then serve the example:

```sh
dx serve --platform web -p example
```

The example shows five editors:

1. **Plain editable text** -- type to edit; the mirrored text updates live.
2. **YAML** with line numbers and highlighting.
3. **Markdown** with line numbers and highlighting.
4. **Set value externally + LSP** -- the **Set to template** button replaces the
   contents, and the panel shows JSON-RPC flowing both ways to the (synchronous)
   mock server.
5. **Async LSP + server-pushed diagnostics** -- the async mock server pushes its
   replies and emits `textDocument/publishDiagnostics` unprompted on
   open/change; the panel shows the pushed JSON-RPC.

The example is also deployed at <https://azriel.im/dioxus_codemirror>.


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

The component drives CodeMirror through a single long-lived `document::eval`
channel (`dioxus_codemirror/src/code_mirror/glue.js`).

Messages are typed Rust enums (`Cmd` outbound, `Evt` inbound) serialized to JSON
with a `type` tag -- the tag is the exact string the glue script dispatches on,
so the wire protocol is compile-time-checked on the Rust side.


## LSP

`CodeMirror` takes an optional `LspBridge`, which connects the editor's
`@codemirror/lsp-client` to an in-page language server. There are two transport
flavours, chosen by which constructor you call.

### Synchronous: `LspBridge::lsp_bridge_from_server`

Drives an [`LspServer`] -- the extension point a real in-page language server
implements (`fn lsp_message_handle(message) -> replies`). It is request/response:
each message from the editor is handed to the server, and the messages it
*returns* are forwarded straight back, within the component's message loop, so
there is no round-trip latency through the render cycle. This covers requests
(e.g. `initialize`, hover, completion) and notifications.

The `example` ships a `MockLspServer` that returns canned JSON-RPC responses, so
the round trip is demonstrable without a real server.

### Async / worker-capable: `LspBridge::lsp_bridge_from_server_async`

Drives an [`LspServerAsync`], for a server whose work does not complete within
the call that receives a message -- one running in a **Web Worker** (or a real
WASM server off the main thread), or one that emits **diagnostics** after a
processing step.

Instead of returning replies, the server is handed an `LspPusher` once (via
`fn lsp_pusher_set(pusher)`) and pushes messages onto it whenever they are ready
(`fn lsp_message_handle(message)` returns nothing). The component drains the
receiving end and forwards each message to the editor, the same way prompted
replies are. Because the push path is independent of any request, this is what
makes **server-initiated, unprompted messages** work -- a server pushing
`textDocument/publishDiagnostics` (lint/error squiggles) some time *after* a
`didOpen`/`didChange`, rather than as the return value of handling a request.

The `example` ships a `MockLspServerAsync` that pushes its responses and emits
diagnostics on open/change. Replace either mock with your WASM language server to
get genuine language features.


## Vendored CodeMirror assets

CodeMirror and the full language superset are vendored into
`dioxus_codemirror/assets/codemirror-vendor/` -- one ES module file per npm
package, with each package's imports rewritten to its siblings, plus a generated
`index.js` entry. This directory is committed and is served directly as the
Dioxus folder asset (`code_mirror.rs`). The glue script imports that single
`index.js`.

### Why the whole superset ships

The served folder must live inside the crate, because manganis (Dioxus's asset
macro) rejects `asset!` paths outside `CARGO_MANIFEST_DIR`, while a build script
may only write to `OUT_DIR`. So a per-feature folder generated at build time
cannot be both written (by `build.rs`) and served (by `asset!`) -- and it could
not be regenerated on a consumer's machine anyway, since a published crate's
source is read-only. We therefore commit and serve the whole superset; the
enabled `lang-*` features are currently no-ops for the shipped JS. See
<https://github.com/DioxusLabs/dioxus/issues/4426>, the issue tracking generated
asset support; once it lands, `index.js` (and the served set) can be trimmed to
the enabled features again.

### Why a single `index.js` entry

The glue imports **only** `index.js`, which re-exports the symbols the component
needs. This matters because Dioxus's asset pipeline runs esbuild over each `.js`
file and *bundles* it -- inlining that file's imports. If the glue imported
several entry files (`codemirror.js`, `codemirror__state.js`, ...) each would be
bundled separately, loading **multiple copies of `@codemirror/state`**, which
trips CodeMirror's "multiple instances of @codemirror/state" check in
`EditorState.create`. Importing one entry means esbuild produces a single module
graph with one shared `state` instance.

Consequence: at runtime only the (bundled) `index.js` is loaded; the other
per-package files in the folder are build-time inputs for esbuild to bundle
`index.js` from, and are never fetched directly by the browser.

### Refreshing / upgrading versions

The `xtask` crate fetches the modules from [esm.sh] (no npm required), rewrites
their imports, and (re)writes `index.js`:

```sh
cargo run -p xtask -- vendor
```

Pinned versions live in `xtask/src/main.rs` (`package_spec`). Note the meta
`codemirror` package must be pinned to an exact version -- esm.sh resolves
`codemirror@6` to an unrelated CodeMirror 5 lineage. After changing a version
there, re-run the command and commit the regenerated `assets/codemirror-vendor/`.

### Adding a language

To support a language not already listed, add an entry to `LANGUAGES` in
`xtask/src/main.rs`, a matching `Language` variant, and a `lang-*` feature in
`dioxus_codemirror/Cargo.toml`, then re-run the vendor command above.


[esm.sh]: https://esm.sh/
[`Language`]: dioxus_codemirror/src/language.rs
[`LspServer`]: dioxus_codemirror/src/lsp/lsp_server.rs
[`LspServerAsync`]: dioxus_codemirror/src/lsp/lsp_server_async.rs
