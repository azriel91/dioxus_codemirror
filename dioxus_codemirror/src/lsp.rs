//! Plumbing to connect the [`CodeMirror`] editor to a language server.
//!
//! [`CodeMirror`]: crate::code_mirror::CodeMirror

pub mod lsp_bridge;
pub mod lsp_message;
pub mod lsp_server;

pub use self::{
    lsp_bridge::LspBridge, lsp_message::LspMessage, lsp_server::LspServer,
};
