# Research: Agent Instructions Feature Parity

**Feature**: [spec.md](./spec.md) | **Date**: 2026-08-31

Decisions grounded in `src/hooks/glue/mod.rs` (5 constants), `src/hooks/installer.rs` (`codex_section()` at :470-473, Windsurf base at :496, `with_instruction` at :243), the repo's canonical `AGENTS.md` (already current), `CONTRIBUTING.md`, and the existing tests (`tests/contract/hook_installer.rs`, `tests/unit/hook_editors.rs`).

## 1. Current state audit (FR-001, FR-002)

**Finding**: The repo's `AGENTS.md` teaches the full set (`record --number-format`, `derive`, `status`, `query`, `graph`, `gc`, `doctor`). The installed glue artifacts lag:

| Artifact | Current feature mentions | Missing |
|---|---|---|
| `CLINE_RULES` (glue/mod.rs:117) | record, status, derive, query, graph | doctor, gc, --number-format |
| `ANTIGRAVITY_RULES` (:138) | same | same |
| `KILO_RULES` (:159) | same | same |
| `VIBE_PROMPT` (:180) | record, derive, status | doctor, gc, --number-format, query/graph |
| `COPILOT_CLI_RULES` (:192) | record, status, derive, query, graph | doctor, gc, --number-format |
| `codex_section()` (installer.rs:471) | record, status, derive, query, graph | doctor, gc, --number-format |
| Windsurf base (installer.rs:496) | record, derive, status | doctor, gc, --number-format, query/graph |

All share the same root cause: frozen at the pre-hardening feature set.

## 2. Canonical content model (FR-001)

**Decision**: The added text follows the canonical `AGENTS.md` structure but preserves each artifact's heading style:
- Structured artifacts (Cline/Copilot/Antigravity/Kilo/codex — `###` or `##` style): add `rgt record <file> --number-format <us|eu|auto>` under "Recording Values" (or equivalent), and add two bullets under "Inspecting the Graph": `rgt gc [--vacuum]` and `rgt doctor`.
- Compact artifacts (Vibe/Windsurf — prose style): add a `--number-format` mention in the record line and sentences mentioning `rgt doctor` and `rgt gc`.

**Rationale**: Consistent with AGENTS.md's proven wording (already verified correct) while honoring each artifact's established format; avoids inventing new prose.

**Alternatives considered**: Rewriting all artifacts to be byte-identical to AGENTS.md — rejected: would destroy each agent's format-specific heading structure and is not required for parity.

## 3. Wrapper preservation (FR-003)

**Finding**: `glue::with_instruction(rules)` appends `NON_TEXT_SOURCE_INSTRUCTION` + `RGT_END_MARKER_LINE` (glue/mod.rs:243-248). The constants themselves do not include the marker; the wrapper does.

**Decision**: Only the body strings change; the `with_instruction` call sites are untouched. This keeps the non-text-source instruction and the delimited-block semantics (specs 019/028) intact.

## 4. CONTRIBUTING.md (FR-004)

**Decision**: Update the `src/cli/` tree listing to include `doctor.rs` and `gc.rs` (and add `query.rs` which the current listing omits — verify); update the "Running Tests" examples to reference the real current test files (`hook_editors.rs`, `store_integrity.rs`, `test_update_attestation.rs`, `test_update_semver.rs`, `test_doctor_contract.rs`, `test_store_maintenance_contract.rs`, plus the others that exist).

## 5. Test strategy (SC-001, SC-003, FR-005)

**Decision**: Add assertions to `tests/contract/hook_installer.rs` (the existing installer contract file) that run the affected agents and assert the written artifacts contain `doctor`, `gc`, and `number-format`. Extend the existing tests rather than create a new file (the harness is already there). Keep existing positive assertions untouched.

**Rationale**: Constitution IX — the new content must be covered; the existing harness (`set_cwd`/`run`/`read`) makes this cheap. Full suite must stay green (SC-003).

## 6. Rebuild/re-init note (Assumptions)

**Decision**: No code-behavior change; users must rebuild and re-run `rgt init` (or `--force`) to pick up corrected text. This is consistent with the idempotent-write behavior already established; the plan/quickstart note it.
