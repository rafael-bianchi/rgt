//! Local extraction of values from binary/special formats (spec 033).

use base64::prelude::*;

/// Maximum characters injected into the model's context via
/// `PostToolUse` `additionalContext` (spec 033 §3.4). A centralized constant
/// today; a future setting can surface it without changing call sites.
pub const MAX_CONTEXT_CHARS: usize = 8000;

/// Decodes a base64 PDF envelope's bytes, or `None` on malformed input
/// (fail open — spec 033 FR-005).
pub fn decode_pdf_base64(base64_str: &str) -> Option<Vec<u8>> {
    BASE64_STANDARD.decode(base64_str.trim()).ok()
}

/// Runs local PDF text extraction on in-memory bytes.
///
/// Returns `Some(text)` even when the PDF has no text layer (scanned/image-only
/// PDFs yield an empty string, not an error); returns `None` only when the
/// extraction panics — the third-party call's behavior on corrupt input is
/// undocumented, so it is wrapped in `catch_unwind` to preserve the hook's
/// fail-open contract (spec 033 Risks; Constitution XI "zero unhandled panics").
pub fn extract_pdf_text(decoded: &[u8]) -> Option<String> {
    std::panic::catch_unwind(|| pdf_extract::extract_text_from_mem(decoded))
        .ok()
        .and_then(|r| r.ok())
}

/// Renders the `additionalContext` string for a PDF capture.
///
/// Non-empty text is truncated to [`MAX_CONTEXT_CHARS`] with an explicit marker
/// naming the omitted characters (the graph always holds the full recorded
/// values); empty text (no text layer) becomes a notice directing the agent to
/// `rgt record --stdin` instead of the previous silent, indistinguishable
/// failure (spec 033 FR-003/FR-004).
pub fn render_additional_context(text: &str) -> String {
    if text.trim().is_empty() {
        return "RGT: no text layer found in this PDF; provenance capture requires the agent to extract values manually via rgt record --stdin".to_string();
    }
    if text.chars().count() <= MAX_CONTEXT_CHARS {
        return text.to_string();
    }
    let truncated: String = text.chars().take(MAX_CONTEXT_CHARS).collect();
    let omitted = text.chars().count() - MAX_CONTEXT_CHARS;
    format!(
        "{}\n[... truncated: {omitted} characters omitted; full text was still extracted and recorded to the provenance graph ...]",
        truncated
    )
}
