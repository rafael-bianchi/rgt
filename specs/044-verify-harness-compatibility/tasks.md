---
description: "Implementation tasks for verifying RGT harness compatibility"
---

# Tasks: Verify Harness Compatibility

**Input**: `specs/044-verify-harness-compatibility/spec.md`, `plan.md`, `research.md`, `data-model.md`, `contracts/`, and `quickstart.md`

**Tests**: Required by the specification's independent tests and SC-002–SC-006, and by constitution Principle IX. Write the listed tests first and confirm that behavior-changing cases fail before implementing them. Use only local commands; no GitHub workflows or paid model calls.

**Organization**: Tasks are grouped by user story. `[P]` means the task can proceed in parallel with adjacent tasks because it writes different files and has no dependency on their unfinished output. Paths are relative to the repository root.

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Establish tracked support and evidence locations without changing client behavior.

- [X] T001 Create `docs/compatibility/support-matrix.md` with all 13 canonical agent identifiers, separate Copilot Chat/CLI and Cline/Roo surfaces, version/platform columns, stable read-path IDs with tool/accepted-result forms and excluded paths for event-capable surfaces, and the baseline unverified or instruction-only status from `specs/044-verify-harness-compatibility/evidence.md`; show guidance-only surfaces without invented native read paths, show native registration and per-path value-capture tiers separately for Codex and Windsurf, and do not infer live verification from RTK.
- [X] T002 [P] Create `tests/fixtures/hooks/README.md` defining native versus synthetic fixture labels and redaction rules for private paths, source contents, credentials, and session IDs.
- [X] T003 [P] Create tracked `docs/compatibility/evidence.schema.json` from `specs/044-verify-harness-compatibility/contracts/compatibility-evidence.schema.json`, document one schema-conforming record per observation with a read-path ID, loaded registration, completion/content evidence, observed number/date values and node IDs, and the same-path success plus no-op/error promotion rule in `docs/compatibility/evidence/README.md`, and initialize `docs/compatibility/evidence/observations.json` as an empty array.

**Checkpoint**: The tracked matrix and evidence locations exist; no client is called verified automatically.

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Give all stories a single client-surface identity and local registration vocabulary.

- [X] T004 Add and register `tests/unit/hook_surfaces.rs` in `Cargo.toml` with failing cases for 13 canonical identifiers, alias resolution, distinct `copilot-chat`/`copilot-cli` and `cline`/`roo-code` surfaces, stable native read-path IDs and exclusions without inventing paths for guidance-only clients, Codex/Windsurf native registration versus per-path value-capture tiers, and the rule that file existence never means verified capture.
- [X] T005 Implement the shared `ClientSurface`, `ReadPath`, and `IntegrationRegistration` descriptors in `src/hooks/registration.rs`, expose them from `src/hooks/mod.rs`, and reuse the existing canonical agent mapping in `src/hooks/installer.rs`; distinguish tool/accepted result form, registration kind, project/user scope, before/after phases, and local health without assigning a live verification claim.

**Checkpoint**: Surface identity and registration states are stable for capture, migration, doctor, and evidence.

---

## Phase 3: User Story 1 - Automatic Capture Actually Runs (Priority: P1) 🎯 MVP

**Goal**: Record values and capture-agent attribution only from a successful completed read while returning a valid neutral response to the client.

**Independent Test**: For one available no-cost client and exact version/platform, a native successful read produces the distinctive number/date and canonical agent association; a pre-only, failed, canceled, or no-op event creates no values and leaves the original client result unchanged. If no native check is available, the surface remains unverified and the story's live-acceptance condition remains open.

### Tests for User Story 1

- [X] T006 [P] [US1] Extend `tests/unit/hook_parsers.rs` with failing cases for before/after phase, explicit success/completed result/failure/cancellation/unknown outcome, empty/malformed/BOM input, Codex full versus truncated `PostToolUse` result, Windsurf path-only `post_read_code`, partial/filtered result, and changed snapshot; before, absent-result, and unmappable cases must be ineligible.
- [X] T007 [P] [US1] Extend `tests/contract/test_hooks_contract.rs` with failing CLI cases for a valid neutral no-op on every supported host dialect, unchanged tool result and approval/error flow, and fail-open behavior when parsing fails or the RGT executable is missing or moved, including direct command-hook hosts.
- [X] T008 [P] [US1] Extend `tests/contract/test_pdf_capture_contract.rs` with failing cases proving native PDF values and Claude `additionalContext` are handled only after a qualifying completed read whose decoded full source bytes match the current file snapshot.
- [X] T009 [P] [US1] Extend `tests/unit/store_integrity.rs` with failing cases for one root node and one `(node_id, agent_name)` association after duplicate callbacks, plus a separate association when a different client observes the same root.

### Client Contract Evidence for User Story 1

- [X] T010 [P] [US1] Document Claude Code, Cursor, and Codex read-event phases, stable read-path IDs with tools/accepted result forms and exclusions, success/error fields, required no-op response, vendor source, version/platform, and any no-cost native observation in `docs/compatibility/surfaces/claude-code.md`, `docs/compatibility/surfaces/cursor.md`, and `docs/compatibility/surfaces/codex.md`; include Codex `PostToolUse`, `tool_response`, trust review, and truncation limits; mark unobserved facts unknown.
- [X] T011 [P] [US1] Document Copilot Chat and Copilot CLI separately, including hook discovery, read-path IDs with tools/accepted result forms and exclusions, payload/result shape, and no-op response, in `docs/compatibility/surfaces/copilot-chat.md` and `docs/compatibility/surfaces/copilot-cli.md`; label RTK-only findings comparative.
- [X] T012 [P] [US1] Document Gemini CLI and Mistral Vibe completion-event availability, read-path IDs with tools/accepted result forms and exclusions, and read-result fields in `docs/compatibility/surfaces/gemini.md` and `docs/compatibility/surfaces/vibe.md`; if only before events are supported, state the instruction-only read-capture outcome.
- [X] T013 [P] [US1] Document OpenCode and Pi plugin callback phases, read-path IDs with tools/accepted result forms and exclusions, successful-read result fields, and neutral responses in `docs/compatibility/surfaces/opencode.md` and `docs/compatibility/surfaces/pi.md`; distinguish observed host behavior from comparative source.
- [X] T014 [P] [US1] Document Hermes plugin and Windsurf native hook contracts and read-path IDs with tools/accepted result forms and exclusions in `docs/compatibility/surfaces/hermes.md` and `docs/compatibility/surfaces/windsurf.md`, including Windsurf `post_read_code` path-only payload, active hook-file precedence, Restricted Mode, and neutral response; leave automatic value capture unverified without a qualifying observed full-content event.

### Implementation for User Story 1

- [X] T015 [US1] Extend `NormalizedCapture` in `src/hooks/parser.rs` with `phase`, `read_target`, `outcome_evidence`, `completed_content`, and original source-line mapping; treat explicit success without content, an after-event name alone, and a before event as ineligible for value nodes.
- [X] T016 [US1] Gate ordinary and PDF capture in `src/hooks/mod.rs` before database writes: reconstruct exact full source bytes and source lines from the completed result, compare them with the current file snapshot before storing its fingerprint, and create no source-backed root for path-only, partial/filtered, changed, or unmappable results; preserve canonical-agent attribution.
- [X] T017 [US1] Update `src/main.rs` and the command-hook registration launch path in `src/hooks/installer.rs` so Claude, Cursor, Copilot, Gemini, Vibe, Codex, and Windsurf have an enforced 800 ms RGT-process deadline or a documented tested host timeout/fail-open outcome, and receive their tested neutral host response on success, no-op, timeout, internal failure, and missing/moved RGT executable via a host-compatible fallback launcher where needed; never rewrite a command, approval, tool result, or error.
- [X] T018 [US1] Cap hook stdin at 8 MiB and enforce an elapsed stdin deadline in `src/hooks/mod.rs` even for a slow stream without EOF; enforce the 800 ms invocation/child deadline in the thin OpenCode, Pi, and Hermes wrappers in `src/hooks/glue/mod.rs`; return the required neutral response within one second and keep value extraction and success decisions in Rust.
- [X] T019 [US1] Implement only documented or observed per-surface payload/outcome adapters in `src/hooks/parser.rs` for Claude Code, Cursor, Copilot Chat/CLI, Gemini, Vibe, OpenCode, Pi, Hermes, Codex, and Windsurf; Codex accepts only a proven complete file result, Windsurf's documented path-only event creates no values, and unknown or before-only shapes fail open without values.
- [X] T020 [US1] Extend `tests/contract/test_hooks_contract.rs` with end-to-end local CLI replay that queries number/date nodes, source lines, stored file fingerprint, and canonical capture attribution after an exact full read, then proves path-only/partial/changed/pre/error and repeated callbacks add no source-backed nodes or associations.

**Checkpoint**: The Rust capture path is safe and fixture-tested. A verified automatic value-capture claim still requires the no-cost native evidence specified above; otherwise the client remains unverified or instruction-only for value capture.

---

## Phase 4: User Story 2 - Existing Installations Can Be Corrected Safely (Priority: P1)

**Goal**: Normal `rgt init` repairs recognized obsolete RGT entries without touching user-owned configuration or adding duplicate callbacks.

**Independent Test**: In a fake home containing a legacy RGT registration and an unrelated entry, normal initialization leaves one active current RGT callback, preserves the unrelated entry and a backup, and makes the second run a no-op; mixed or unknown ownership produces a diagnostic without editing that content.

### Tests for User Story 2

- [X] T021 [P] [US2] Add failing ownership and edit cases to `tests/unit/hook_editors.rs`: foreign absolute executables ending in `hook pre|post` are user-owned, mixed callback groups are ambiguous, and JSONC/TOML comments and unrelated entries survive a recognized RGT migration.
- [X] T022 [P] [US2] Add failing per-client current/legacy registration, backup, duplicate, `--force`, and ambiguous-ownership cases to `tests/contract/hook_installer.rs`, covering Claude, Cursor, Copilot Chat/CLI, Gemini, Vibe, OpenCode, Pi, Hermes, Codex, Windsurf, and the three guidance-only identifiers; test Codex trust notice, Windsurf hook-file precedence and Restricted Mode notice, and macOS/Linux/Windows path/command generation without a hosted runner.
- [X] T023 [P] [US2] Add failing fake-home normal-init and second-run tests to `tests/integration/test_hooks_installer.rs`, including an obsolete RGT artifact beside unrelated user content and a malformed artifact that must remain unchanged with an actionable failure; test one `rgt init -g` run detecting and configuring Claude Code, Cursor, Codex CLI, and Windsurf together, preserving unrelated settings and avoiding duplicates on rerun.

### Implementation for User Story 2

- [X] T024 [US2] Narrow RGT ownership recognition and in-place JSONC/TOML edits in `src/hooks/editor.rs` to verified RGT executable/marker signatures and whole-group safety; reject foreign commands and mixed RGT/user groups without modification.
- [X] T025 [US2] Update `src/hooks/installer.rs` and `src/hooks/paths.rs` to register and auto-migrate the documented Claude, Cursor, Copilot Chat/CLI, Gemini, Vibe, Codex, and Windsurf hook formats during normal `rgt init` and `rgt init -g`, with host-compatible missing-executable fallback where needed, backups, one active callback per intended event, and no-op reinitialization; never bypass Codex trust or write a shadowed Windsurf hook file.
- [X] T026 [US2] Update `src/hooks/installer.rs` and `src/hooks/glue/mod.rs` to install and auto-migrate documented OpenCode, Pi, and Hermes plugin entrypoints; preserve foreign plugin files and make uncertain ownership an explicit conflict.
- [X] T027 [US2] Update `src/hooks/installer.rs` and `src/hooks/paths.rs` so Cline/Roo, Antigravity, Kilo, and any hooked client lacking qualifying completed-read content, including documented Windsurf `post_read_code`, receive loadable instruction-only value-recording guidance without an automatic value-capture claim.
- [X] T028 [US2] Update `src/cli/init.rs` to report configured, migrated, already-current, and conflict outcomes with artifact/backup paths and recovery actions; retain exit 0 for success, 1 for expected configuration failure, and 2 for invalid input.

**Checkpoint**: Fake-home migration tests pass for each supported artifact form; no normal-init path overwrites user-owned content or multiplies callbacks.

---

## Phase 5: User Story 3 - Support Status Is Truthful (Priority: P2)

**Goal**: `rgt doctor` reports local registration/guidance health, while tracked support documentation reports versioned live verification separately.

**Independent Test**: For all 13 identifiers, a bare settings file, manual graph values, a stale/malformed registration, and instruction-only guidance produce distinct correct findings; no local result claims a host was live-verified.

### Tests for User Story 3

- [X] T029 [P] [US3] Extend `tests/contract/test_doctor_contract.rs` with failing per-surface checks for active/missing/obsolete/malformed/duplicate/unresolvable registrations, Codex trust and Windsurf active-path/Restricted Mode diagnostics where locally observable, instruction-only guidance, Copilot Chat versus CLI, and manually recorded values that must not imply capture.

### Implementation for User Story 3

- [X] T030 [US3] Implement local inspection in `src/hooks/registration.rs` of the expected active artifact, RGT entrypoint/executable, event phase, ownership, and duplicates; include Codex trust-review and Windsurf path-precedence/restricted-workspace caveats, use health states `active`, `missing`, `malformed`, `obsolete`, `ambiguous`, `trust-pending`, `inactive-workspace`, and `guidance-only` only when locally supportable, and never infer host invocation.
- [X] T031 [US3] Replace aggregate artifact-existence and nonempty-graph capture claims in `src/cli/doctor.rs` with per-surface local findings and recovery text; preserve exit 0 for healthy/advisory findings and 1 for errors.
- [X] T032 [US3] Update `docs/compatibility/support-matrix.md` with evidence-linked registration status, per-read-path value-capture tiers, named tool/result-form exclusions, and exact verified client version/platform combinations; classify before-only and Windsurf's documented path-only read path as instruction-only for value capture where guidance loads, and leave every unobserved path/version/platform combination unverified.
- [X] T033 [P] [US3] Correct automatic-capture and instruction-only claims in `README.md` and `AGENTS.md`, distinguishing client-delivered capture attribution from authorship and linking the tracked support matrix.
- [X] T034 [P] [US3] Correct support and initialization wording in `src/main.rs` and `src/cli/init.rs` so CLI help/output does not describe a written artifact or nonempty graph as verified automatic capture.

**Checkpoint**: Doctor and docs are honest independently: local registration health is inspectable; live verification requires matching evidence.

---

## Phase 6: User Story 4 - Maintainers Can Recheck Compatibility Locally (Priority: P2)

**Goal**: A maintainer can rerun deterministic compatibility checks offline and record optional no-cost native observations without promoting synthetic or comparative evidence.

**Independent Test**: On a fresh local workspace, replay redacted events and verify values, no-op protocol, malformed input, duplicates, and response preservation with zero network/model use; the report includes client version, platform, event, method, and graph result, or explicitly says unverified.

### Tests for User Story 4

- [X] T035 [US4] Create and register `tests/contract/test_harness_compatibility.rs` in `Cargo.toml` with positive and negative fixtures that validate every constraint in tracked `docs/compatibility/evidence.schema.json` using existing `serde_json`, `chrono`, and `regex` (required fields including RGT version, enums including content evidence, read-path slug pattern, date-time, surface pattern, minimum lengths, extra-property rejection, `redacted: true`); reserve `read_path_id: none` for guidance-only/non-read observations, require a claimed read-path ID to resolve to a documented tool/result form, loaded registration, `full-snapshot-match`, accepted neutral response, unchanged tool result, actual number/date values with node IDs, known client/RGT versions and platform, and paired `native-client` success plus no-op/error records for the same non-`none` read path before a verified automatic value-capture claim; include a negative case where success and no-op/error use different read-path IDs.
- [X] T036 [US4] Create `tests/fixtures/hooks/synthetic-events.json` as a matrix keyed by client surface and `read_path_id`: for every path advertised as automatic, include that host dialect's exact full-read success, empty, malformed, repeated, and no-op/error events; include Codex complete/truncated `PostToolUse`, Windsurf path-only `post_read_code`, partial/filtered/changed result, pre-only, failed, canceled, BOM, duplicate, missing-executable, and host-neutral-response cases where applicable. Label ineligible and unverified paths without treating them as automatic; use only synthetic values and paths, never label the file native evidence.
- [X] T037 [US4] Extend `tests/contract/test_harness_compatibility.rs` to replay the per-surface/read-path matrix in `tests/fixtures/hooks/synthetic-events.json` locally, fail when any read path advertised as automatic lacks its success, empty, malformed, repeated, or no-op/error case, and check graph values/attribution, no-op/error responses, duplicate behavior, and unchanged host result without network or model access; pair this with T022's per-client obsolete-registration coverage for SC-003.
- [X] T038 [P] [US4] Add deterministic 8 MiB input-cap, injected reader/clock for slow stdin that never reaches EOF, injected 800 ms plugin-child and direct command-hook process timeout, and missing-executable fail-open cases to `tests/contract/test_hooks_contract.rs`; assert bounded processing, no values, and a host-valid response without a wall-clock-dependent test gate.

### Implementation for User Story 4

- [X] T039 [US4] Add a manual end-to-end latency runner in `benches/bench_hook_capture.rs` and register it in `Cargo.toml`; measure invocation-to-neutral-response time, including launch and response framing, for 64 KiB text and 1 MiB binary-envelope success/no-op/malformed/repeated events plus an oversized input (over 8 MiB), slow stdin without EOF, and a stalled child on both direct command-hook and plugin launch paths. Check each representative and failure response against one second and confirm timeout cases add no values, without a flaky timing assertion in unit tests.
- [X] T040 [US4] Write `docs/compatibility/verification.md` with runnable local fixture, per-read-path matrix, fake-home global-init, format/lint/test, and manual latency commands for 64 KiB/1 MiB normal events plus oversized, no-EOF, and stalled-child cases on direct hooks and plugins; explain the 8 MiB cap, elapsed stdin deadline, 800 ms process bounds, no-cost native smoke steps, redaction, read-path/version/platform evidence, and the unverified outcome when a client is unavailable.
- [X] T041 [US4] Exercise only available no-cost native clients, including Codex/Windsurf when available, and add redacted observations with the RGT build version, read-path ID, loaded registration, actual number/date values and node IDs only for qualifying captures, source lines, accepted response, and unchanged result to `docs/compatibility/evidence/observations.json`; record Windsurf path-only hook invocation without value nodes separately. If no native observation is available, retain `[]`; if no qualifying full-content capture is observed, mark US1 live acceptance pending and state each unverified read path/version/platform reason in `docs/compatibility/support-matrix.md`.

**T041 outcome**: No isolated, no-cost native client session was available for verification. The evidence ledger remains `[]`, and US1 live acceptance remains pending as required by the task's fallback condition.
- [X] T042 [US4] Run the local replay, evidence, and normal/oversized/stalled manual latency checks from `docs/compatibility/verification.md`; record exact commands, final exit codes, measured invocation-to-response times, read-path/client-version/platform coverage, and any unverified cases in that file; do not run GitHub Actions or a paid session.

**Checkpoint**: Routine checks are locally reproducible; evidence is labeled by method and cannot silently turn comparative or fixture-only results into a verified claim.

---

## Phase 7: Polish & Cross-Cutting Concerns

**Purpose**: Close documentation, privacy, performance, and local quality gates across the completed stories.

- [X] T043 Audit `README.md`, `AGENTS.md`, `src/main.rs`, and `docs/compatibility/support-matrix.md` against the final registration/capture behavior and evidence; remove any unsupported automatic-capture claim or outdated hook path.
- [X] T044 Check `tests/fixtures/hooks/synthetic-events.json` and `docs/compatibility/evidence/observations.json` for private paths, source contents, credentials, and session IDs; keep only redacted synthetic or consented no-cost native observations.
- [X] T045 Run local `rtk proxy cargo fmt --all -- --check`, `rtk proxy cargo clippy --all-targets -- -D warnings`, targeted tests, and the full `rtk proxy cargo test`; record final process exits and any limitation in `docs/compatibility/verification.md` without running CI/CD or GitHub workflows.
- [X] T046 Compare all 16 functional requirements and eight success criteria in `specs/044-verify-harness-compatibility/spec.md` to tested behavior and evidence; record gaps or unverified read-path/client-version/platform combinations in `docs/compatibility/support-matrix.md`, and do not mark US1 or the feature complete without at least one native-client full-snapshot success plus no-op/error pair for the same read path, version, and platform.
- [X] T047 [P] Add `tests/contract/test_cli_version.rs` and register it in `Cargo.toml`; verify `rgt --version` and `rgt version` both print the exact package version and exit successfully.
- [X] T048 Add the Clap version flag and `version` subcommand in `src/main.rs`, source both from `CARGO_PKG_VERSION`, add `rgt_version` to the compatibility evidence schema and ledger instructions, and document the command.

---

## Dependencies & Execution Order

```text
Phase 1 Setup -> Phase 2 Foundation -> US1 capture safety (P1)
                              |-------> US2 safe migration (P1)
                              |-------> US3 truthful status (P2)
                              |-------> US4 local verification (P2)
US1 host contract notes ------> US2 host-specific registration changes
US2 registration inspection --> US3 final doctor checks
US1 + US2 + US3 + US4 -------> Phase 7 polish and full local validation
```

- **US1** starts after Phase 2. Its parser, response, PDF, and graph tests may be written in parallel, as may the distinct client contract notes. Code follows failing tests and evidence-backed contracts.
- **US2** starts after Phase 2, with host-specific installer changes waiting for the relevant US1 contract note. It remains independently testable in a fake home and does not require US1's live smoke to run.
- **US3** starts after Phase 2; final doctor behavior uses the shared registration model and benefits from US2's current registration forms. Its doctor tests can be written independently.
- **US4** starts after Phase 2; replay and evidence validation use the contracts, while final native observations and report depend on the integrated stories. A client that cannot be exercised at no cost remains unverified.
- **Phase 7** follows the desired story checkpoints. Local tests and review are required before any later merge decision; this task list does not authorize a hosted workflow.

## Parallel Execution Examples

- **US1**: T006–T009 touch separate test files; T010–T014 touch separate client notes. After they finish, T015, T016, T017, T018, and T019 are sequenced where they share parser/hook/glue files.
- **US2**: T021–T023 create independent unit, contract, and fake-home tests. T024–T028 then change shared editor/installer/init code in order.
- **US3**: T029 doctor tests can begin while US2 migration work proceeds. After local health semantics are fixed, T033 documentation and T034 CLI wording touch distinct files.
- **US4**: T038 timeout/failure tests touch `tests/contract/test_hooks_contract.rs` while T035–T037 build the separate evidence/replay target. Manual benchmark and guide work follow their prerequisites.

## Implementation Strategy

1. **MVP**: Complete Setup, Foundation, and US1. Keep only surfaces with demonstrated completed-read events eligible for automatic claims. A real-client success plus no-op/error pair for at least one no-cost surface is required to call US1 live-verified; otherwise deliver safe capture code and an explicit unverified checkpoint.
2. **Incremental delivery**: Add US2 migration, US3 diagnostics/docs, and US4 local evidence as separate reviewable increments. Each story has its own tests and can be checked without a paid host session.
3. **Final validation**: Use local Cargo checks, redacted evidence, and the manual latency runner. Do not run hosted CI/CD or paid model calls; do not claim unsupported version/platform coverage.

## Notes

- `[P]` applies only to tasks with disjoint output files and no unfinished prerequisite in the same phase.
- Exact client registration and response forms must come from documented or observed host behavior. RTK remains comparative evidence.
- `.gitignore` allows this feature's `specs/044-verify-harness-compatibility/` artifacts to be tracked. Include them with product documentation, fixtures, and evidence in a later compliant changeset.

## Phase 8: Convergence

- [X] T049 CRITICAL: Correct the `rgt hook` table entry and capture description in `README.md`, and the public `handle_passive_hook_event` documentation in `src/hooks/mod.rs`, so they state that only qualifying completed reads create values and that guidance-only surfaces have no automatic hook capture, per Constitution X and FR-009 (contradicts).
- [X] T050 CRITICAL: Replace the generated direct-hook command's unescaped executable-path quoting in `src/hooks/installer.rs` with POSIX- and Windows-safe command construction; add contract cases for paths containing spaces and shell expansion characters, and prove fail-open behavior when the executable is missing, per Constitution Cross-Platform Parity and FR-005/FR-008 (partial).
- [X] T051 Make `rgt doctor` inspect the exact active host event key, callback group, agent identity, plugin entrypoint, and underlying RGT executable rather than accepting any hook-subtree command or plugin marker; add false-positive cases for a post command registered under a pre event, a marker-only plugin, and a Windows wrapper whose RGT target is missing, per FR-010 and SC-004 (partial).
- [X] T052 Validate every tracked record in `docs/compatibility/evidence/observations.json` against the evidence schema and semantic promotion rules; bind each `read_path_id` to its `surface_id` and canonical agent, require no-op/error records to have no value attribution, and allow a verified claim only from individually valid native success and no-op/error records with the same path/version/platform tuple, per FR-011 and SC-002 (partial).
- [X] T053 Reduce and remeasure cold first-invocation latency on the plugin launch path so 64 KiB text, 1 MiB binary-envelope, oversized, and stalled cases meet the one-second invocation-to-response bound; retain the 800 ms child deadline and record both cold and repeated local measurements without a wall-clock-dependent unit test, per FR-015 and SC-006 (partial).
- [X] T054 Complete US1 live acceptance with a no-cost native client read-path success and no-op/error pair on the same RGT/client version and platform, confirming loaded registration, exact source values and lines, capture attribution, accepted neutral response, and unchanged tool result in redacted evidence; if no such client event is available without a paid session, keep the support tier and acceptance status explicitly unverified, per US1/AC1 and FR-002 (partial).

**T054 outcome (no-cost fallback)**: No qualifying native event could be generated in an isolated no-cost client session. No client was launched, the evidence ledger remains empty, and every native read path stays unverified. This completes T054's fallback; US1 live acceptance remains pending until native success and no-op/error evidence can be obtained without a paid session.

## Phase 9: Convergence

- [X] T055 Bound the generated Windows PowerShell direct-hook wrapper in `src/hooks/installer.rs` from invocation, including stdin that never reaches EOF, child termination, and host-valid fail-open output; add deterministic generated-command contract coverage for stalled input and child behavior, per FR-015 (partial).
- [X] T056 Replace the unconditional rejection of a native success/no-op pair in `tests/contract/test_harness_compatibility.rs` with validation that every published verified automatic-capture claim has individually valid native success and no-op/error observations for the same read path, client version, RGT version, and platform; allow a valid pair in the tracked ledger, per FR-011 and SC-002 (contradicts).
- [X] T057 Align Vibe's installed `pre_tool` command in `src/hooks/installer.rs` with the before-event identity expected by `src/hooks/registration.rs`, retain instruction-only value capture, and add a fake-home init-then-doctor regression case showing the fresh registration is current and idempotent, per FR-008 and FR-010 (partial).
- [X] T058 Extend `benches/bench_hook_capture.rs` and `docs/compatibility/verification.md` with plugin no-op and malformed timing cases and a stalled direct command-hook process case, checking sub-second fail-open responses and zero new values locally without a wall-clock unit-test gate, per T039 and FR-015 (partial).

**T055–T058 outcome**: All four convergence tasks are implemented and locally validated. Windows PowerShell behavior has generated-command contract coverage only; no Windows runtime was available in this macOS run.

## Phase 10: Convergence

- [X] T059 CRITICAL: Correct `benches/bench_hook_capture.rs` so plugin success and repeated cases run against an isolated graph, check the generated Pi extension's neutral return, and assert actual new-node and capture-association outcomes instead of accepting a root previously created by the direct hook; update `docs/compatibility/verification.md` to label Pi's unsupported value-capture path and measured graph result accurately, per Constitution X and T039 (contradicts).
- [X] T060 CRITICAL: Fix the manual benchmark command in `specs/044-verify-harness-compatibility/quickstart.md` to provide the required built RGT executable argument, and verify that the documented local command runs as written, per Constitution X and plan: quickstart (contradicts).
- [X] T061 Add distinct 1 MiB binary-envelope successful-result, no-op, malformed, and repeated event cases on the generated plugin launch path in `benches/bench_hook_capture.rs`; check each case's sub-second neutral response and zero or supported graph effect, then record the measured cases and Pi's unverified capture limit in `docs/compatibility/verification.md`, per T039 and FR-015 (partial).

**T059–T061 outcome**: The plugin benchmark now isolates graph state, checks neutral return and input immutability, and verifies node and capture-association counts. The local format check, binary build, and manual latency runner all exited 0. No test suite, GitHub workflow, or paid native-client session was run. T054 remains open because no qualifying no-cost native observation is available.

## Phase 11: Convergence

- [X] T062 Make `benches/bench_hook_capture.rs` exit nonzero with an explicit incomplete result when Node is unavailable or a required direct/plugin stalled-path case is skipped; exercise an equivalent local case on Windows when possible and document the coverage outcome, per T039 and FR-015 (partial).
- [X] T063 Read the already initialized project store without creating a replacement, propagate store-open and node/capture-association query errors instead of converting them to zero, and fail every affected direct or plugin verdict with a diagnostic, per T059 and T039 (partial).
- [X] T064 Make the stalled Pi child in `benches/bench_hook_capture.rs` record that it started and verify its termination after the plugin callback, so an immediate spawn failure cannot pass as a successful timeout; retain the neutral-response and zero-value checks without a lower-bound wall-clock unit-test gate, per T039 and FR-015 (partial).

**T062–T064 outcome**: The runner fails closed for missing Node.js, unsupported stalled-child coverage, and store/query errors. Graph counts use read-only access to the existing project database. On macOS arm64, the manual benchmark passed, and both stalled child stubs recorded startup and verified termination. Windows runtime coverage was unavailable; unsupported non-Unix stalled cases report `INCOMPLETE` and cause a nonzero exit.

## Phase 12: Convergence

- [X] T065 Make each stalled Pi callback in `benches/bench_hook_capture.rs` record a distinct child start and PID, verify that both children terminated, and add a deterministic negative check proving a missing second launch cannot pass; retain neutral-response and zero-graph checks without a lower-bound timing gate, per T064 and T039 (partial).
- [X] T066 Exercise the generated Windows PowerShell direct-hook command in `benches/bench_hook_capture.rs` instead of invoking `rgt hook` directly on Windows; run a Windows-compatible stalled-child case when a local runtime is available, or report every unexercised direct-wrapper case as `INCOMPLETE` with a nonzero exit, and record the coverage result in `docs/compatibility/verification.md`, per T039 and FR-015 (partial).

**T065–T066 outcome**: The Mac formatting check and benchmark compile passed. The benchmark contains per-callback Pi child records and invokes the generated PowerShell command in its Windows direct path. The Windows cross-target check stopped before compiling RGT because `x86_64-w64-mingw32-gcc` is unavailable, and no Windows runtime was available; those limits are recorded in the verification guide.

## Phase 13: Convergence

- [X] T067 Run the current manual runner in `benches/bench_hook_capture.rs` locally with the documented built RGT binary; confirm its normal, oversized, no-EOF, and stalled cases pass, including separate Pi child starts, PID records, terminations, missing-second rejection, neutral responses, zero timeout graph effects, and per-callback latency bounds; record the exact command, exit status, and measured results in `docs/compatibility/verification.md` while keeping unavailable Windows runtime coverage explicit, per T039, T065, FR-015, and SC-006 (partial).

**T067 outcome**: Local build and manual runner both exited 0. Every emitted benchmark row passed; the first and repeated Pi stall callbacks recorded separate start/PID evidence and verified termination, and the one-child negative check was rejected. Measurements and the Windows runtime limitation are recorded in `docs/compatibility/verification.md`.

## Phase 14: Convergence

- [X] T068 Complete the US1 live acceptance gate with a demonstrably no-cost native client session: record a successful full-content read and a no-op/error event for the same read path, client version, RGT version, and platform; confirm loaded registration, exact source values and lines, capture attribution, accepted neutral response, and unchanged tool result; add redacted native observations to `docs/compatibility/evidence/observations.json` and promote only the proven tuple in `docs/compatibility/support-matrix.md`, per US1/AC1, FR-002, SC-002, and plan: implementation sequence 5.

## Phase 15: Convergence

- [X] T069 CRITICAL: Correct the Quick Start comment in `README.md` that says Codex and Windsurf use rules files; describe their native hook registrations and separate their guidance from Cline, Antigravity, and Kilo rules, per Constitution X and FR-001 (contradicts).
- [X] T070 Reject explicitly failed or canceled OpenCode `tool.execute.after` read payloads in `src/hooks/parser.rs`, including root/result outcome flags that can accompany otherwise complete display metadata; add focused parser and hook replay cases proving no values or capture associations are written, per FR-004 and US1/AC2 (partial).
- [X] T071 Audit surfaces labeled instruction-only or instruction-assisted without a verified completed-read path, especially Cursor, Gemini CLI, and Copilot Chat; install ownership-safe RGT value-recording guidance at each documented loadable client location, or correct the support classification where no loadable guidance contract exists; add installer/doctor tests for the resulting per-surface status, per FR-009, US3/AC4, and T027 (partial).
- [X] T072 Bind every verified support-matrix row in `tests/contract/test_harness_compatibility.rs` to its named client surface, canonical agent, documented read path, and native evidence pair, and add a negative case that moves a valid path into another client's row while retaining its evidence, per FR-002, SC-002, and T052 (partial).
- [X] T073 Make the OpenCode `empty` fixture and replay use a genuinely empty source file with a successful complete read and no added value nodes; keep changed-snapshot rejection as a separate case, per SC-003 and T036–T037 (partial).
- [X] T074 Extend the local manual latency runner and verification report with 64 KiB and 1 MiB successful OpenCode plugin reads through `opencode-read-display-text`, checking response framing, unchanged result, expected graph values and attribution, and the one-second budget without a hosted workflow or paid model call, per FR-015, SC-006, and T039 (partial).

## Phase 16: Convergence

- [X] T075 Reject explicit top-level `error` and root/result `isError` failure indicators in the shared completed-read normalizer in `src/hooks/parser.rs` before value eligibility; add focused parser and hook replay cases using otherwise complete, snapshot-matching Claude Code and Codex `Read` results that prove failed events add neither nodes nor capture associations, per FR-004 and US1/AC2 (partial).
- [X] T076 Inspect RGT-owned guidance content and its documented local load path separately from native hook health for instruction-only and instruction-assisted surfaces in `src/hooks/registration.rs` and `src/cli/doctor.rs`; distinguish missing, incomplete, or unowned guidance from usable local guidance without promoting live capture, and add installer/doctor cases for guidance-only and dual hook-plus-guidance clients, per FR-009, FR-010, and plan: implementation sequence 4 (partial).

## Phase 17: Convergence

- [X] T077 CRITICAL: Make `rgt doctor` report missing, incomplete, unowned, managed, and hand-maintained guidance distinctly for instruction-only surfaces as well as hook-plus-guidance surfaces; add CLI output and exit-code tests for Cline/Roo and another guidance-only client, and keep `docs/compatibility/verification.md` and `README.md` aligned with the actual diagnostic, per Constitution X, FR-010, and T076 (contradicts).
- [X] T078 Establish a loadable instruction path for Pi and Hermes and install ownership-safe value-recording guidance with local inspector, installer, doctor, and documentation coverage; if a client has no supported guidance path, mark its read-capture tier unsupported and state that limit instead of implying usable instruction-only capture, per FR-009 and T027 (missing).
- [X] T079 Validate Cursor's `.cursor/rules/rgt.mdc` frontmatter during normal `rgt init` and `rgt init -g`; repair a clearly RGT-owned rule so `alwaysApply: true` is inside valid frontmatter, or fail with an actionable ownership conflict before reporting a usable guidance installation, while preserving unrelated content and covering reruns in installer/doctor tests, per FR-009 and US2/AC3 (partial).
- [X] T080 Verify whether Vibe loads `~/.vibe/prompts/rgt.md` and how a user activates that guidance; document and test the supported local load path, or correct its instruction-only support classification and installer behavior when that path cannot be established, per FR-009 (partial).
- [X] T081 Make `test_quickstart_cargo_package_clean` in `tests/integration/test_distribution_quickstart.rs` a reliable bounded local check that does not depend on network availability; rerun the complete local `rtk proxy cargo test` without exclusions and record its final exit and the earlier packaging limit in `docs/compatibility/verification.md`, without running hosted workflows, per T045 (partial).

## Phase 18: Convergence

- [X] T082 CRITICAL: Make `repair_cursor_frontmatter` in `src/hooks/installer.rs` accept valid LF and CRLF frontmatter in a clearly RGT-owned `.cursor/rules/rgt.mdc`, repair `alwaysApply: false` without discarding unrelated content or the existing backup behavior, and add normal `rgt init`, `rgt init -g`, doctor, and idempotent-rerun tests for CRLF rules, per Constitution Cross-Platform Parity, T079, and US2/AC1 (partial).
- [X] T083 CRITICAL: Make the default local Cargo suite deterministic by replacing the live `cargo run -- update --check` assertions in `tests/integration/test_distribution_quickstart.rs` and `tests/integration/test_distribution_performance.rs` with local mocked or injected update behavior, moving wall-clock performance thresholds and nested release-build checks out of the default test gate, and updating `docs/compatibility/verification.md` with the reproducible commands and limits, per Constitution IX and T045 (contradicts).
- [X] T084 Validate the matching project or user `.hermes/config.toml` `plugins.enabled` entry in `src/hooks/registration.rs` before `rgt doctor` calls a Hermes plugin registration active; report missing, malformed, or disabled enablement with an actionable diagnostic and add fake-home doctor cases that preserve plugin-file and guidance health as separate findings, per FR-010, SC-004, and T030 (partial).

## Phase 19: Convergence

- [X] T085 HIGH: Repair Cursor's RGT-owned `.cursor/rules/rgt.mdc` frontmatter by editing only the actual `alwaysApply` key line, preserving unrelated LF/CRLF header text; make `rgt doctor` reject missing or conflicting duplicate `alwaysApply` keys rather than treating any `alwaysApply: true` line as sufficient; add normal/global init, backup, rerun, and doctor cases with a description containing `alwaysApply: false` before the setting and with contradictory settings, per FR-009, US2/AC1, and T082 (partial).
- [X] T086 HIGH: Make Hermes normal `rgt init` and `rgt init -g` handle a valid inline `plugins = { enabled = ["rgt"] }` config that `rgt doctor` already calls active without an edit or duplicate, and report a safe conflict for malformed enablement arrays instead of reporting an already-current registration; preserve unrelated TOML and add project/user fake-home init-to-doctor parity tests, per FR-008, US2/AC2, and T084 (partial).
- [X] T087 Record the completed Phase 18 local format, Clippy, full Cargo test, and diff-check exits in `docs/compatibility/verification.md`, distinguish these deterministic checks from optional release-build and latency measurements, and state that no hosted workflow or native-client verification was run in that pass, per T045 (partial).
