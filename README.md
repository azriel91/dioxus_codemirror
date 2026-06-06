# 📝 dioxus_codemirror

[![Crates.io](https://img.shields.io/crates/v/dioxus_codemirror.svg)](https://crates.io/crates/dioxus_codemirror)
[![docs.rs](https://img.shields.io/docsrs/dioxus_codemirror)](https://docs.rs/dioxus_codemirror)
[![CI](https://github.com/azriel91/dioxus_codemirror/workflows/CI/badge.svg)](https://github.com/azriel91/dioxus_codemirror/actions/workflows/ci.yml)

A Dioxus **web** component that wraps the [CodeMirror 6] editor, for use in
Dioxus web applications.

Demo: <https://azriel.im/dioxus_codemirror>.

> [!NOTE]
>
> 🚧 This crate is new and not yet stable; its API may change between releases.

No JavaScript build step is required: the component drives CodeMirror through a
single long-lived `document::eval` channel. CodeMirror itself is **vendored** as
a Dioxus folder asset, so there is no runtime CDN dependency.


## Features

<details open>

* [x] Pure Rust build / no `node` dependency.
* [x] Set / receive text via `Signal<String>`.
* [x] Syntax highlighting per-language, feature-gated so you ship only what
  you use.
* [x] Light / dark / auto theme (auto follows `prefers-color-scheme`).
* [x] In-page LSP bridge -- synchronous, or async / worker-backed.

</details>


## Usage

Add the following to `Cargo.toml`:

```toml
[dependencies]
dioxus_codemirror = "0.1.0"

# Select which language syntax highlighting you want to bundle. See
# "Choosing languages" below. `lang-yaml` and `lang-markdown` shown here.
dioxus_codemirror = { version = "0.1.0", features = ["lang-yaml", "lang-markdown"] }
```

In code:

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
* `language: Language` -- syntax highlighting (default plain text). The variant
  must have its `lang-*` Cargo feature enabled to be bundled; see
  [Choosing languages](#choosing-languages). Selecting a `Language` whose feature
  is disabled falls back to plain text (with a console warning) rather than
  failing.
* editor feature toggles (e.g. `allow_multiple_selections: bool`) -- optional
  CodeMirror features, off by default; see [Editor features](#editor-features).
* `theme: Theme` -- color theme, `Theme::Auto` (default, follows the OS
  `prefers-color-scheme`), `Theme::Light`, or `Theme::Dark`.
* `lsp: LspBridge` -- connect an in-page language server, synchronous or async
  (optional); see [LSP](#lsp).


## Editor features

Optional CodeMirror behaviours are individual props, each named after the
CodeMirror extension it enables (API parity). All are off by default; set the
ones you want:

```rust
use dioxus_codemirror::CodeMirror;

rsx! {
    CodeMirror {
        value,
        allow_multiple_selections: true,   // Alt-click, Ctrl/Cmd-d / Ctrl/Cmd-F2 cursors
        highlight_selection_matches: true, // highlight other occurrences
        bracket_matching: true,
        close_brackets: true,
        line_wrapping: true,
    }
}
```

| Prop | CodeMirror extension |
| --- | --- |
| `allow_multiple_selections` | Allows multiple selections in the editor. Also binds `Ctrl/Cmd-d` to select the next match and `Ctrl/Cmd-F2` to select all matches. |
| `bracket_matching` | Highlight the bracket matching the one next to the cursor. |
| `close_brackets` | Auto-insert closing brackets and quotes. |
| `highlight_active_line` | Highlight the line the primary cursor is on. |
| `highlight_selection_matches` | Highlight every occurrence of the selected text, the selection included. |
| `highlight_whitespace` | Render whitespace characters visibly. |
| `indent_on_input` | Re-indent lines as you type. |
| `indent_with_tab` | Bind `Tab`/`Shift-Tab` to indent, so `Tab` inserts indentation rather than moving focus out of the editor. |
| `language` | Syntax highlighting language, e.g. `Language::Yaml`. Defaults to plain text (`None`). |
| `line_numbers` | Show a line-number gutter. |
| `line_wrapping` | Wrap long lines instead of scrolling horizontally. |
| `read_only` | Make the document read-only, |
| `rectangular_selection` | Allow rectangular (block) selection via `Alt`-drag. |
| `tab_size` | Width of a tab in spaces. |
| `theme` | Color theme, e.g. `Theme::Dark`. Defaults to `Theme::Auto`. |


## Choosing languages

Each syntax-highlighting language is gated behind a `lang-*` Cargo feature, so a
consumer ships only the languages they use -- the files for disabled languages
are never copied into the build output. Enable the ones you need:

```toml
[dependencies]
dioxus_codemirror = { version = "0.1.0", features = [
    "lang-css",
    "lang-html",
] }

# Or bundle everything:
dioxus_codemirror = { version = "0.1.0", features = ["lang-all"] }
```

Available features: `lang-yaml`, `lang-markdown`, `lang-javascript`, `lang-css`,
`lang-html`, and `lang-all`. Each matches a [`Language`] variant. No language is
bundled by default; selecting a `Language` whose feature is disabled falls back
to plain text (with a console warning) rather than failing.

Selection is a build-time, crate-wide choice. To add a language not listed
above, see [`DEVELOPMENT.md`](DEVELOPMENT.md).


## LSP

`CodeMirror` takes an optional `LspBridge`, which connects the editor's
`@codemirror/lsp-client` to an in-page language server. There are two transport
flavours, chosen by which constructor you call:

* **Synchronous** -- `LspBridge::lsp_bridge_from_server` drives an [`LspServer`],
  the extension point a real in-page language server implements. It is
  request/response: each message from the editor is handed to the server, and
  the messages it *returns* are forwarded straight back.

* **Async / worker-capable** -- `LspBridge::lsp_bridge_from_server_async` drives
  an [`LspServerAsync`], for a server running in a **Web Worker** (or off the
  main thread), or one that emits **diagnostics** after a processing step.
  Instead of returning replies, the server pushes messages onto an `LspPusher`
  whenever they are ready -- which is what makes **server-initiated, unprompted
  messages** (e.g. `textDocument/publishDiagnostics`) work.

The `example` ships mock servers for both flavours. Replace either mock with your
WASM language server to get genuine language features. See
[`DEVELOPMENT.md`](DEVELOPMENT.md) for the wire protocol and architecture.


## Development

See [`DEVELOPMENT.md`](DEVELOPMENT.md) for the architecture, the message
protocol, the vendored CodeMirror assets, and how to add a language.

To run the bundled example:

```sh
dx serve --platform web -p example
```


## License

Licensed under either of

* Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or https://www.apache.org/licenses/LICENSE-2.0)
* MIT license ([LICENSE-MIT](LICENSE-MIT) or https://opensource.org/licenses/MIT)

at your option.


### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.

[CodeMirror 6]: https://codemirror.net/
[`Language`]: dioxus_codemirror/src/language.rs
[`LspServer`]: dioxus_codemirror/src/lsp/lsp_server.rs
[`LspServerAsync`]: dioxus_codemirror/src/lsp/lsp_server_async.rs
