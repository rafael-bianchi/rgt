# Research: Turtle Provenance Export

**Date**: 2026-09-24

**Input**: [spec.md](spec.md), current Rust/store/hook code, and the earlier [PROV-O assessment](../../.specify/assessments/prov-o-turtle-export/research.md).

## Decision 1: Project-scoped identities

**Decision**: Persist a random 128-bit `rdf_project_id` once in the existing `project_settings` table, using SQLite's available `randomblob(16)` source. Use a project-scoped HTTPS IRI namespace such as `https://github.com/rafael-bianchi/rgt/iri/v1/project/{token}/node/{encoded-local-id}`. Keep the original node ID as an exported literal. Create the token with an atomic insert-if-absent and reread it, including when opening a legacy store without one. A copied store retains the same identity; independently initialized stores get distinct tokens.

**Rationale**: Root IDs hash the caller-supplied path spelling, line, occurrence, and value, while derived IDs hash ordered parents, operation, and value. Neither includes project identity; the expression is absent from derived IDs. Matching local IDs therefore cannot prove cross-project equivalence. An opaque token stays stable if a project moves and does not reveal its directory (`src/types/node.rs`, `src/cli/record.rs`, `src/store/queries.rs`).

**Alternatives considered**: Bare node IDs conflate independent projects. A namespace based on the current path changes after a move and leaks location. A new token per export breaks repeat identity. Rewriting node IDs would break stored lineage. A new schema column is unnecessary because `project_settings` already stores durable project values.

## Decision 2: Source location and path privacy

**Decision**: Identify source documents with project-scoped IRIs based on their stored row IDs, never with raw paths. Emit a relative path only when an in-project source can be established safely. Use only the path-free source IRI for outside, missing, or unclassifiable paths. An explicit `--include-absolute-paths` option for Turtle includes available absolute source-document paths as literals. Neither project nor source identifiers embed local paths. This privacy rule governs RGT-generated source locations, not preserved user-supplied expression text.

**Rationale**: `source_documents.file_path` is stored verbatim and may be absolute, relative, contain `..`, or pass through a symlink. A raw export could expose usernames and local directories. `source_doc_id` is nullable on roots, so missing sources must not be fabricated (`src/store/schema.rs`, `src/store/queries.rs`).

**Alternatives considered**: Exporting stored paths unchanged violates the clarified privacy default. Omitting all source locations loses useful in-project provenance. Encoding an absolute path in an IRI still discloses it. A hash of a path can be guessed and is less directly interpretable than the existing opaque row identity.

## Decision 3: Derivation activity evidence

**Decision**: Preserve every supported parent-child relationship as child `prov:wasDerivedFrom` parent. Reconstruct one derivation activity for a child only when its edge rows, sorted by insertion ID, have one consistent operation and expression, reference distinct existing parents, and reproduce the stored child ID using RGT's ordered-parent ID function (full 128-bit or historical 48-bit prefix). Otherwise omit the activity and any consolidated operation/expression claim, and warn on stderr while the Turtle export succeeds. Make future derived node and edge writes transactional so a failed write cannot leave a partial group.

**Rationale**: The store has edge rows, not an activity row; a multi-parent calculation is several edges. `derive` currently writes the node then each edge without a transaction, and duplicate edges are ignored. Matching edge labels alone cannot prove a complete parent group. Checking the derived ID gives a conservative completeness test for recorded data. It does not prove an exact historical event log, especially for old shorter IDs or a later repeated derivation whose expression was not retained (`src/store/schema.rs`, `src/store/queries.rs`, `src/cli/derive.rs`, `src/types/node.rs`).

**Alternatives considered**: Inferring an activity whenever labels match can overstate partial writes. Failing the whole export conflicts with the clarified partial-output rule. Adding durable activity and usage tables would support richer event history but expands the schema and write path beyond the one-activity-per-recorded-child contract. It remains a future option if a consumer needs exact derivation events; validated edge insertion order is sufficient to publish the recorded operand order.

## Decision 4: Durable capture associations and agent scope

**Decision**: Add `capture_associations(node_id, agent_name)` with a pair primary key and node foreign key with cascade deletion. Bump the store schema version from 1 to 2 with an explicit, atomic migration. Normalize `--agent` to the registry's canonical name before hook parsing; omitted agent retains the existing Claude Code dialect. Insert the pair in the same transaction as each successfully captured text or PDF node, using conflict-ignore for repeat captures. Export one capture activity per pair and a canonical software-agent entity. Manual records and unknown/rules-only agents get no association.

**Rationale**: The current `tracked_nodes` and `source_documents` tables have no agent field, so a serializer alone cannot recover attribution. A node may have several capturing agents, making a single node column unsuitable. Five of the 13 configured integrations are rules files without programmatic events; eight currently have hook or plugin capture paths. The hook parser accepts only those event dialects (`src/hooks/installer.rs`, `src/hooks/parser.rs`, `src/hooks/mod.rs`, `src/store/schema.rs`).

**Alternatives considered**: A `prov:Agent` triple without persisted evidence would fabricate or lose attribution. A project setting keyed by node/agent lacks referential integrity and clean garbage collection. An agent column overwrites earlier agents. An event log for every repeated read conflicts with the clarified summary rule and grows without bound. Broadening five rules-only integrations to hooks is a separate feature.

## Decision 5: Turtle mapping and writer

**Decision**: Use a small Rust Turtle serializer with fixed `prov:`, `xsd:`, and documented `rgt:` terms. A recorded value is `prov:Entity`; a source document is also an entity; a supported derivation or capture is `prov:Activity`; a canonical coding tool is `prov:Agent`. For a derivation, the activity `prov:used` each parent, the child `prov:wasGeneratedBy` the activity, and the child `prov:wasDerivedFrom` each parent. Add `prov:qualifiedUsage` with an RGT operand index when edge order has passed the derived-ID check, so expression variable order is not lost. A capture activity `prov:used` the recorded value and `prov:wasAssociatedWith` the tool. Preserve number, date, and duration as valid typed literals; do not label RGT's human-readable duration string as `xsd:duration`.

**Rationale**: This follows the direction of PROV-O relationships verified in the earlier assessment. It interoperates without a runtime triple store. Stable project-scoped object IRIs and canonical tool IRIs allow ordinary RDF graph union while keeping independent project values distinct. Escaping must cover quotes, backslashes, control characters, UTF-8, and percent-encoded IRI segments (`src/types/value.rs`, `.specify/assessments/prov-o-turtle-export/research.md`).

**Alternatives considered**: DOT/Mermaid/text are not RDF exchange formats. JSON-LD and N-Triples are additional public formats outside the requested scope. A runtime RDF engine adds weight without a demonstrated need. Raw interpolation of paths or expressions would produce invalid or privacy-leaking Turtle.

## Decision 6: Consistent snapshot, deterministic output, and failure behavior

**Decision**: Bound Turtle export to 10,000 recorded values, 30,000 derivation edges, and 64 MiB of rendered UTF-8 Turtle. Referenced sources cannot exceed root count, and one association per value per eight eligible agents bounds capture pairs, so separate source and capture quotas would have unreachable one-over-limit cases. Apply one command-wide deadline of one second from CLI entry through final stdout writing, with bounded SQLite lock waiting. Check value and edge limits through bounded `limit + 1` reads inside the same consistent SQLite snapshot used for full loading; validate source references and canonical capture pairs while loading. Do not use current per-node graph queries or getters that substitute default values for malformed rows. Sort all nodes, edges, agents, and emitted facts. Validate and buffer rendering before writing stdout, aborting immediately when the next bytes would exceed 64 MiB. Perform final stdout writing on a worker thread and wait on its completion channel for the remaining deadline; on timeout, write the error to stderr and terminate the process nonzero without waiting for a blocked writer. Send ambiguity warnings to stderr only. Preserve existing graph formats and their current unknown-format fallback. Treat quota excess, pre-write deadline expiry, malformed rows, failed queries, and invalid project tokens as hard errors with nonzero exit and zero stdout before any write. A timed-out final write may leave incomplete bytes but must never report success.

**Rationale**: Current `graph` output is incremental, its edge read errors are ignored, and node/edge list queries have no ordering. Some getters decode malformed dates/numbers by substituting current time or zero; that would create false provenance and nondeterministic exports. A bulk snapshot avoids per-node query overhead at 10,000 values. Explicit row and byte limits plus a command-wide deadline address the constitution's sub-second CLI bound even for oversized graphs or stalled output (`src/cli/graph.rs`, `src/store/queries.rs`). A same-thread clock check cannot stop an OS write blocked on a full pipe, so the worker/channel boundary and process exit are required on every target OS.

**Alternatives considered**: Unbounded whole-graph export cannot satisfy the constitution's sub-second CLI bound. Streaming triples while querying can leave a seemingly complete prefix on later failure. Reusing permissive getters would fabricate data. Sorting only one class of rows would leave output order unstable. An OS-level failure during the final stdout write cannot be prevented, so it must return an error rather than claim success.

## Decision 7: Validation and delivery gate

**Decision**: Use deterministic CLI, migration, and hook contract tests plus an independent Turtle parser in test code. Include fresh/old stores, matching local IDs across projects, historical short IDs, in/out-of-project paths, missing sources, ambiguous derivations, duplicate captures, all eight event-capable dialects, PDF capture, escaping, quota failures, and controlled deadline failures. Use the exact 10,000-value and value/edge/byte boundary fixtures, release-build timing protocol, and pinned Linux runner class in the [contract](contracts/turtle-export-contract.md). Run a separate ignored, real non-reading stdout-pipe probe on Linux, macOS, and Windows so the output deadline is demonstrated against an actual blocked OS write. Update user-facing docs with implementation.

**Rationale**: The constitution requires tests for changed behavior and documentation parity. The prior product assessment still has no named consumer; this plan does not reverse its hold recommendation.

**Alternatives considered**: Visual inspection or substring-only checks cannot establish valid Turtle or correct graph semantics. Implementation without a named consumer remains possible only after a separate product decision, not by treating this plan as approval.
