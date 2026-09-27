# Research: Harness Compatibility Verification

**Date**: 2026-09-26
**Spec**: [spec.md](spec.md)
**Baseline**: local RGT `develop` at `5bfff8597e283532f51f4ddb22c675e70808c1d7`; comparative RTK commit `a89a31494670fcec8ffa20d939dd94c64bd998fb` in [evidence.md](evidence.md).

The comparative matrix identifies likely contract mismatches. It is not evidence that a client loads RGT. No live RGT client integration is verified in this baseline. Native client events and vendor documentation must be recorded per client surface, version, and platform before a support claim is promoted.

## Decisions

### 1. Gate value recording on a completed successful read

- **Decision**: Normalize each host event into a phase, outcome evidence, source path, and completed result content. A pre event never records values. An explicit success status proves completion, but a path-only success cannot justify value nodes. A source-backed root requires full source bytes and source line positions reconstructed from the completed result and matched to the current file snapshot before hashing. Partial, filtered, truncated, changed-snapshot, or unmappable results produce no roots. Keep PDF capture subject to the same gate.
- **Rationale**: `src/hooks/mod.rs` currently ignores its `event_type` for ordinary capture and falls back to disk reads; `src/hooks/parser.rs` carries only path/content. A failed or canceled read, or a later file change, can therefore produce misleading value nodes.
- **Alternatives considered**: Capture from pre events or path-only disk reads. Both were rejected by the clarified provenance rule. Adding a separate pre-read observation node was rejected because it changes the graph model and is out of scope.

### 2. Treat support as a versioned evidence claim, not an installer property

- **Decision**: Maintain a surface-level support matrix for the 13 canonical agent identifiers, with Copilot Chat/CLI and Cline/Roo checked separately, and identify each read tool and accepted result form with a stable read-path ID. Add native hooks for Codex and Windsurf under the constitution. Promote to **verified automatic value capture** only for a read-path/version/platform combination with real-client successful-read and same-path no-op/error observations that include full source content. List other paths and result forms as excluded or unverified; when no suitable completed content exists, provide instruction-only read capture where the client loads guidance, even if a native hook is configured.
- **Rationale**: `src/hooks/installer.rs` currently labels eight agents capture-capable based on an emitted artifact, while [evidence.md](evidence.md) has no live RGT client check. [Codex's official hook documentation](https://developers.openai.com/codex/hooks) describes `PostToolUse` with `tool_response`, but project hooks require trust review. [Windsurf's Cascade hook documentation](https://docs.windsurf.com/windsurf/cascade/hooks) describes `post_read_code` after a successful read with a `file_path` only; this proves timing, not source bytes. RTK source is comparative evidence only.
- **Alternatives considered**: Promote from generated files, parser fixtures, or RTK behavior. These do not show that RGT runs in the named host.

### 3. Keep host-specific contracts at the boundary

- **Decision**: Use small Rust adapters to interpret each observed host event and emit a common capture decision. Keep TypeScript/Python client glue limited to registration, field forwarding, subprocess invocation, and required host response framing. Validate registration, phase, input, and output independently for each surface; do not assume a payload from a different host.
- **Rationale**: The pinned RTK comparison points to different Copilot, Gemini, Vibe, OpenCode, Pi, and Hermes registration or entrypoint shapes and Cursor response behavior. Codex and Windsurf need native hook adapters that keep their distinct response semantics. Current RGT parsing assumes several synthetic dialects. The existing Rust/CLI architecture and constitution favor one business-logic path.
- **Alternatives considered**: One generic JSON parser, or business rules in each plugin. The former hides host differences; the latter duplicates provenance decisions and violates the thin-glue rule. No new runtime dependency is needed.

### 4. Migrate only registrations proven to be RGT-owned

- **Decision**: Normal `rgt init` replaces known obsolete RGT registrations, writes a backup before changing an existing artifact, and becomes a no-op on re-run. Match ownership using a narrow RGT executable/marker/signature and the expected host structure. Leave mixed or ambiguous groups and user-owned files untouched, with a specific diagnostic and recovery action. Verify the resulting single active callback per intended event.
- **Rationale**: `src/hooks/editor.rs` already supports some in-place JSONC/TOML changes, but its command match can treat a foreign absolute executable followed by `hook pre|post` as RGT-owned. Some text/plugin writers skip any existing file without `--force`; legacy cleanup can also remove files too broadly.
- **Alternatives considered**: Blanket overwrite under `--force`, or manual migration for every install. Blanket overwrite risks user data; manual migration contradicts the clarified normal-init behavior.

### 5. Make `rgt doctor` a local registration inspector

- **Decision**: Inspect the actual RGT registration or guidance per surface and report local status with an actionable reason. Do not infer live capture from a settings file, nonempty graph, or historical evidence. Publish live verification separately in versioned support evidence.
- **Rationale**: `src/cli/doctor.rs` currently checks whether any known artifact exists and calls a nonempty graph “Values are being captured,” even when the graph may contain manually recorded values.
- **Alternatives considered**: Store last smoke result in the local database or run a host session from `doctor`. The former can become stale; the latter may require a paid call and was not selected in clarification.

### 6. Preserve fail-open behavior with host-valid output and bounded work

- **Decision**: A malformed, empty, BOM-prefixed, or uncapturable event returns success to the host with that host's required no-op response. Cap stdin at 8 MiB and impose an elapsed deadline even on slow input without EOF; bound plugin children and direct command-hook processes by 800 ms from invocation, or use a documented and tested host timeout/fail-open outcome. Benchmark 64 KiB text and 1 MiB binary-envelope events within the one-second response budget. A missing RGT executable needs a host-compatible fallback launcher or a documented and tested host fail-open outcome before automatic support can be claimed. Never modify the client's command, result, approval, or error outcome.
- **Rationale**: Cursor may require structured JSON even for no-op; current RGT emits empty stdout. `src/hooks/mod.rs` reads unbounded stdin, and OpenCode/Pi glue currently uses unbounded synchronous subprocesses; Hermes uses a five-second timeout. Existing graph deadline code does not bound hooks.
- **Alternatives considered**: One empty response and unbounded `spawnSync` for all hosts. These can violate host protocols or the passive latency budget.

### 7. Use deterministic fixtures plus optional no-cost live smoke checks

- **Decision**: Check redacted native event fixtures, installer output, migration, parser outcome, response protocol, graph attribution, deduplication, and timing locally. Record a real-client smoke observation only when the installed host can generate it without a paid request. An unavailable client remains unverified, never a skipped pass; US1 live acceptance remains pending if no such observation exists.
- **Rationale**: Tests in `tests/contract/hook_installer.rs` and `tests/unit/hook_parsers.rs` prove RGT-generated artifacts and synthetic parsing, not host acceptance. The user has ruled out GitHub workflows and paid model calls for routine validation.
- **Alternatives considered**: Docker for every harness or a hosted matrix. GUI and proprietary clients cannot all be faithfully exercised in Docker, and hosted/paid validation conflicts with the current constraint.

### 8. Reuse graph identity and attribution storage

- **Decision**: Keep existing root-node identity, tracked values, and `capture_associations` schema. Repeated callbacks for the same observed value should reuse the root node and agent association; an event from a distinct client may add its own association. Add explicit tests for these cases and avoid unnecessary writes on identical repeated events when practical.
- **Rationale**: `src/store/queries.rs` already upserts nodes and uses a unique node/agent association. The feature changes capture eligibility and evidence, not value or derivation semantics.
- **Alternatives considered**: Add event nodes or a graph schema migration. Neither is needed for the clarified requirements.

## Unresolved external facts and their handling

Exact accepted registration schemas, callback phases, payload fields, and no-op responses for some host versions are not yet demonstrated by a live RGT run. This is a **verification condition**, not permission to assume compatibility. Implementation must use the client/version evidence ledger in [contracts/compatibility-evidence.schema.json](contracts/compatibility-evidence.schema.json); where completion cannot be proven or a no-cost live check cannot run, the published tier stays instruction-only or unverified. The design requires no further product choice before tasks are generated.
