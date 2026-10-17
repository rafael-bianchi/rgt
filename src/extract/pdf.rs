//! Local extraction of values from binary/special formats (spec 033).

use base64::prelude::*;

/// Maximum characters injected into the model's context via
/// `PostToolUse` `additionalContext` (spec 033 §3.4). A centralized constant
/// today; a future setting can surface it without changing call sites.
pub const MAX_CONTEXT_CHARS: usize = 8000;

/// Maximum allowed decoded PDF size or decompressed text output (10 MiB limit).
pub const MAX_PDF_BYTES: usize = 10 * 1024 * 1024;

/// Decodes a base64 PDF envelope's bytes, or `None` on malformed input
/// (fail open — spec 033 FR-005). Enforces a 10 MiB limit on decoded bytes.
pub fn decode_pdf_base64(base64_str: &str) -> Option<Vec<u8>> {
    let trimmed = base64_str.trim();
    if trimmed.len() > MAX_PDF_BYTES * 4 / 3 + 1024 {
        return None;
    }
    let bytes = BASE64_STANDARD.decode(trimmed).ok()?;
    if bytes.len() > MAX_PDF_BYTES {
        return None;
    }
    Some(bytes)
}

/// Runs local PDF text extraction on in-memory bytes.
///
/// Returns `Some(text)` even when the PDF has no text layer (scanned/image-only
/// PDFs yield an empty string, not an error); returns `None` only when the
/// extraction panics — the third-party call's behavior on corrupt input is
/// undocumented, so it is wrapped in `catch_unwind` to preserve the hook's
/// fail-open contract (spec 033 Risks; Constitution XI "zero unhandled panics").
pub fn extract_pdf_text(decoded: &[u8]) -> Option<String> {
    if decoded.len() > MAX_PDF_BYTES {
        return None;
    }
    std::panic::catch_unwind(|| pdf_extract::extract_text_from_mem(decoded))
        .ok()
        .and_then(|r| r.ok())
        .map(|text| {
            if text.len() > MAX_PDF_BYTES {
                let split = text
                    .char_indices()
                    .take_while(|(idx, _)| *idx <= MAX_PDF_BYTES)
                    .last()
                    .map(|(idx, c)| idx + c.len_utf8())
                    .unwrap_or(0);
                text[..split].to_string()
            } else {
                text
            }
        })
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
    let total_chars = text.chars().count();
    if total_chars <= MAX_CONTEXT_CHARS {
        return text.to_string();
    }
    let split_byte = text
        .char_indices()
        .nth(MAX_CONTEXT_CHARS)
        .map(|(idx, _)| idx)
        .unwrap_or(text.len());
    let truncated = &text[..split_byte];
    let omitted = total_chars - MAX_CONTEXT_CHARS;
    format!(
        "{}\n[... truncated: {omitted} characters omitted; full text was still extracted and recorded to the provenance graph ...]",
        truncated
    )
}
