#[cfg(test)]
mod tests {
    use rgt::hooks::parser::normalize_agent_event;
    use rgt::hooks::parser::NumberFormat;
    use std::io::Write;
    use std::sync::Mutex;
    use tempfile::tempdir;

    static CWD_MUTEX: Mutex<()> = Mutex::new(());

    fn set_cwd(dir: &std::path::Path) -> std::sync::MutexGuard<'static, ()> {
        let guard = CWD_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        std::env::set_current_dir(dir).unwrap();
        guard
    }

    fn rgt() -> &'static str {
        env!("CARGO_BIN_EXE_rgt")
    }

    /// Builds a structurally valid minimal PDF with the given text.
    fn valid_pdf(body: &str) -> Vec<u8> {
        let content = format!("BT /F1 12 Tf 72 720 Td ({}) Tj ET", body);
        build_pdf(&content)
    }

    fn build_pdf(content_stream: &str) -> Vec<u8> {
        let mut objects: Vec<String> = Vec::new();
        objects.push("<< /Type /Catalog /Pages 2 0 R >>".to_string());
        objects.push("<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_string());
        objects.push(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R /Resources << /Font << /F1 5 0 R >> >> >>".to_string(),
        );
        objects.push(format!(
            "<< /Length {} >>\nstream\n{}\nendstream",
            content_stream.len(),
            content_stream
        ));
        objects.push("<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_string());

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

    fn base64_of(bytes: &[u8]) -> String {
        use base64::prelude::*;
        BASE64_STANDARD.encode(bytes)
    }

    fn envelope(file_path: &str, base64: &str) -> String {
        format!(
            r#"{{"tool_name":"Read","tool_input":{{"file_path":"{file_path}"}},"tool_response":{{"type":"pdf","file":{{"filePath":"{file_path}","base64":"{base64}","originalSize":100}}}}}}"#
        )
    }

    // -----------------------------------------------------------------------
    // 033 / US1 (SC-001): a real PDF envelope records values end-to-end
    // -----------------------------------------------------------------------

    #[test]
    fn pdf_envelope_records_values_into_the_graph() {
        let dir = tempdir().unwrap();
        let _guard = set_cwd(dir.path());
        assert!(std::process::Command::new(rgt())
            .args(["init", "--agent", "codex"])
            .current_dir(dir.path())
            .output()
            .expect("spawn rgt init")
            .status
            .success());

        // A real PDF whose text layer carries a known numeric value.
        let pdf = valid_pdf("Invoice Total: 1234.56");
        let pdf_path = dir.path().join("invoice.pdf");
        std::fs::write(&pdf_path, &pdf).unwrap();

        // The exact captured envelope shape (spec 033 §2.1): type pdf + file
        // {filePath, base64, originalSize}, no content string.
        let stdin = envelope(&pdf_path.to_string_lossy(), &base64_of(&pdf));

        let mut child = std::process::Command::new(rgt())
            .args(["hook", "post", "--agent", "claude-code"])
            .current_dir(dir.path())
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .spawn()
            .expect("spawn rgt hook post");
        child
            .stdin
            .take()
            .unwrap()
            .write_all(stdin.as_bytes())
            .unwrap();
        let output = child.wait_with_output().expect("wait for hook post");
        assert!(
            output.status.success(),
            "hook must exit 0 (fail open), stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        // SC-001: the PDF's value is recorded as a tracked node.
        let db = rgt::store::DbStore::open_in_project(".").expect("open store");
        let conn = db.conn();
        let all = rgt::store::queries::list_all_nodes(conn).expect("list nodes");
        assert!(
            all.iter().any(|n| n.value.to_string_repr() == "1234.56"),
            "expected the PDF's value 1234.56 to be recorded, got: {:?}",
            all.iter()
                .map(|n| n.value.to_string_repr())
                .collect::<Vec<_>>()
        );
        let captured = all
            .iter()
            .find(|node| node.value.to_string_repr() == "1234.56")
            .unwrap();
        let associations = rgt::store::queries::list_capture_associations(conn).unwrap();
        assert!(associations.contains(&(captured.id.clone(), "claude-code".into())));
    }

    #[test]
    fn pdf_envelope_emits_additional_context_for_claude_code() {
        let dir = tempdir().unwrap();
        let _guard = set_cwd(dir.path());
        assert!(std::process::Command::new(rgt())
            .args(["init", "--agent", "codex"])
            .current_dir(dir.path())
            .output()
            .expect("spawn rgt init")
            .status
            .success());

        let pdf = valid_pdf("Amount due: 99.50");
        let pdf_path = dir.path().join("due.pdf");
        std::fs::write(&pdf_path, &pdf).unwrap();
        let stdin = envelope(&pdf_path.to_string_lossy(), &base64_of(&pdf));

        let mut child = std::process::Command::new(rgt())
            .args(["hook", "post", "--agent", "claude-code"])
            .current_dir(dir.path())
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .expect("spawn rgt hook post");
        child
            .stdin
            .take()
            .unwrap()
            .write_all(stdin.as_bytes())
            .unwrap();
        let output = child.wait_with_output().expect("wait");

        let stdout = String::from_utf8_lossy(&output.stdout);
        let json: serde_json::Value =
            serde_json::from_str(stdout.trim()).expect("stdout must be JSON");
        let context = json["hookSpecificOutput"]["additionalContext"]
            .as_str()
            .expect("additionalContext must be a string");
        assert!(
            context.contains("99.50"),
            "additionalContext must carry the extracted text: {context}"
        );
    }

    #[test]
    fn pdf_envelope_emits_no_stdout_for_other_agents() {
        let dir = tempdir().unwrap();
        let _guard = set_cwd(dir.path());
        assert!(std::process::Command::new(rgt())
            .args(["init", "--agent", "codex"])
            .current_dir(dir.path())
            .output()
            .expect("spawn rgt init")
            .status
            .success());

        let pdf = valid_pdf("Amount due: 99.50");
        let pdf_path = dir.path().join("due.pdf");
        std::fs::write(&pdf_path, &pdf).unwrap();
        let stdin = envelope(&pdf_path.to_string_lossy(), &base64_of(&pdf));

        let mut child = std::process::Command::new(rgt())
            .args(["hook", "post", "--agent", "opencode"])
            .current_dir(dir.path())
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .expect("spawn rgt hook post");
        child
            .stdin
            .take()
            .unwrap()
            .write_all(stdin.as_bytes())
            .unwrap();
        let output = child.wait_with_output().expect("wait");

        assert!(
            output.status.success(),
            "hook must fail open: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(
            output.stdout.is_empty(),
            "non-claude-code agents must not emit stdout, got: {}",
            String::from_utf8_lossy(&output.stdout)
        );
    }

    #[test]
    fn malformed_base64_envelope_fails_open() {
        let dir = tempdir().unwrap();
        let _guard = set_cwd(dir.path());
        assert!(std::process::Command::new(rgt())
            .args(["init", "--agent", "codex"])
            .current_dir(dir.path())
            .output()
            .expect("spawn rgt init")
            .status
            .success());

        let stdin = envelope("/tmp/x.pdf", "!!!not-base64!!!");

        let mut child = std::process::Command::new(rgt())
            .args(["hook", "post", "--agent", "claude-code"])
            .current_dir(dir.path())
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .expect("spawn rgt hook post");
        child
            .stdin
            .take()
            .unwrap()
            .write_all(stdin.as_bytes())
            .unwrap();
        let output = child.wait_with_output().expect("wait");

        assert!(
            output.status.success(),
            "malformed base64 must fail open (exit 0): {}",
            String::from_utf8_lossy(&output.stderr)
        );

        // No nodes recorded from a malformed envelope.
        let db = rgt::store::DbStore::open_in_project(".").expect("open store");
        let conn = db.conn();
        let all = rgt::store::queries::list_all_nodes(conn).expect("list nodes");
        assert!(all.is_empty(), "no nodes may be recorded: {all:?}");
    }

    // -----------------------------------------------------------------------
    // sanity: the envelope normalizer still works for text payloads
    // -----------------------------------------------------------------------

    #[test]
    fn text_payload_normalizes_without_pdf_field() {
        let payload = r#"{"tool_name":"Read","tool_input":{"file_path":"/tmp/d.csv"},"tool_response":{"content":"a,1\n"}}"#;
        let c = normalize_agent_event(Some("claude-code"), payload).expect("text payload");
        assert_eq!(c.pdf_base64, None);
        assert_eq!(c.path.as_deref(), Some("/tmp/d.csv"));
        let _ = NumberFormat::Auto;
    }
}
