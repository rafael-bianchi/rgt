# Quickstart: Automatic PDF Provenance Capture via `additionalContext`

**Feature**: [spec.md](./spec.md) | **Date**: 2026-08-31

Runnable end-to-end validation. See [contracts/pdf-capture-contract.md](./contracts/pdf-capture-contract.md) for the authoritative invariants.

## Prerequisites

- Built binary: `cargo build`
- A scratch project dir:

```bash
SCRATCH=$(mktemp -d) && cd "$SCRATCH"
BIN=/Users/rafaelbianchi/dev/repos/rgt/target/debug/rgt
```

## Scenario 1 — a real PDF envelope records values automatically

Create a small PDF containing a known value (e.g. `Total: 1234.56`), base64 it, and feed a Claude-Code-shaped envelope:

```bash
"$BIN" init --agent codex >/dev/null 2>&1
BASE64=$(base64 < /tmp/sample.pdf)
printf '{"tool_name":"Read","tool_input":{"file_path":"/tmp/sample.pdf"},"tool_response":{"type":"pdf","file":{"filePath":"/tmp/sample.pdf","base64":"%s","originalSize":100}}}' "$BASE64" \
  | "$BIN" hook post --agent claude-code
"$BIN" status
```

Expected: `status` shows the PDF's values as tracked nodes; the hook stdout (if captured) contains `hookSpecificOutput.additionalContext` with the extracted text.

## Scenario 2 — malformed base64 fails open

```bash
printf '{"tool_name":"Read","tool_input":{"file_path":"/tmp/x.pdf"},"tool_response":{"type":"pdf","file":{"filePath":"/tmp/x.pdf","base64":"!!!not-base64!!!","originalSize":9}}}' \
  | "$BIN" hook post --agent claude-code; echo "exit=$?"
```

Expected: exit 0, no crash, no nodes recorded.

## Scenario 3 — no-text-layer PDF emits a notice

```bash
# feed an envelope whose base64 decodes to a PDF with no text layer
```

Expected: `additionalContext` is the `rgt record --stdin` notice; no nodes recorded.

## Automated checks

```bash
cargo test --test test_pdf_extract
cargo test --test test_pdf_capture_contract
cargo fmt --all -- --check
cargo clippy --all-targets
cargo test
```

Cleanup: `rm -rf "$SCRATCH"`.
