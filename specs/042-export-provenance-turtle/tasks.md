---
description: "Dependency-ordered implementation tasks for Turtle provenance export"
---

# Tasks: Export Provenance as Turtle

**Input**: [spec.md](spec.md), [plan.md](plan.md), [research.md](research.md), [data-model.md](data-model.md), and [Turtle export contract](contracts/turtle-export-contract.md)

**Prerequisites**: Implementation starts only after the separate [product assessment](../../.specify/assessments/prov-o-turtle-export/decision.md) has a go decision and work is on a `feat/...` branch from `develop`. This task list does not change that decision.

**Tests**: Required by the feature's independent-test scenarios, measurable success criteria, and the project constitution. Write the story tests before the corresponding implementation, and use an independent Turtle parser for graph assertions. Keep the timing benchmark outside normal deterministic CI tests.

**Organization**: Each user story is a separately testable increment. `[P]` means different files can be worked on in parallel after their stated prerequisites.

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Prepare the existing single-crate Rust project for the new export and its independent contract tests.

- [X] T001 Select a license-compatible independent Turtle parser as a test-only dependency and update `Cargo.toml` and `Cargo.lock`; register targets and create empty files at `tests/contract/test_turtle_contract.rs`, `tests/unit/test_turtle.rs`, `tests/contract/test_turtle_identity_contract.rs`, `tests/integration/test_turtle_identity.rs`, `tests/integration/test_turtle_migration.rs`, `tests/contract/test_turtle_capture.rs`, and `tests/integration/test_turtle_performance.rs`.
- [X] T002 [P] Add the focused `src/export/mod.rs` and `src/export/turtle.rs` module skeleton and register it in both `src/lib.rs` and `src/main.rs`, without changing existing graph output.

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Establish durable project identity and a truthful, consistent export read path before any story emits Turtle.

**Critical**: Complete this phase before user-story work.

- [X] T003 Implement `rdf_project_id` persistence in `src/store/queries.rs` and `src/store/db.rs` using the data-model constraint “Random 128-bit lowercase hex value, inserted once with conflict-ignore and reread; immutable during normal use”; initialize absent tokens for new and old stores, validate existing tokens, and never rewrite node IDs.
- [X] T004 [P] Implement `limit + 1` preflight for 10,000 `tracked_nodes` and 30,000 `derivation_edges` inside one SQLite export snapshot, then bulk-load their referenced `source_documents` in `src/export/turtle.rs`; validate that each root references at most one source and fail hard on exceeded limits, malformed values, or broken required references rather than using the existing permissive query fallbacks.

**Checkpoint**: A project has a stable token and the exporter can obtain one validated graph snapshot without producing output.

---

## Phase 3: User Story 1 - Share a project's lineage (Priority: P1) MVP

**Goal**: Export one project's values, sources, and supported derivations as valid, deterministic Turtle while preserving existing graph formats.

**Independent Test**: Record a source value and a two-parent derivation, export with `rgt graph --format ttl`, and use an independent Turtle reader to establish typed values, source location, direct lineage, validated derivation activity, and operand order. Also cover empty, ambiguous, escaped, and error cases.

### Tests for User Story 1

- [X] T005 [P] [US1] Add parser-based CLI contract cases for empty output, number/date/duration literals, `rgt:localNodeId`, roots and sources, multi-parent PROV-O direction and ordered usage, escaping, default/full-path disclosure, ambiguous derivations with no consolidated operation/expression claim, row-limit errors and an injected small byte-cap boundary with zero stdout, controlled pre-write and final-write deadline failures, invalid flag exit 2, and unchanged text/Mermaid/DOT output in `tests/contract/test_turtle_contract.rs`.
- [X] T006 [P] [US1] Add focused cases for all project-scoped IRI suffixes, Turtle literal and IRI escaping, safe project-relative path classification, XML Schema duration lexical form, the full-ID/historical-48-bit derivation evidence rule, and quota/deadline decisions under an injected clock and controlled writer in `tests/unit/test_turtle.rs`.
- [X] T007 [P] [US1] Add a derived-node write-failure regression proving that the child and all parent edges commit together or all roll back in `tests/contract/test_cli_record_derive_contract.rs`.

### Implementation for User Story 1

- [X] T008 [US1] Implement the contract's project-token IRI builder for project, node, source, derivation activity, and qualified usage resources, percent-encode stored local IDs, and add safe Turtle string escaping plus exact `xsd:double`, `xsd:dateTime`, `xsd:duration`, `xsd:boolean`, and `xsd:integer` lexical output in `src/export/turtle.rs`; retain RGT's stored number/date/duration values without reusing human-readable duration display text.
- [X] T009 [US1] Map root sources to path-free project-scoped source IRIs in `src/export/turtle.rs`; emit `rgt:relativePath` only for verified in-project files, omit paths for outside/unclassifiable files, and emit `rgt:absolutePath` only under the explicit full-path option.
- [X] T010 [US1] Classify each derived child's incoming edges in insertion-ID order in `src/export/turtle.rs`: require existing distinct parents, equal operation and expression, and recomputed full 128-bit or historical 48-bit child ID; preserve supported `prov:wasDerivedFrom` links while omitting uncertain activities and any consolidated operation/expression claim, with one stderr warning per omitted child.
- [X] T011 [P] [US1] Make the verified derived-node insert and all `derivation_edges` inserts one transaction in `src/cli/derive.rs`, preserving the current invalid-input and verification exit semantics.
- [X] T012 [US1] Complete the fixed-order, buffered Turtle mapping in `src/export/turtle.rs`: project and value `prov:Entity` facts with the unchanged `rgt:localNodeId`, typed RGT terms, root-source lineage, supported derivation `prov:Activity` facts, `prov:used`, `prov:wasGeneratedBy`, `prov:wasDerivedFrom`, and `prov:qualifiedUsage` with zero-based `rgt:operandIndex`; abort buffered rendering when the next bytes would exceed 64 MiB (67,108,864 bytes), before stdout.
- [X] T013 [US1] Add `ttl` dispatch and the TTL-only `--include-absolute-paths` option in `src/main.rs` and `src/cli/graph.rs`, with a Turtle-specific bounded lock wait in `src/store/db.rs`; keep unknown-format text fallback and legacy formats unchanged. Enforce one deadline from CLI entry through snapshot and render. For final stdout, move the fully rendered buffer to a dedicated writer thread, wait on a completion channel for only the remaining budget, and on timeout report to stderr and terminate nonzero without joining a blocked writer; never write a success/completeness claim after a partial write. Return exit 1 with empty stdout for quota/store/pre-write timeout errors, exit 2 for invalid flags, and nonzero for final timeout or OS write failure. Cover successful, broken-pipe, and controlled stalled-writer outcomes in the existing T005/T006 tests.

**Checkpoint**: User Story 1 passes its independent Turtle-reader and regression cases without any agent attribution.

---

## Phase 4: User Story 2 - Combine project exports safely (Priority: P2)

**Goal**: Preserve identities within one project while preventing accidental equality when independent projects share local node IDs.

**Independent Test**: Export two independently initialized projects with matching relative paths, lines, and local node IDs twice; parse the RDF union and establish two distinct value identities, stable repeat identities, and unchanged identity in a physical copy of one store.

### Tests for User Story 2

- [X] T014 [P] [US2] Add independent-parser RDF union assertions for matching local IDs in two projects, the `rgt:localNodeId` comparison literal, and absence of cross-project equivalence claims in `tests/contract/test_turtle_identity_contract.rs`.
- [X] T015 [P] [US2] Add repeated-export, copied-store, concurrent first-open, malformed-token, and historical 48-bit ID identity cases in `tests/integration/test_turtle_identity.rs`.

### Implementation for User Story 2

- [X] T016 [US2] Add an identity invariant check in `src/export/turtle.rs` that rejects any RGT instance IRI outside the current project's persisted token namespace before stdout, except the canonical global agent namespace; apply it to historical short IDs and copied-store exports without treating PROV-O, XSD, or RGT vocabulary IRIs as instances.

**Checkpoint**: An ordinary RDF union keeps independent projects separate without an RGT-specific merge operation.

---

## Phase 5: User Story 3 - See which agent captured a value (Priority: P3)

**Goal**: Durably retain one known-agent/value association per successful capture and export it as a capture activity, without implying source authorship.

**Independent Test**: Capture the same value twice with one event-capable agent and once with another, reopen the store, and establish exactly one activity per agent/value pair; compare manual, historical, unknown-agent, rules-only, failed-hook, and PDF cases.

### Tests for User Story 3

- [X] T017 [P] [US3] Add version-0/version-1 to version-2 migration, rollback, preservation of old nodes/edges, newer-version rejection, malformed-token error, and node-delete cascade cases in `tests/integration/test_turtle_migration.rs`.
- [X] T018 [P] [US3] Add hook-to-Turtle contract cases for all eight event-capable integrations (`claude-code`, `cursor`, `copilot`, `gemini`, `vibe`, `opencode`, `pi`, `hermes`), aliases, duplicate and multi-agent capture, PDF capture, fresh-process durability, manual/historical absence, unknown/rules-only no-attribution, and fail-open behavior in `tests/contract/test_turtle_capture.rs`.

### Implementation for User Story 3

- [X] T019 [US3] Advance `PRAGMA user_version` to 2 with an atomic migration in `src/store/schema.rs`; add `capture_associations(node_id, agent_name)` with `(node_id, agent_name)` primary key, where `node_id` is a “Required foreign key to `tracked_nodes.id`; delete cascades when the node is removed” and `agent_name` is a “Required canonical agent name, validated against the supported registry before insertion.”
- [X] T020 [US3] Add idempotent association insert and ordered association reads in `src/store/queries.rs`, enforcing the canonical-name constraint and preserving zero rows for historical/manual values; do not add a timestamp or capture count.
- [X] T021 [US3] Normalize hook `--agent` spellings through the existing 13-agent registry before event parsing in `src/main.rs` and `src/hooks/mod.rs`; admit only the eight hook/plugin tiers to capture evidence, retain omitted-agent Claude Code behavior, and leave unknown/rules-only events fail-open without attribution.
- [X] T022 [US3] Insert each known-agent/value association in the same successful text-capture node transaction in `src/hooks/mod.rs`; keep repeated pairs idempotent and another agent's pair additive.
- [X] T023 [US3] Pass canonical agent identity through the PDF capture path and insert its association in the same successful PDF node transaction in `src/hooks/mod.rs`, preserving the current Claude Code additional-context behavior and fail-open errors.
- [X] T024 [P] [US3] Load capture associations in the same validated export snapshot in `src/export/turtle.rs`, rejecting unknown agents, duplicate pairs, and missing value references; the eight-agent registry and one-pair-per-value rule bound their count without another export quota. Emit one project-scoped capture `prov:Activity` per stored pair, `prov:used` the value, `prov:wasAssociatedWith` the canonical `prov:Agent`, and `rgt:agentName`, using the project IRI builder and identity check without authorship or derivation-responsibility claims.

**Checkpoint**: User Story 3 passes fresh-process attribution and migration tests while every supported host hook remains passive.

---

## Phase 6: Polish & Cross-Cutting Concerns

**Purpose**: Bring documentation, performance evidence, and repository gates into line with the delivered CLI behavior.

- [X] T025 Update the Turtle command, path privacy flag and free-text caveat, row/byte limits and timeout behavior, project-scoped identity, derivation warning, and capture-attribution scope in `README.md` and `AGENTS.md`.
- [X] T026 [P] Update graph-format references and the new flag in `README_pt.md`, `README_fr.md`, `README_es.md`, `README_zh.md`, `README_ja.md`, and `README_ko.md` without changing unrelated translations.
- [X] T027 Add two separately filterable ignored probes, `release_latency_boundaries` and `blocked_stdout_pipe`, in `tests/integration/test_turtle_performance.rs`, both invoking the `RGT_BENCH_BIN` release binary. The latency probe covers the contract's fixed 10,000-value fixture and exact 10,000/10,001 value, 30,000/30,001 edge, and 67,108,864/67,108,865 rendered-byte boundaries. Generate valid deterministic stores, confirm all non-target counts and rendered bytes, measure fresh process startup through output to a local file, and use one warm-up plus ten measured runs per fixture. The pipe probe uses a valid output of at least 2 MiB: retain but never read the child's pipe, assert its nonzero timeout exit within one second, and use an outer watchdog only to kill and fail a hung child. Keep these wall-clock probes ignored in normal deterministic CI.
- [ ] T028 Add an on-demand `workflow_dispatch` matrix for `ubuntu-24.04`, `macos-latest`, and `windows-latest` in `.github/workflows/turtle-performance.yml`; build the release binary and pass its path as `RGT_BENCH_BIN`, run the ignored `blocked_stdout_pipe` probe on every OS, and run the ignored `release_latency_boundaries` probe only on the standard GitHub-hosted x86_64 Ubuntu runner. Run the workflow and record the runner image version, CPU, memory, Rust version, exact fixture counts and output bytes, all ten elapsed times per boundary and representative case, stdout/stderr/exit outcomes, blocked-pipe results for all three OSes, and the one-second release decision in `docs/turtle-export-performance.md`; do not release if any supported exact-limit case or blocked-pipe deadline misses the bound.
- [X] T029 Run the story contract and migration targets, the scenarios in `specs/042-export-provenance-turtle/quickstart.md`, the full `cargo test` suite, `cargo fmt --all -- --check`, and `cargo clippy --all-targets`; record results and remaining limits in `specs/042-export-provenance-turtle/validation.md`.

---

## Dependencies & Execution Order

### Phase Dependencies

- Setup (T001–T002) precedes Foundation (T003–T004).
- Foundation precedes every user-story phase.
- User Story 1 is the MVP exporter. User Story 2 and the schema/hook work of User Story 3 can start after User Story 1. User Story 3's Turtle mapping T024 follows User Story 2's T016 because both edit `src/export/turtle.rs`.
- Polish follows all stories selected for delivery. T025 and T026 can proceed in parallel once CLI behavior is settled; T027–T029 are final gates.

### Within Each User Story

- Write each story's listed tests before its implementation tasks.
- In User Story 1, T008–T010 precede final mapping T012, and T012 precedes CLI dispatch and deadline wiring T013. The full project-scoped IRI and `rgt:localNodeId` contract is present by the US1 checkpoint; T016 only adds a defensive identity invariant. T011 is independent of the serializer after T007.
- In User Story 2, T014–T015 precede T016.
- In User Story 3, T019 precedes T020; T020 precedes hook writes T021–T023 and capture serialization T024. T024 can proceed in parallel with hook work once association reads exist.

### Parallel Execution Examples

- **US1**: After Foundation, T005, T006, and T007 edit separate test files in parallel. After tests exist, T011 (`src/cli/derive.rs`) can proceed alongside T008–T010 (`src/export/turtle.rs`).
- **US2**: After US1, T014 and T015 edit separate contract and integration test files in parallel; T016 follows their assertions.
- **US3**: After US1, T017 and T018 edit separate migration and capture test files in parallel with US2 tests. After T020 and US2's T016, T024 (`src/export/turtle.rs`) can proceed alongside T021–T023 (`src/main.rs` and `src/hooks/mod.rs`).

## Implementation Strategy

1. Complete Setup and Foundation, then User Story 1 as the single-project MVP. Validate it with an independent Turtle parser before adding project-combination and capture-attribution claims.
2. Complete User Story 2 and validate ordinary RDF union, copied-store identity, and historical IDs.
3. Complete User Story 3 and validate versioned migration plus all eight event-capable capture paths.
4. Finish documentation, the manual latency gate, and repository-wide tests and lint checks before delivery.

## Notes

- `[P]` marks work on different files without a dependency on another incomplete task at the same step.
- Tests describe the external contract and evidence boundaries; they must not infer agent identity or complete derivations from missing records.
- The reviewer-owned [Turtle export checklist](checklists/turtle-export.md) remains unchecked until a reviewer assesses requirements quality; these implementation tasks do not mark it complete.

## Phase 7: Convergence

- [X] T030 CRITICAL: Replace panicking duration reconstruction in `src/export/turtle.rs` with a checked conversion that returns an actionable pre-output error; add deterministic tests for out-of-range stored `duration_secs` values and zero stdout per Constitution XI and FR-014 (contradicts).
- [X] T031 Create and validate `rdf_project_id` during new and legacy store opens in `src/store/db.rs`, preserving conflict-ignore and reread behavior, concurrent-open safety, and the Turtle lock deadline; test a physical store copy made before either copy is exported per plan: identity and migration, data-model: project export identity, and T003 (partial).
- [X] T032 Report one stderr omission warning for each derived child with no incoming edge group while emitting no unsupported derivation activity; add an independent-parser CLI regression for the zero-edge case per FR-005, US1/AC3, and T010 (partial).
- [ ] T033 Run the hosted `ubuntu-24.04`, `macos-latest`, and `windows-latest` performance workflow, record its runner details, all required latency samples and pipe outcomes in `docs/turtle-export-performance.md`, and keep the release decision pending until every gate passes per SC-006 and T028 (partial).

## Phase 8: Convergence

- [ ] T034 Add a pre-release trigger for the Turtle performance matrix that can run before the workflow reaches the default branch, while retaining `workflow_dispatch`; execute the hosted Ubuntu latency/boundary and three-OS blocked-pipe probes and record all runner details, samples, outcomes, and the release decision per SC-006, plan: Interfaces and validation, Constitution: Latency Budget, T028, and T033 (partial).

## Phase 9: Convergence

- [ ] T035 Run the pre-release hosted Turtle performance matrix; record its workflow URL, runner image versions, CPU, memory, Rust versions, exact fixture counts and rendered bytes, all ten Ubuntu latency samples and stdout/stderr/exit outcomes at and over each limit, and blocked-pipe elapsed times, partial bytes, and errors on Ubuntu, macOS, and Windows in `docs/turtle-export-performance.md`; make the one-second release decision only after every required probe passes per SC-006, plan: Interfaces and validation, Constitution: Latency Budget, and T028/T033/T034 (partial).

## Phase 10: Convergence

- [ ] T036 Execute the existing pre-release Turtle performance workflow, record the hosted Ubuntu latency and quota-boundary samples plus Ubuntu, macOS, and Windows blocked-pipe outcomes and runner details in `docs/turtle-export-performance.md`, and decide the one-second release gate only after all required probes pass per SC-006, plan: Interfaces and validation, Constitution: Latency Budget, and T028/T033/T034/T035 (partial).
