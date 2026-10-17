# Tasks: Fix Query JSON Flag Behavior

**Input**: Design documents from `/specs/040-fix-query-json-flag-behavior/`

**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/query-output-contract.md, quickstart.md

**Tests**: Included — the spec requires contract coverage for both output modes (Constitution IX).

**Organization**: Single user story; small, self-contained change.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (US1)
- Exact file paths in descriptions

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Confirm baseline and register the test target.

- [ ] T001 Verify baseline: run `cargo test` and confirm green; confirm `rgt query <id>` currently prints JSON in both modes (reproduce the bug) and that nothing in `src/` or `tests/` greps for a `"lineage"` wrapper key
- [ ] T002 Register new test target `test_query_contract` (path `tests/contract/test_query_contract.rs`) in `Cargo.toml`

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: The text renderer + JSON cleanup.

**⚠️ CRITICAL**: No user-story work can begin until this phase is complete.

- [ ] T003 Rewrite `src/cli/query.rs`: add pure `fn render_lineage_text(result: &serde_json::Value) -> String` implementing the format in `research.md` §3 (header, node summary with `ROOT|DERIVED` type, per-step value/kind/stale-reason/line/parents, `parents: none` for roots, `n/a` for missing line, `total ancestors: N`); make `--json` print `serde_json::to_string_pretty(&result)` directly with NO `{"lineage": ...}` wrapper; keep `execute_query` as a thin entry calling `query_provenance(node_id)?` before rendering

**Checkpoint**: `execute_query` compiles; both branches produce different output (text vs JSON).

---

## Phase 3: User Story 1 - Human-Readable Lineage by Default (Priority: P1) 🎯 MVP

**Goal**: Default output is human-readable text; `--json` is clean JSON.

**Independent Test**: `cargo test --test test_query_contract` — asserts text-vs-JSON behavior.

### Tests for User Story 1

> **NOTE: Write these tests FIRST, ensure they FAIL before implementation**

- [ ] T004 [US1] Contract test in `tests/contract/test_query_contract.rs`: build an in-memory store, record a root value, derive a child, capture `execute_query` output in both modes; assert (a) no-flag output has no leading `{` and no `"lineage"` key, (b) `--json` output parses via `serde_json` with `node_id`/`lineage_steps` and no wrapper key, (c) both modes carry the same steps in the same order, (d) unknown node errors in both modes (exit non-zero)

### Implementation for User Story 1

- [ ] T005 [US1] Expose/refactor the rendering seam needed by T004 (e.g. `execute_query` keeps its signature; the test can call `render_lineage_text` + the JSON serializer directly for pure-assertion cases, and `execute_query` for the error case)

**Checkpoint**: `cargo test --test test_query_contract` green; `cargo test` full suite green.

---

## Phase 4: Polish & Cross-Cutting Concerns

**Purpose**: Final validation.

- [ ] T006 Run `cargo fmt --all -- --check`, `cargo clippy --all-targets`, and full `cargo test`; confirm README's `rgt query` line and `--help` text are accurate with no edits needed

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies.
- **Foundational (Phase 2)**: Depends on Setup; BLOCKS US1.
- **User Story 1 (Phase 3)**: Depends on Foundational (T003).
- **Polish (Phase 4)**: Depends on US1.

### User Story Dependencies

- **US1 (P1)**: single story.

### Within User Story 1

- Tests written first (T004), then implementation seam (T005).

### Parallel Opportunities

- None meaningful — all changes touch `src/cli/query.rs` and the new test file.

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Phase 1: Setup (T001–T002)
2. Phase 2: Foundational (T003)
3. Phase 3: US1 (T004–T005)
4. **STOP and VALIDATE**: `cargo test --test test_query_contract` green

### Incremental Delivery

1. Foundational → renderer + JSON cleanup
2. US1 → contract coverage for both modes + errors
3. Polish → fmt/clippy/full test + doc-accuracy check

---

## Notes

- The lineage *data* (`src/query/mod.rs`) is intentionally untouched — only presentation in `src/cli/query.rs` changes.
- The `{"lineage": ...}` wrapper is removed; nothing depends on it (verified in T001).
- Human-friendly strings (`ROOT`/`DERIVED`, kind display) come from existing display impls, not `Debug`.
