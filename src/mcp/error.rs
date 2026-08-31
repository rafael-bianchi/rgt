//! Error types surfaced by the MCP tool layer. `VerifyDeriveError` lives in
//! `crate::verify` (it is core logic shared with the CLI); it is re-exported
//! here so MCP code has one obvious import path.

#[allow(unused_imports)]
pub use crate::verify::VerifyDeriveError;
