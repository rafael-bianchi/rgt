# Local Validation Guide: Harness Compatibility

This guide is for the implemented feature. It uses local Rust checks and optional no-cost client observations. It does not run GitHub Actions, hosted CI, or a paid model request. A client without a no-cost native event remains unverified.

## Prerequisites

- Rust toolchain and the RGT checkout on `feat/harness-compatibility-verification`.
- For a native smoke check only: the exact client version being claimed, installed locally, with a known no-cost way to generate the relevant read event. Do not launch a paid session to satisfy this guide.
- Redacted event fixtures and support evidence that follow [hook-capture.md](contracts/hook-capture.md) and [compatibility-evidence.schema.json](contracts/compatibility-evidence.schema.json).

## 1. Run deterministic checks

From the repository root:

```bash
rtk proxy cargo fmt --all -- --check
rtk proxy cargo clippy --all-targets -- -D warnings
rtk proxy cargo test --test test_hook_parsers
rtk proxy cargo test --test test_hook_installer_contract
rtk proxy cargo test --test test_doctor_contract
rtk proxy cargo test --test test_pdf_capture_contract
rtk proxy cargo test --test test_harness_compatibility
```

The implementation must add a fixture matrix keyed by client surface and read-path ID. Every path advertised as automatic needs local success, empty, malformed, repeated, and no-op/error cases using its own host dialect; the check fails if a claimed path is missing a case. Include Codex `PostToolUse` with complete and truncated `tool_response`, Windsurf path-only `post_read_code`, partial/changed results, before-only, failed/canceled, BOM-prefixed, missing-executable, slow stdin without EOF, direct-command/plugin timeouts, and protocol no-op events. Per-client installer tests cover obsolete registrations. Expected outcomes: only successful completed reads with full reconstructed source bytes that match the current file snapshot produce source-backed nodes; repeated callbacks do not duplicate nodes or agent associations; no-op/error responses remain host-valid; unrelated client configuration survives `rgt init` and reinitialization changes nothing.

Run the local full suite before a merge decision:

```bash
rtk proxy cargo test
```

Record actual final exit codes. Partial Cargo output is not a passing suite.

## 2. Check local registration and migration

Use the fake-home integration tests to exercise current, obsolete RGT-owned, mixed, user-owned, and malformed artifacts, plus one `rgt init -g` run that detects and configures Claude Code, Cursor, Codex CLI, and Windsurf together. Check [init-doctor.md](contracts/init-doctor.md): normal `rgt init` automatically migrates only a recognized old RGT entry, retains a backup, preserves unrelated content, and stays idempotent. `rgt doctor` must report active RGT registration or instruction guidance per client surface; a bare settings file or a nonempty graph must never prove a working hook or live capture.

For a real installation, inspect the intended client configuration before running `rtk rgt init --agent <name>`. The command may edit user or project settings, so use a disposable profile or a configuration backup for a smoke run. Restart the client if its hook registration is loaded only at session start. For Codex, review project hook trust in `/hooks`; for Windsurf, ensure the workspace is not in Restricted Mode and that the selected hook file is not shadowed. Run `rtk rgt doctor` and check that it reports **local registration health**, not live verification. A Windsurf path-only post-read event can verify hook invocation but cannot verify automatic value capture.

## 3. Record a no-cost native client observation, when available

For each claimed read path (named client tool and accepted result form), client version, and platform:

1. Use a disposable project with a distinctive synthetic number and ISO date. Start a local/free client session that can produce a native successful file-read callback. If that requires a paid request or the client is unavailable, record **unverified** and stop this step for that combination.
2. Confirm the client loaded the RGT registration, invoked the hook after the read, received its required neutral response, and returned its original tool result unchanged. A pre-only callback does not count as a successful capture.
3. Inspect `rtk rgt status` and `rtk rgt query <node_id>` to confirm the expected values, source lines, and canonical capture agent. Repeat the same read and confirm no duplicate nodes or associations.
4. Generate a native no-op or failed/canceled read without cost. Confirm that the client action and error flow are unchanged and that RGT added no value nodes.
5. Redact private paths, source contents, credentials, and session identifiers from the observed payload. Record two [evidence](contracts/compatibility-evidence.schema.json) entries, one success and one no-op/error with the same `read_path_id`, client version, and OS/architecture, plus loaded registration, completion/content evidence, accepted response, unchanged tool result, number/date values with node IDs, source lines, canonical capture agent, and method `native-client`.

Promote only that exact non-`none` read-path/version/platform combination to **verified automatic value capture** after both observations pass with full-snapshot content evidence. Use `read_path_id: none` only for guidance-only or non-read observations. List other tools or result forms as excluded or unverified rather than inheriting the claim. If no client can be exercised at no cost, mark US1 live acceptance and the overall feature pending even when local tests pass. Compare tracked support documentation with the evidence ledger; a fixture replay or RTK example alone cannot promote a tier.

## 4. Check passive latency

Use deterministic hook contract tests for oversized input, a slow stream without EOF, injected direct-command/plugin timeouts, and missing-executable fail-open behavior. Build RGT, then run the local manual benchmark with the resulting executable path:

```bash
rtk proxy cargo build --bin rgt
rtk proxy cargo bench --bench bench_hook_capture -- "$PWD/target/debug/rgt"
```

Measure invocation-to-neutral-response time, including launch and response framing, for 64 KiB text and 1 MiB binary-envelope successful, no-op, malformed, and repeated events. Also time an input over 8 MiB, a slow stream that never reaches EOF, and a stalled child through both a direct command hook and a plugin. Each normal and fail-open response must finish under one second; timeout cases must create no values. Record the machine, input sizes, per-case measured times, and the 800 ms process cutoff in `docs/compatibility/verification.md`.
