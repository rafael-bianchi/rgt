# Tasks: Safe Duration Calculations and Unit Display

**Input**: Design documents from `specs/043-date-diff-units/`

**Prerequisites**: [plan.md](plan.md), [spec.md](spec.md), [research.md](research.md), [data-model.md](data-model.md), [CLI contract](contracts/duration-cli.md), [graph contract](contracts/duration-graph.md)

**Tests**: Required by RGT Constitution IX for every user-facing behavior. Write each story's deterministic tests first and confirm the intended failure locally before implementing it. Do not run CI/CD or GitHub workflows under the current user instruction.

**Organization**: Tasks are grouped by the four prioritized user stories. The existing Rust CLI, SQLite tables, and test layout are reused.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel with adjacent marked tasks after their shared prerequisites are complete because they touch different files.
- **[Story]**: [US1] through [US4] map to the stories in [spec.md](spec.md).
- Paths are relative to the repository root.

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Prepare a feature branch and deterministic test entry points without changing runtime behavior.

- [X] T001 Create `feat/043-safe-duration-units` from `develop` after preserving unrelated working-tree files; keep the active feature reference in `.specify/feature.json` pointing to `specs/043-date-diff-units`.
- [X] T002 Register the new duration unit, identity, CLI, integration, and performance test files under `[[test]]` in `Cargo.toml`, following the existing nested `tests/unit/`, `tests/contract/`, and `tests/integration/` convention. Name the performance target `test_duration_performance`; its timing test must be ignored by default as specified in T032.

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Make exact units, canonical identity, and strict temporal reads available to every story.

**Critical**: Finish this phase before a user story implementation; these rules are shared safety controls.

- [X] T003 [P] Write failing tests for `DurationUnit` names exactly `seconds`, `minutes`, `hours`, `days`, `weeks`, scales 1/60/3600/86400/604800, exact decimal claims, signed values, overflow, and exact/approximate display in `tests/unit/test_duration_unit.rs`.
- [X] T004 [P] Write failing tests for a versioned Duration ID containing exact signed seconds, kind, operation, and length-delimited parents; ordered `DATE_DIFF` versus sorted distinct sum/average parents; and display-string collision resistance in `tests/unit/test_duration_identity.rs`.
- [X] T005 [P] Add failing corrupt-row tests for missing/malformed Date and Duration columns so no temporal read substitutes current UTC or zero in `tests/unit/store_integrity.rs`.
- [X] T006 Implement `DurationUnit`, exact lexical `DurationClaim`, fixed scales, signed whole-second validation, lossless seconds text, and exact/marked-approximate selected-unit display in `src/types/duration_unit.rs`; expose the new module from `src/types/mod.rs` (after T003).
- [X] T007 Implement the canonical Duration-derived BLAKE3 ID preimage and parent canonicalization in `src/types/node.rs`: preserve Date order, sort and reject repeated aggregate parent IDs, retain `node_drv_<32 hex>` output, and leave root/Number IDs unchanged (after T004).
- [X] T008 Replace silent malformed Date/Duration fallbacks with actionable strict typed-row errors for temporal consumers in `src/store/queries.rs`, preserving valid historical rows and checking a representable Duration range (after T005).

**Checkpoint**: Unit conversion, identity, and typed reads are deterministic and locally testable without adding a duration CLI operation.

---

## Phase 3: User Story 1 - Derive a date gap without calculating it first (Priority: P1) 🎯 MVP

**Goal**: `derive DATE_DIFF` computes a whole-second Duration, optionally checks a claim, and reports a chosen unit while preserving the old no-unit command result.

**Independent Test**: Two Date parents 45 days apart produce one linked Duration of 3,888,000 seconds with `--unit days` and no `--result`; an equivalent 45-day assertion reuses its ID; wrong, reversed, or subsecond inputs write nothing.

### Tests for User Story 1

- [X] T009 [P] [US1] Write failing CLI contract tests for automatic `DATE_DIFF`, `--result 45 --result-unit days`, wrong assertions, no-unit stdout compatibility, and exit/no-write behavior in `tests/contract/test_duration_cli_contract.rs`.
- [X] T010 [P] [US1] Add failing full-precision tests for zero, 45-day, negative subsecond, positive subsecond, and reversed Date gaps in `tests/unit/test_date_diff.rs`.
- [X] T011 [P] [US1] Write failing graph integration tests for one Duration with two Date edges, equivalent legacy 12/32-character ID reuse, conflicting legacy candidate protection, and new-ID Turtle activity reconstruction in `tests/integration/test_duration_date_diff.rs`.

### Implementation for User Story 1

- [X] T012 [US1] Make `DATE_DIFF` compute from exactly two ordered Date parents, reject a negative or fractional-second actual gap before `num_seconds()`, and return exact supported seconds in `src/verify/date_diff.rs` (after T010).
- [X] T013 [US1] Add candidate-ID lookup and collision-safe, transactional Duration insert/reuse in `src/store/queries.rs`: reuse a historical `DATE_DIFF` only if kind, seconds, operation, and full parent evidence match; never use an incompatible-ID upsert (after T008 and T011).
- [X] T014 [US1] Wire optional temporal `--result`, `--result-unit`, and display-only `--unit` through `src/main.rs` and `src/cli/derive.rs`; parse a Date claim from the original decimal token with T006's exact conversion, default its claim unit to seconds, reject `--result-unit` without `--result`, and report an exact expected/supplied-seconds mismatch before T013 writes. Retain required f64-style `EXPRESSION` result semantics, preserve no-unit `Derived node:` stdout, and report node ID plus selected unit/exact seconds when requested (after T006, T012, T013).
- [X] T015 [US1] Extend `src/export/turtle.rs` derivation classification to recognize canonical and historical `DATE_DIFF` IDs, preserving supported PROV activity, parent uses, and typed `xsd:duration` output (after T007, T011, T014).

**Checkpoint**: US1 passes its unit, CLI, graph, and Turtle tests and can be demonstrated using the 45-day section of [quickstart.md](quickstart.md) without US2–US4.

---

## Phase 4: User Story 2 - Average or add durations without losing their type (Priority: P2)

**Goal**: `DURATION_SUM` and `DURATION_AVG` operate on 2–26 distinct signed Duration parents, produce typed Duration nodes, and reuse one identity for parent permutations.

**Independent Test**: A 45-day and 45-minute gap yield a 3,890,700-second Duration sum and 1,945,350-second Duration average; reversed parent order reuses IDs; duplicate/wrong-kind/overflow/fractional-second inputs write nothing.

### Tests for User Story 2

- [X] T016 [P] [US2] Write failing unit tests for checked signed sum, exact divisible average, 1.5-second average rejection, overflow, 2–26 parent count, repeated IDs, and Date/Number type rejection in `tests/unit/test_duration_aggregate.rs`.
- [X] T017 [P] [US2] Write failing CLI contract tests for automatic `DURATION_SUM`/`DURATION_AVG`, `--unit days`, invalid flags/types, and exact no-write error paths in `tests/contract/test_duration_aggregate_contract.rs`.
- [X] T018 [P] [US2] Write failing integration tests for both typed graph nodes, complete parent edges, signed values, permutation deduplication, repeated-ID rejection, and Turtle activities in `tests/integration/test_duration_aggregate_graph.rs`.

### Implementation for User Story 2

- [X] T019 [US2] Implement `DURATION_SUM` and `DURATION_AVG` over exact signed canonical seconds in `src/verify/duration.rs` and register both in `src/verify/mod.rs`; require 2–26 distinct Duration parents, reject checked overflow and non-whole-second means without rounding (after T016).
- [X] T020 [US2] Route both operations through `src/main.rs` and `src/cli/derive.rs`, canonicalize parent order for identity and edge insertion, record only Duration results through T013's safe transaction path, and keep display/claim units out of the graph (after T017, T018, T019).
- [X] T021 [US2] Extend `src/export/turtle.rs` operation allow-list and canonical-ID validation for sum/average, so a supported activity `prov:used` every parent and the child remains typed `xsd:duration` (after T018 and T020).

**Checkpoint**: US2 passes independently using stored Duration parents; the mixed-scale section of [quickstart.md](quickstart.md) yields typed values and complete lineage.

---

## Phase 5: User Story 3 - Verify claims with explicit units (Priority: P3)

**Goal**: `verify` and optional derive assertions use exact `--result-unit` semantics, preserve old seconds claims, and warn when legacy general expressions coerce temporal parents.

**Independent Test**: `verify DATE_DIFF --result 45 --result-unit days` and `--result 3888000` both succeed; wrong/unrepresentable claims fail without writes; `EXPRESSION` keeps its Number result and emits a temporal-coercion warning.

### Tests for User Story 3

- [X] T022 [P] [US3] Write failing CLI contract tests for derive/verify claim parity, default seconds, scientific and signed decimal tokens, invalid unit/flag combinations, exit codes 0/1/2, and unchanged `EXPRESSION` behavior in `tests/contract/test_duration_claim_contract.rs`.
- [X] T023 [P] [US3] Write failing unit tests for exact `0.1 minutes = 6 seconds`, `1.0000000000000001 seconds` rejection, non-finite/overflow syntax, and equivalent assertions across all five units in `tests/unit/test_duration_claim.rs`.

### Implementation for User Story 3

- [X] T024 [US3] Reuse T014's raw temporal `--result` and `--result-unit` parsing for `verify` and both typed aggregates, while retaining required `--result` and existing numeric parsing for `EXPRESSION`; reject `verify --unit` and unit flags on unrelated operations in `src/main.rs` (after T022, T023).
- [X] T025 [US3] Generalize T014's exact claim conversion and expected/supplied-seconds mismatch diagnostics across all temporal derive/verify operations in `src/verify/mod.rs` and `src/cli/derive.rs`; keep successful verify silent, classify malformed/unrepresentable input as exit 2 and a representable mismatch as exit 1, and write no node on either failure (after T024).
- [X] T026 [US3] Emit a stderr warning when legacy `EXPRESSION` receives a Date or Duration parent, naming Unix timestamp seconds versus elapsed seconds and the Number result, without changing the expression's value or exit status in `src/verify/expression.rs` and `src/verify/mod.rs` (after T022, T025).

**Checkpoint**: US3 passes its claim tests for both `DATE_DIFF` and typed aggregates and leaves historical seconds-based commands meaningful.

---

## Phase 6: User Story 4 - Redisplay a stored duration (Priority: P3)

**Goal**: `query --unit` adds a selected-unit view of the requested Duration without altering `value`, lineage, storage, or the no-unit response.

**Independent Test**: Query one 3,888,000-second Duration in days and minutes; keep its existing `value`, ID, and lineage; add exact seconds and selected-unit display only to the requested node; reject Number/Date targets.

### Tests for User Story 4

- [X] T027 [US4] Write failing CLI contract tests for default and `--json` query wrappers, unchanged no-unit shape, requested-node-only `duration_seconds` and `selected_unit_display`, approximate conversion, non-Duration rejection, and no graph write in `tests/contract/test_duration_query_contract.rs`.

### Implementation for User Story 4

- [X] T028 [US4] Add the selected unit view only to the requested Duration's lineage entry in `src/query/mod.rs`, preserving `value` and other entries; expose lossless `duration_seconds` as a decimal string and a structured display with `unit`, `decimal`, `approximate`, and `text` (after T006 and T027).
- [X] T029 [US4] Add `query --unit` routing and validation in `src/main.rs` and `src/cli/query.rs`, preserving the existing wrapped default and direct `--json` shapes when the flag is omitted (after T028).

**Checkpoint**: US4 passes independently on any stored Duration node, including a historical one; selecting a unit is response-only.

---

## Phase 7: Polish & Cross-Cutting Concerns

**Purpose**: Bring docs, export, performance, and local quality checks into alignment across all stories.

- [X] T030 [P] Update command examples and behavior notes for automatic date gaps, typed sum/average, claim/display flags, whole-second limits, query fields, legacy expression warning, and unsupported months/years in `README.md`, `README_pt.md`, `README_es.md`, `README_fr.md`, `README_ja.md`, `README_ko.md`, `README_zh.md`, and `src/cli/init.rs`.
- [X] T031 [P] Update `AGENTS.md` recording/derivation/query guidance so agents calculate date gaps and mixed Duration results with typed operations and read exact seconds from selected-unit responses.
- [X] T032 [P] Add a fixed 10,000-node, bounded-parent fixture in `tests/integration/test_duration_performance.rs` for non-gating local release-build subprocess timing of valid and invalid derive/verify commands for all three temporal operations plus valid and invalid `query --unit`, including 26 distinct Duration parents for aggregates. Mark the timing test `#[ignore = "manual local timing"]` so ordinary `cargo test` skips it, and run it only with `rtk cargo test --release --test test_duration_performance -- --ignored --nocapture`. Follow the protocol in `specs/043-date-diff-units/quickstart.md`: run one warm-up and 20 measured invocations per command, include startup/output, calculate the 95th percentile against the one-second budget, and record OS, CPU, release profile, commands, and measurements. Report timing without a wall-clock pass/fail assertion; use no network or hosted workflow.
- [X] T033 Run the isolated local scenarios in `specs/043-date-diff-units/quickstart.md` against a temporary project, check typed Turtle output and historical fixtures, and record any corrected validation notes in `specs/043-date-diff-units/quickstart.md`.
- [X] T034 Run local `rtk cargo fmt --all -- --check`, `rtk cargo clippy --all-targets -- -D warnings`, and relevant/full `rtk cargo test` suites; resolve only concrete failures in `src/` and `tests/`, and do not invoke CI/CD or GitHub workflows.
- [X] T035 Before any future feature commit or PR, explicitly stage `specs/043-date-diff-units/` despite the repository's `specs/` ignore rule, verify its spec, plan, tasks, contracts, and validation notes appear in the staged changeset with the implementation and user-facing docs, and keep the staged design documents aligned with the implemented behavior. Do not launch CI/CD or GitHub workflows.

**Checkpoint**: All acceptance scenarios, docs, deterministic tests, local quality checks, and the staged canonical feature documents agree with the spec. Preserve the user's no-workflow restriction; any future PR/merge has a separate governance gate.

---

## Dependencies & Execution Order

### Phase Dependencies

```text
Setup (T001–T002)
  → Foundation (T003–T008)
  → US1 (T009–T015, MVP)
  → US2 (T016–T021)
  → US3 (T022–T026)
  → US4 (T027–T029)
  → Polish (T030–T035)
```

US2 needs US1's safe Duration write path and canonical ID. US3 needs computed temporal results from US1/US2 to verify all three operations. US4 can begin after Foundation with any stored Duration, but follows US3 in the default single-developer sequence. Polish follows completed stories.

### Within Each User Story

- Write the story's tests first and observe the intended local failure; implement only after those tests define the contract.
- Complete models/shared calculation rules before CLI routing, then graph/export integration.
- Use the story checkpoint and its [quickstart.md](quickstart.md) scenario before proceeding in the default sequence.

### Parallel Opportunities

- Foundation tests T003–T005 touch different test files and can be written in parallel; their implementation counterparts T006–T008 touch separate source modules after the tests exist.
- US1 tests T009–T011, US2 tests T016–T018, and US3 tests T022–T023 each cover different files and can be written in parallel within their phase.
- Cross-cutting documentation T030–T031 and performance fixture T032 touch different files after core behavior is stable.
- US4's query path can be developed alongside US2/US3 after Foundation and a stored Duration fixture exists, but shared `src/main.rs` integration must be serialized.

## Parallel Examples

### User Story 1

```text
T009: Write tests/contract/test_duration_cli_contract.rs
T010: Extend tests/unit/test_date_diff.rs
T011: Write tests/integration/test_duration_date_diff.rs
```

### User Story 2

```text
T016: Write tests/unit/test_duration_aggregate.rs
T017: Write tests/contract/test_duration_aggregate_contract.rs
T018: Write tests/integration/test_duration_aggregate_graph.rs
```

### User Story 3

```text
T022: Write tests/contract/test_duration_claim_contract.rs
T023: Write tests/unit/test_duration_claim.rs
```

### User Story 4

```text
T027: Write tests/contract/test_duration_query_contract.rs while US2/US3 use their own test files
T028–T029: Integrate query output after T027; serialize the `src/main.rs` edit with US2/US3
```

## Implementation Strategy

### MVP First (User Story 1)

1. Complete Setup and Foundation.
2. Implement US1 only, including exact 45-day derivation, claim check, legacy ID reuse, and Turtle activity support.
3. Validate US1 locally and confirm no implementation step launched a hosted workflow.

### Incremental Delivery

1. Add US2 typed sums and averages; keep parent sets/identity correct.
2. Add US3 complete verify and exact claim coverage while preserving seconds-based and expression behavior.
3. Add US4 query presentation; keep the existing `value` field and no-flag response.
4. Finish docs, performance fixtures, and local quality checks. Keep commits/PR/merge subject to the constitution and the user's separate no-workflow instruction.

## Notes

- `specs/` and `.specify/` are ignored locally by this repository. Keep the feature artifacts available to the implementer, and explicitly stage `specs/043-date-diff-units/` for any future implementation changeset as required by Constitution VII and X. `.specify/` tooling is outside this feature's publication task.
- `--unit` is presentation; `--result-unit` qualifies an asserted result. Neither changes stored seconds or graph identity.
- No task authorizes CI/CD or GitHub workflow execution.

## Phase 8: Convergence

- [X] T036 Prevent lossy temporal verification through the public `verify(parents, operation, expression, result: f64)` path in `src/verify/mod.rs`: a fractional or rounded `DATE_DIFF` claim must never pass by conversion to `i64`; use an exact temporal claim API or return an actionable error that directs callers to one, while preserving `EXPRESSION` behavior. Add deterministic public-API regression cases for fractional and precision-sensitive inputs per FR-005 and T025.
- [X] T037 Update `derive`, `verify`, and `query` command help plus `README.md` and `AGENTS.md` to state the five fixed units, whole-second limit, unsupported calendar months/years, legacy temporal `EXPRESSION` warning, and that claim/display units are not persisted as node metadata. Add help-output contract assertions for these promises per FR-012.
- [X] T038 Add deterministic CLI fixtures that derive and query an equivalent Duration using every supported `--result-unit` and `--unit`, asserting stable node identity, exact `duration_seconds`, unchanged `value` and lineage, no duplicate graph writes, and approximate marking where conversion repeats; retain the current no-unit response checks per SC-002 and Constitution IX.

## Phase 9: Convergence

- [X] T039 Reject positive and negative fractional `ValueData::Duration` inputs before `num_seconds()` in the public aggregate and persistence paths in `src/verify/duration.rs` and `src/store/queries.rs` (`insert_tracked_node` and `insert_or_reuse_duration`); add deterministic tests proving an error with no node or edge write, while preserving exact whole-second behavior per FR-007 and FR-009.

## Phase 10: Convergence

- [X] T040 Return invalid-input exit code 2 for `query --unit` on Number and Date nodes in `src/main.rs` and `src/cli/query.rs`, preserving the no-write error and adding CLI contract assertions for both kinds per FR-010 and the plan's CLI contract (partial).
- [X] T041 Require supported temporal operation, parent count and kinds, and absent expression before `src/export/turtle.rs` classifies a Duration derivation as a `prov:Activity`; preserve evidence-backed direct lineage for unsupported groups and add deterministic malformed-graph export cases per FR-009 and T021 (partial).

## Phase 11: Convergence

- [X] T042 Make `DATE_DIFF` with the same Date node ID in both ordered operand positions reusable in `src/store/queries.rs` and reconstructable in `src/export/turtle.rs` from its canonical or historical ID and one distinct stored edge; emit two indexed operand usages, keep duplicate IDs invalid for duration aggregates, and add deterministic first/repeated derive, zero-gap, historical-reuse, and Turtle cases per FR-001, FR-008, and FR-009.

## Phase 12: Convergence

- [X] T043 Parse bounded decimal Duration claims in `src/types/duration_unit.rs` without overflowing on a large intermediate mantissa when the scaled value is an exact, representable whole-second Duration; retain token and exponent work limits, reject genuinely fractional or out-of-range values, and add deterministic regression cases such as `1.0000000000000000000000000000000000000000` seconds per FR-005 (partial).
