# Implementation Plan: Verify Harness Compatibility

**Branch**: `feat/harness-compatibility-verification` | **Date**: 2026-09-26 | **Spec**: [spec.md](spec.md)

**Input**: Clarified feature specification at `specs/044-verify-harness-compatibility/spec.md`.

## Summary

Make RGT's thirteen named agent integrations truthful and locally verifiable. Add native Codex and Windsurf registrations, gate passive value capture on a successful completed read with exact source content, repair only clearly RGT-owned obsolete registrations during normal initialization, inspect active registrations in `rgt doctor`, and maintain versioned evidence for each client surface. Preserve the existing Rust binary, graph schema, provenance attribution, fail-open hooks, and guidance tier. Use local deterministic fixtures and no-cost client smokes; never promote an unobserved client to verified automatic value capture.

## Technical Context

**Language/Version**: Rust 2021; generated client-side TypeScript/Python glue where required by host plugin contracts.

**Primary Dependencies**: Existing `clap`, `serde_json`, `jsonc-parser`, `toml_edit`, `rusqlite`, `chrono`, `blake3`; standard library for process and time bounds. No planned new runtime dependency.

**Storage**: Existing project `.rgt/store.db` for values and `capture_associations`; tracked redacted fixtures and versioned compatibility evidence files for verification. No graph or SQLite migration.

**Testing**: Existing Rust unit, contract, and integration test layout via local Cargo; fake-home installer/doctor checks; a synthetic event matrix for each advertised automatic read path; manual normal and failure-path latency measurements; optional no-cost real-client smoke. No hosted workflows or paid model calls.

**Target Platform**: RGT binary on macOS, Linux, and Windows. A host integration is verified only on the specific client version and platform actually observed; other combinations remain unverified.

**Project Type**: Single Rust CLI and native agent hook/plugin integrations.

**Performance Goals**: 64 KiB text and 1 MiB binary-envelope events, including response framing, complete within one second. Cap hook stdin at 8 MiB, bound reading even without EOF, and enforce an 800 ms invocation deadline for plugin children and direct command-hook processes so oversized or stalled events fail open within the command budget.

**Constraints**: Passive hooks do not rewrite commands, tool results, permissions, or errors. Host-specific output must satisfy the host protocol even on a no-op or when the RGT executable is missing. Source-backed roots require full result bytes and source lines matched to the current file snapshot; path-only, partial, filtered, or changed-snapshot results create no roots. Normal `rgt init` changes only proven RGT-owned entries, preserves unrelated settings, and creates backups before edits. No routine GitHub Actions or paid model calls.

**Scale/Scope**: Thirteen canonical agent identifiers; Copilot Chat/CLI and Cline/Roo are separately tested client surfaces. Each value-capture claim names a read path comprising a tool and accepted completed-result form, plus client version/platform; unverified paths and exclusions remain visible. Add native Codex and Windsurf registrations to the existing eight hook/plugin-capable agents. Cline/Roo, Antigravity, and Kilo remain guidance-only registrations. A native hook with a path-only or before-only read event remains instruction-only for value capture. No new agent names.

## Constitution Check

*Gate: checked before Phase 0 and again after Phase 1 design.*

| Principle or standard | Pre-research check | Post-design check |
|---|---|---|
| I, Rust-only binary and thin glue exemption | Pass: existing binary and client glue only | Pass: host adapters and capture decisions stay in Rust; client glue delegates. |
| II–IV, detection and graph correctness | Pass: no planned change to file-change or graph algorithms | Pass: no node identity, value, derivation, or SQLite schema change. |
| V–VI, initialized integrations and passive/active CLI | Pass: keep `rgt init`, `rgt hook`, and explicit guidance | Pass: `rgt init -g` adds inspectable Codex and Windsurf native hooks; registration health and value-capture capability remain separate, and path-only clients use instruction guidance for reads. |
| VII, open-source distribution | Pass: no new dependency planned | Pass for this feature: `.gitignore` allows `specs/044-verify-harness-compatibility/` to be tracked with implementation evidence and product docs. |
| VIII, CLI-first hook architecture | Pass: keep thin host glue | Pass: parsing and eligibility in Rust; glue only forwards fields and frames responses. |
| IX, deterministic tests | Pass: local test strategy required | Pass: fixture replay and fake-home tests are deterministic; live smoke is labeled separately and never a unit-test gate. |
| X, documentation matches behavior | Pass: support claims must be evidence-based | Pass: README, AGENTS.md, help/status text, and tracked support matrix are implementation deliverables. |
| XI, errors and quality | Pass: existing Rust idioms and CLI codes apply | Pass: hooks fail open with valid output; init conflicts get actionable diagnostics; local fmt/clippy/tests remain required before merge. |
| Latency and cross-platform standards | Pass: under-one-second requirement in spec | Pass: 8 MiB input cap, elapsed stdin deadline, 800 ms plugin-child and direct-hook process deadlines, representative 64 KiB/1 MiB events, and read-path/platform-scoped claims in contracts. |

**Gate result**: Planning may proceed locally. The constitution's PR CI gate is not satisfied by local checks, so no PR merge is in scope while the user's no-workflow instruction remains in force. This feature's `specs/044-verify-harness-compatibility/` path is eligible for tracking; the artifacts must be included in a later compliant changeset.

## Project Structure

### Documentation (this feature)

```text
specs/044-verify-harness-compatibility/
├── spec.md
├── evidence.md
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/
│   ├── hook-capture.md
│   ├── init-doctor.md
│   └── compatibility-evidence.schema.json
└── tasks.md                 # Generated by speckit-tasks, not this plan
```

### Source code and tracked evidence (implementation targets)

```text
src/
├── main.rs                  # CLI hook/init/doctor entry points
├── hooks/
│   ├── mod.rs               # Success gate, bounded passive processing
│   ├── parser.rs            # Host-specific normalization and outcome evidence
│   ├── registration.rs      # Client surfaces and local registration inspection
│   ├── installer.rs         # Agent registry and registration migration
│   ├── editor.rs            # Narrow RGT ownership matching, safe edits
│   ├── paths.rs             # Client artifact locations
│   └── glue/mod.rs          # Thin host plugin wrappers and valid responses
├── cli/
│   ├── init.rs              # Per-agent migration outcomes
│   └── doctor.rs            # Local registration inspection
└── store/                   # Reuse existing nodes and capture associations

tests/
├── unit/                    # Parser, phase/outcome, ownership, dedup
├── contract/                # CLI, host response, installer, doctor contracts
├── integration/             # Fake-home migration and graph capture
└── fixtures/hooks/          # Redacted native event examples

docs/compatibility/          # Tracked support matrix, evidence schema, and live evidence ledger
README.md
AGENTS.md
```

**Structure Decision**: Extend the existing single-binary CLI and test layout. Keep verification evidence in tracked text/JSON files rather than adding it to the provenance database. No separate service, runtime, or Docker harness is required.

## Implementation Sequence

1. Establish surface-level host contracts from vendor docs and observed native events; assign a stable read-path ID to each documented tool and accepted result form, with exclusions for other paths. Mark gaps unverified. Check Codex `PostToolUse` response fields and hook trust, and Windsurf `post_read_code` path-only payload, configuration precedence, and restricted mode. Check both successful and failed/no-op event shapes, registration discovery, and required responses. Do not adapt a client by copying RTK alone.
2. Add common Rust event normalization with explicit phase, outcome evidence, and completed content. Gate ordinary and PDF capture before any database write; reconstruct full source bytes/lines and match the current file snapshot before source-backed storage. Path-only or partial results create no roots. Bound input bytes and elapsed stdin time, including a slow stream without EOF; preserve host-valid fail-open responses even if the RGT executable is missing. Enforce the invocation deadline for plugin children and direct command hooks.
3. Repair host registration and thin plugin entrypoints per proven contract, including Codex and Windsurf native hook files. Add narrow RGT ownership matching, backups, automatic normal-init migration, idempotency, and ambiguity diagnostics. Keep unrelated configuration byte-for-byte where the format permits; surface any client trust or restricted-mode condition without overriding it.
4. Replace aggregate artifact-existence diagnosis with per-surface registration/guidance inspection. Separate local health from versioned live verification in output and docs.
5. Add deterministic event fixtures for every advertised automatic read path, graph/attribution/dedup tests, per-client migration and response contract tests, and manual end-to-end timing for 64 KiB text, 1 MiB binary-envelope, oversized, and stalled events on direct hooks and plugins. Record no-cost live smokes only for available hosts; leave all other combinations unverified and the US1 live gate pending if none can run.
6. Update tracked README, AGENTS.md, help text, and support matrix to match the measured tiers and local check procedure. Run local formatting, lint, and relevant tests before any later merge decision.

## Phase 0 Research

[research.md](research.md) records decisions, alternatives, the current comparative baseline, and the rule for unresolved external host facts. Uncertain host contracts are verification gates with an explicit instruction-only or unverified outcome.

## Phase 1 Design

- [data-model.md](data-model.md) defines client surface, registration, normalized event, evidence, and support-status lifecycle without changing the graph schema.
- [contracts/hook-capture.md](contracts/hook-capture.md) defines passive CLI and host response behavior.
- [contracts/init-doctor.md](contracts/init-doctor.md) defines migration and local diagnostic behavior.
- [contracts/compatibility-evidence.schema.json](contracts/compatibility-evidence.schema.json) defines versioned evidence records.
- [quickstart.md](quickstart.md) gives local, no-cost validation scenarios and expected outcomes.

**Post-design gate result**: Pass. The design retains the Rust-only binary and thin glue, requires deterministic local tests and documentation updates, and prevents unsupported live-compatibility claims. No complexity exception is needed.
