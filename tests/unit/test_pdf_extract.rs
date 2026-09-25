#[cfg(test)]
mod tests {
    use rgt::extract::pdf::{decode_pdf_base64, extract_pdf_text, render_additional_context};
    use rgt::hooks::parser::{normalize_agent_event, NormalizedCapture, NumberFormat};

    // -----------------------------------------------------------------------
    // 033 / envelope detection (FR-001)
    // -----------------------------------------------------------------------

    #[test]
    fn pdf_envelope_is_recognized_with_path_and_base64() {
        let payload = r#"{
            "tool_name": "Read",
            "tool_input": { "file_path": "/tmp/invoice.pdf" },
            "tool_response": {
                "type": "pdf",
                "file": {
                    "filePath": "/tmp/invoice.pdf",
                    "base64": "aGVsbG8=",
                    "originalSize": 5
                }
            }
        }"#;
        let c: NormalizedCapture = normalize_agent_event(Some("claude-code"), payload)
            .expect("pdf envelope must normalize");
        assert_eq!(c.path.as_deref(), Some("/tmp/invoice.pdf"));
        assert_eq!(c.content, None, "pdf envelopes carry no content string");
        assert_eq!(
            c.pdf_base64.as_deref(),
            Some("aGVsbG8="),
            "base64 must be surfaced for local decoding"
        );
    }

    #[test]
    fn plain_text_response_has_no_pdf_base64() {
        let payload = r#"{
            "tool_name": "Read",
            "tool_input": { "file_path": "/tmp/data.csv" },
            "tool_response": { "content": "revenue,120000\n" }
        }"#;
        let c = normalize_agent_event(Some("claude-code"), payload).expect("text payload");
        assert_eq!(c.pdf_base64, None);
        assert_eq!(c.path.as_deref(), Some("/tmp/data.csv"));
    }

    #[test]
    fn shape_mismatch_falls_through_without_base64() {
        // A future Claude Code envelope that changes the shape must degrade to
        // the existing fail-open disk-read fallback (no base64, no panic).
        let payload = r#"{
            "tool_name": "Read",
            "tool_input": { "file_path": "/tmp/x.pdf" },
            "tool_response": { "type": "weird", "blob": "abc" }
        }"#;
        let c = normalize_agent_event(Some("claude-code"), payload).expect("shape mismatch");
        assert_eq!(c.pdf_base64, None);
        assert_eq!(c.path.as_deref(), Some("/tmp/x.pdf"));
    }

    // -----------------------------------------------------------------------
    // 033 / base64 decode + extraction (FR-002, FR-005, SC-003)
    // -----------------------------------------------------------------------

    #[test]
    fn decode_valid_base64_returns_bytes() {
        assert_eq!(
            decode_pdf_base64("aGVsbG8=").as_deref(),
            Some(b"hello".as_slice())
        );
    }

    #[test]
    fn decode_malformed_base64_returns_none() {
        assert!(decode_pdf_base64("!!!not-base64!!!").is_none());
        assert!(decode_pdf_base64("a===").is_none());
    }

    #[test]
    fn extract_pdf_text_does_not_panic_on_garbage() {
        // Fuzz-style: a variety of corrupt/truncated inputs must never panic
        // (catch_unwind guard; SC-003). The call may return None (panic) or
        // Some(empty/partial) — either is a valid fail-open outcome.
        let garbage_inputs: &[&[u8]] = &[
            b"",
            b"%PDF-",
            b"%PDF-1.4\n",
            b"x\x00\x01\x02",
            b"\xff\xfe\x00\x00not a pdf",
            b"%PDF-1.4\n1 0 obj\n<<>>\nendobj\ntrailer\n<<>>\n%%EOF",
            b"%PDF-1.7\n%%EOF\n%%EOF\n",
        ];
        for garbage in garbage_inputs {
            let _ = extract_pdf_text(garbage); // must not panic
        }
    }

    #[test]
    fn extract_pdf_text_recovers_known_text_from_valid_pdf() {
        let pdf = valid_pdf_with_text("Total: 1234.56");
        let out = extract_pdf_text(&pdf).expect("valid PDF must not panic");
        assert!(
            out.contains("Total"),
            "expected the PDF text layer to be recovered: {:?}",
            out
        );
    }

    #[test]
    fn extract_pdf_text_returns_some_even_without_text_layer() {
        // A structurally valid PDF with no text operators yields Some(empty) —
        // the no-text-layer signal the hook relies on (FR-004).
        let pdf = valid_pdf_no_text();
        let out = extract_pdf_text(&pdf);
        assert!(out.is_some(), "no-text-layer PDF must not panic");
    }

    // -----------------------------------------------------------------------
    // 033 / additionalContext rendering (FR-003, FR-004)
    // -----------------------------------------------------------------------

    #[test]
    fn short_text_is_not_truncated() {
        let out = render_additional_context("Amount: 1234.56");
        assert!(out.contains("Amount: 1234.56"));
        assert!(!out.contains("truncated"));
    }

    #[test]
    fn long_text_is_truncated_with_marker() {
        let long = "x".repeat(10_000);
        let out = render_additional_context(&long);
        assert!(out.contains(
            "[... truncated: 2000 characters omitted; full text was still extracted and recorded to the provenance graph ...]"
        ));
        assert!(out.len() < long.len());
    }

    #[test]
    fn empty_text_yields_no_text_layer_notice() {
        let out = render_additional_context("");
        assert!(out.contains("no text layer"));
        assert!(out.contains("rgt record --stdin"));
    }

    #[test]
    fn number_format_auto_still_works() {
        // Guards that the new field did not disturb existing parse behavior.
        let _ = NumberFormat::Auto;
        let text = "a,1.234,56\nb,2\n";
        let result = rgt::hooks::parser::extract_values_from_content(text, NumberFormat::Auto);
        assert!(!result.values.is_empty());
    }

    /// Builds a structurally valid minimal single-page PDF with the given text
    /// in its content stream (real xref offsets so pdf-extract can parse it).
    fn valid_pdf_with_text(body: &str) -> Vec<u8> {
        build_pdf(&format!("BT /F1 12 Tf 72 720 Td ({}) Tj ET", body))
    }

    /// Builds a structurally valid minimal single-page PDF with no text stream.
    fn valid_pdf_no_text() -> Vec<u8> {
        build_pdf("q Q")
    }

    /// Assembles a minimal single-page PDF with correct object offsets.
    fn build_pdf(content_stream: &str) -> Vec<u8> {
        let mut objects: Vec<String> = Vec::new();
        objects.push(
            "<< /Type /Catalog /Pages 2 0 R >>".to_string(), // obj 1
        );
        objects.push(
            "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_string(), // obj 2
        );
        objects.push(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R /Resources << /Font << /F1 5 0 R >> >> >>".to_string(),
        ); // obj 3
        objects.push(format!(
            "<< /Length {} >>\nstream\n{}\nendstream",
            content_stream.len(),
            content_stream
        )); // obj 4
        objects.push(
            "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_string(), // obj 5
        );

        let mut out = Vec::new();
        out.extend_from_slice(b"%PDF-1.4\n");
        let mut offsets = Vec::new();
        for (i, obj) in objects.iter().enumerate() {
            offsets.push(out.len() as u64);
            out.extend_from_slice(format!("{} 0 obj\n{}\nendobj\n", i + 1, obj).as_bytes());
        }
        let xref_pos = out.len() as u64;
        out.extend_from_slice(format!("xref\n0 {}\n", objects.len() + 1).as_bytes());
        out.extend_from_slice(b"0000000000 65535 f \n");
        for off in offsets {
            out.extend_from_slice(format!("{:010} 00000 n \n", off).as_bytes());
        }
        out.extend_from_slice(
            format!(
                "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n",
                objects.len() + 1,
                xref_pos
            )
            .as_bytes(),
        );
        out
    }
}
