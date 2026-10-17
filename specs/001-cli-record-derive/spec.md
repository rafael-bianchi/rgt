# Feature Specification: CLI Record & Derive Commands

**Feature Branch**: `001-cli-record-derive`

**Created**: 2026-08-08

**Status**: Draft

**Input**: User description: "Add `rgt record` and `rgt derive` CLI commands to enable Kilo Code (and other rules-file agents) to record root values and verify-then-record derived values into the provenance graph, without requiring automatic hook interception."

## Clarifications

### Session 2026-08-08

- Q: What limit applies to `rgt record` for files with very large numbers of extractable values? → A: Soft limit of 10,000 values per file; a warning is printed to stderr and excess values are silently dropped; exit code remains 0.
- Q: What should `rgt record` print to stdout on success? → A: A summary line: `Recorded <N> values from <file>`. Silent on failure (errors go to stderr).

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Record Root Values from Data Files (Priority: P1)

A developer using an AI coding agent that lacks automatic hook interception (Kilo Code, Windsurf) needs to register numeric and date values from data files into the provenance graph so they can later verify derived calculations. The developer, or their agent, runs `rgt record budget.csv` after reading the file, and RGT extracts numbers and dates, creating root nodes in the graph.

**Why this priority**: Without root value recording, the provenance graph remains empty for rules-file agents, rendering staleness detection and derivation verification unusable. This is the foundational command.

**Independent Test**: Create a CSV file with numeric values, run `rgt record <file>`, verify with `rgt status` that root nodes appear with correct values and line numbers.

**Acceptance Scenarios**:

1. **Given** a file `data.csv` containing `revenue,120000` on line 2, **When** the agent runs `rgt record data.csv`, **Then** a root node with value `120000` and line number 2 is created in the provenance graph.
2. **Given** a file `schedule.txt` containing `start: 2026-01-15` on line 1, **When** the agent runs `rgt record schedule.txt`, **Then** a root node with date value `2026-01-15` and value kind `DATE` is created.
3. **Given** an existing root node from a previous recording, **When** the same file is recorded again with unchanged values, **Then** the existing nodes are re-upserted as fresh (is_stale=false) without creating duplicates.
4. **Given** a file that has been modified since the last recording, **When** `rgt record` is run again, **Then** unchanged values are refreshed, changed values create new nodes, and the old nodes remain (to be marked stale by `rgt status`).

---

### User Story 2 - Verify-Then-Record Derived Values (Priority: P1)

A developer or agent computes a derived value from existing root nodes (e.g., `profit = revenue - costs`) and wants to record this derivation into the graph. The agent runs `rgt derive --parents <id1>,<id2> --operation EXPRESSION --expression "a - b" --result 60000`. RGT first verifies the expression against the parent node values, and only if it matches does it insert a derived node and derivation edges.

**Why this priority**: Derived value recording with verification gating is the safety mechanism that prevents agents from inserting incorrect derivations. Without it, the graph could accumulate invalid derived nodes, undermining trust in staleness cascades.

**Independent Test**: Record two root nodes, run `rgt derive` with a correct expression and result, verify the derived node appears in `rgt status`. Then attempt `rgt derive` with a wrong result, verify the derivation is rejected.

**Acceptance Scenarios**:

1. **Given** root nodes for `revenue=120000` and `costs=60000`, **When** the agent runs `rgt derive --parents <rev_id>,<cost_id> --operation EXPRESSION --expression "a - b" --result 60000`, **Then** RGT verifies the expression matches, inserts a derived node with value 60000 and derivation edges linking both parents.
2. **Given** root nodes for `revenue=120000` and `costs=60000`, **When** the agent runs `rgt derive` with `--result 50000` (wrong), **Then** RGT rejects the derivation with exit code 1 and a message explaining the mismatch, and no node or edges are inserted.
3. **Given** two date nodes `2026-01-15` and `2026-06-30`, **When** the agent runs `rgt derive --parents <d1>,<d2> --operation DATE_DIFF --result 14342400`, **Then** RGT verifies the date difference matches and records a derived duration node.
4. **Given** a derived node with edges to parent nodes, **When** either parent becomes stale, **Then** the `rgt status` cascading invalidation marks the derived node as stale.

---

### User Story 3 - Agent Integration via Updated Rules Files (Priority: P2)

When `rgt init --agent codex` runs, the AGENTS.md rules file includes clear instructions for the agent on how to use `rgt record` after reading data files and `rgt derive` after computing derived values, enabling automatic provenance tracking in agent workflows.

**Why this priority**: Agent instructions drive adoption. Without updated AGENTS.md, rules-file agents won't know these commands exist.

**Independent Test**: Run `rgt init --agent codex --force` in a fresh project, verify AGENTS.md contains instructions for `rgt record` and `rgt derive`.

**Acceptance Scenarios**:

1. **Given** a project without RGT, **When** `rgt init --agent codex` is run, **Then** AGENTS.md includes guidance on using `rgt record <file>` after reading data files.
2. **Given** a project without RGT, **When** `rgt init --agent codex` is run, **Then** AGENTS.md includes guidance on using `rgt derive` with example expression syntax.

---

### Edge Cases

- **Empty file**: Recording a file with no numeric or date content prints `Recorded 0 values from <file>` to stdout and exits 0.
- **File not found**: Recording a path that does not exist exits with code 2 and a clear error message.
- **Non-numeric parent**: Attempting EXPRESSION derivation with a DATE parent bound to a variable should evaluate successfully (dates convert to epoch seconds).
- **Division by zero**: Expression evaluation that divides by zero returns a clear error with exit code 2.
- **Unknown variable in expression**: Using a variable not bound to any parent (e.g., `d` when only 2 parents provided) returns an error with exit code 2.
- **Duplicate derivation**: Recording the same derivation twice (same parents, operation, and result) does not create duplicate edges or nodes.
- **Stale parents**: Derivation is permitted even if parents are marked stale; the derived node is created fresh but `rgt status` will mark it stale during the next cascade pass.
- **Record overwrites existing derived nodes**: `rgt record` only creates `NodeType::Root` nodes — it never creates or modifies derived nodes.
- **Large files exceeding value limit**: When a file contains more than 10,000 extractable values, a warning is printed to stderr (`Warning: value limit (10000) reached, excess values skipped for <file>`) and stdout prints `Recorded 10000 values from <file>`; exit code is 0.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: System MUST provide a `rgt record <FILE>` subcommand that reads a file from disk, extracts numeric and ISO-8601 date values using the same extraction engine as passive hooks, and upserts them as root nodes into the provenance graph.
- **FR-002**: System MUST provide `rgt record --stdin <FILE>` variant that reads file content from standard input instead of disk, using the provided path argument for node ID generation and source document tracking.
- **FR-003**: System MUST provide a `rgt derive --parents <ids> --operation <op> [--expression <expr>] --result <val>` subcommand that verifies a derived computation against parent node values before recording.
- **FR-004**: `rgt derive` MUST first look up parent nodes by ID from the database, evaluate the expression or date-diff against their values, and only insert a derived node and derivation edges if verification succeeds (result matches).
- **FR-005**: `rgt derive` MUST reject the derivation with exit code 1 and a descriptive message when the claimed result does not match the computed value, without inserting any nodes or edges.
- **FR-006**: `rgt derive` MUST reject invalid input (unknown parent IDs, wrong parent count for DATE_DIFF) with exit code 2 and a message explaining what was wrong and how to fix it.
- **FR-007**: `rgt record` MUST exit with code 2 when the specified file path does not exist, with a message stating the file was not found.
- **FR-008**: `rgt derive` MUST generate deterministic derived node IDs from parent IDs, operation type, and result value, using the same ID scheme as the existing derivation infrastructure, so derived nodes are stable across re-derivations.
- **FR-009**: The AGENTS.md (codex) and `.windsurfrules` (windsurf) templates in the hook installer MUST be updated to include instructions for `rgt record` and `rgt derive` when initializing for rules-file agents.
- **FR-010**: The `rgt --help` output MUST list `record` and `derive` as available subcommands with brief descriptions.
- **FR-011**: `rgt record` MUST enforce a soft limit of 10,000 extracted values per file; when exceeded, a warning is printed to stderr, excess values are dropped, and exit code remains 0.
- **FR-012**: `rgt record` MUST print a summary line to stdout on success in the format `Recorded <N> values from <file>`, where `<N>` is the count of extracted values (not including values dropped due to the limit). On empty files, prints `Recorded 0 values from <file>`.
- **FR-013**: `rgt derive` MUST print the derived node ID to stdout on success in the format `Derived node: <node_id>`. On verification failure (exit 1) or invalid input (exit 2), prints nothing to stdout; errors go to stderr.

### Key Entities

- **Root Node**: A value extracted from a data file (number or date) with a unique ID derived from file path, line number, and value. Created by `rgt record` or passive hooks. Cannot be created by `rgt derive`.
- **Derived Node**: A computed value verified against parent nodes, with a unique ID derived from parent IDs, operation type, and result value. Created only by `rgt derive` upon successful verification.
- **Derivation Edge**: A directed relationship linking a parent node to a derived child node, annotated with operation type and expression. Inserted only when `rgt derive` succeeds.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: A developer can record values from a data file and see them in `rgt status` within 1 second of running `rgt record`. Bulk inserts are wrapped in a single transaction; the CI regression threshold is 8 seconds — set to catch a per-node auto-commit regression (~16s on CI) with headroom for shared-runner variance (~2.8s observed on Windows CI).
- **SC-002**: An incorrect derivation is rejected 100% of the time with a clear error message — no false positives (incorrect values inserted) or false negatives (correct values rejected).
- **SC-003**: `rgt derive` reuses the existing expression and date-diff verification logic rather than duplicating it, ensuring consistent behavior between the passive hook path and the active CLI path.
- **SC-004**: An AI coding agent can perform a complete provenance workflow (record roots, verify and record a derivation, check staleness after modification) without any automatic hook interception, using only `rgt record`, `rgt derive`, and `rgt status`.
- **SC-005**: Existing integration tests for the hook path continue to pass, confirming that `rgt record` does not alter the behavior of `rgt hook post`.
- **SC-006**: All new CLI commands follow Constitution Principle XI exit code conventions: 0 = success, 1 = expected verification failure, 2 = invalid input or system error.

## Assumptions

- The `evalexpr` crate is already a project dependency and will be reused for `rgt derive` expression evaluation.
- The `insert_derivation_edge` and `generate_derived_id` functions in `src/store/queries.rs` and `src/types/node.rs` are correct as-is and only need wiring, not reimplementation.
- `rgt record` does not need to support `--agent` filtering or hook-specific JSON parsing — it directly uses the extraction engine.
- The `--stdin` variant of `rgt record` exists for agent workflows where the agent has file content in memory but the disk path may differ; the path argument is still required for node ID determinism.
- Derived nodes created by `rgt derive` participate in the existing staleness cascade without any changes to the invalidation module.
- Rules-file agents (Kilo Code, Windsurf) will follow AGENTS.md instructions to call `rgt record` after reading data files; no automatic trigger mechanism is required.
