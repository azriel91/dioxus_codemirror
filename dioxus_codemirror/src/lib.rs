//! A Dioxus web component that wraps the [CodeMirror 6] editor.
//!
//! [`CodeMirror`] drives a CodeMirror editor through a single long-lived
//! `document::eval` channel (the script lives in `code_mirror/glue.js`),
//! loading CodeMirror at runtime from esm.sh -- so there is no JavaScript build
//! step. Typed [`Cmd`]/[`Evt`] messages cross the channel as JSON.
//!
//! It supports:
//!
//! 1. Reacting to edits, via a two-way bound [`CodeMirrorProps::value`].
//! 2. Setting the value when the bound data changes elsewhere on the page.
//! 3. Connecting to an in-page (WASM) language server, via an [`LspBridge`].
//!
//! # Example
//!
//! ```ignore
//! use dioxus::prelude::*;
//! use dioxus_codemirror::CodeMirror;
//!
//! #[component]
//! fn App() -> Element {
//!     let value = use_signal(|| "fn main() {}".to_string());
//!     rsx! {
//!         CodeMirror { value }
//!         p { "{value}" }
//!     }
//! }
//! ```
//!
//! [CodeMirror 6]: https://codemirror.net/

pub mod cmd;
pub mod code_mirror;
pub mod evt;
pub mod language;
pub mod lsp;

pub use crate::{
    cmd::Cmd,
    code_mirror::{CodeMirror, CodeMirrorProps},
    evt::Evt,
    language::Language,
    lsp::{LspBridge, LspMessage, LspServer},
};
