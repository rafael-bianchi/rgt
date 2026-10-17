# Data Model: Turtle Provenance Export

This design keeps RGT's recorded node and edge IDs unchanged. One new table records agent capture associations; a project setting supplies an export namespace. The Turtle graph is a read-time projection of these records.

## Persisted entities

### Project export identity

| Field | Source | Rule |
|-------|--------|------|
| `rdf_project_id` | `project_settings.value` under a fixed key | Random 128-bit lowercase hex value, inserted once with conflict-ignore and reread; immutable during normal use. |

New stores receive the token on initialization/open. Existing stores receive it on first open with this feature. A copied store keeps the same token; two independently initialized stores receive distinct tokens. Malformed existing tokens cause an actionable error, not silent regeneration. The token is an identifier, not a credential.

### Recorded value

Existing `tracked_nodes` rows remain authoritative for `id`, `node_type`, `value_kind`, exactly one typed value, optional `source_doc_id` and line, and stale state. The local ID is exported as data and is not re-keyed. A strict export reader rejects malformed value rows instead of using the permissive query helpers' defaults. Historical 48-bit and current 128-bit hash IDs remain valid as stored.

**Relationships**: A root may point to one source document. A derived value may have several incoming derivation edges. A recorded value may have zero or more capture associations, one per distinct known agent.

### Source document

Existing `source_documents.id` is the path-free identifier within a project, and `file_path` remains a local store value. A default export emits a relative path only for a verified in-project source; outside or unclassifiable sources retain only the path-free source identity. An explicit full-path export may emit an available absolute path. If a root's `source_doc_id` is null, no source entity is inferred.

Source row IDs remain stable while the row remains in an unchanged store. If a row is deleted or replaced, the export does not claim that the new row is the same source entity.

### Derivation edge

Existing `derivation_edges` rows keep `id`, `parent_node_id`, `child_node_id`, `operation_type`, and optional `expression`. `id` is the insertion-order signal for operands in an unambiguous group. Direct parent-child lineage is exported even if a complete derivation activity cannot be reconstructed. Referentially broken edges are store errors, not evidence of an activity.

### Capture association

New schema version 2 table:

| Field | Type | Rule |
|-------|------|------|
| `node_id` | text | Required foreign key to `tracked_nodes.id`; delete cascades when the node is removed. |
| `agent_name` | text | Required canonical agent name, validated against the supported registry before insertion. |

The pair `(node_id, agent_name)` is the primary key. A successful hook capture inserts the pair in the node's transaction. Repeating a capture by the same agent has no effect; a different agent adds one row. There is no event timestamp or read count. Historical and manual nodes have no association unless a future supported hook actually captures them.

**Migration**: Advance `PRAGMA user_version` from 1 to 2 with an explicit, atomic creation of the table and its index. The existing unversioned-store path first establishes the base schema and then reaches version 2. Preserve every old node, source, and edge; do not backfill unknown agents. A newer-than-supported store still fails to open.

## Read-time projection entities

| Entity | Identity | Evidence and relationships |
|--------|----------|----------------------------|
| Project | Project token | Context for every project-scoped value, source, and activity. |
| Value entity | Project token + encoded local node ID | `prov:Entity` with local ID, kind, typed value, stale state, optional line and source relationship. |
| Source entity | Project token + source row ID | `prov:Entity` and RGT source type, with relative path or opted-in absolute path when available. |
| Derivation activity | Project token + encoded child ID | One per child only when the edge group passes the evidence rule below; `prov:used` parents and is referenced by the child's `prov:wasGeneratedBy`. |
| Derivation usage | Derivation activity + zero-based operand index | `prov:Usage` linking a parent and its validated order in the expression. |
| Capture activity | Project token + encoded node ID + canonical agent name | One per capture association; `prov:used` the value and `prov:wasAssociatedWith` the agent. |
| Agent | Canonical software-tool name | One `prov:Agent` identity per known tool name; this represents the tool, not a human author or session. |

Every object IRI has a fixed RGT-controlled prefix and encoded components. It contains no raw project or source path. Independent project tokens make same local node IDs distinct in an RDF union. A normal RDF reader can combine files without a custom merge command; this feature provides no assertion of shared identity across projects.

## Export admission and deadline

Turtle export admits at most 10,000 recorded values, 30,000 derivation edges, and 64 MiB (67,108,864 bytes) of rendered UTF-8 Turtle. Each root references at most one source, and each value can have at most eight distinct known-agent associations, so the value limit also bounds those rows. These limits apply only to the Turtle projection, not to stored graph size or other commands. Value and edge limits are checked by bounded reads inside the consistent export snapshot before full loading; source and association references are validated as the snapshot is loaded. Buffered rendering stops as soon as it would exceed the byte limit, before stdout. A single one-second deadline starts at CLI entry and covers lock waits, reads, rendering, and the final write. A dedicated stdout writer reports completion over a channel; the main thread waits only for the remaining deadline and exits nonzero without joining a stalled writer. An exceeded quota or pre-write deadline produces an error with empty stdout; a write timeout or OS write failure exits nonzero and may leave incomplete bytes in the sink.

## Derivation evidence rule

For each derived child, inspect all incoming edges in insertion-ID order. An activity is supported only if:

1. At least one edge is present and each parent entity exists.
2. Parent IDs are distinct, and every edge has the same operation and expression value.
3. Recomputing RGT's derived ID from the ordered parents, operation, and stored child value matches the complete current ID or the historical 48-bit prefix form.

If any check fails, emit only supported direct `prov:wasDerivedFrom` parent links and report the omitted activity on stderr. Do not publish a `prov:wasGeneratedBy` relation or consolidate edge operation/expression labels into one claim for the uncertain group. This proves consistency with the recorded graph, not a complete history of every historical calculation. Future `derive` writes must commit the child and all its edges together.

## State transitions and invariants

- **Project token**: absent → generated once → stable. A malformed stored value is an error.
- **Capture association**: absent → present after a successful known-agent capture → unchanged on repeats → removed by node cascade deletion. Unknown, rules-only, failed, and manual paths do not create it.
- **Recorded value**: existing staleness and re-recording behavior remains authoritative; export observes the current snapshot without changing it.
- **Derivation projection**: valid group → activity emitted; ambiguous group → direct lineage and warning. The export itself does not repair the store.
- **Path disclosure**: default → relative or path-free source location; explicit full-path option → available absolute source-document path literal. Identifiers stay path-free in both modes. Recorded free text is preserved verbatim and may contain path-like strings.
- **Turtle admission**: within all row and byte limits → complete export if the deadline is met; over a limit or pre-write deadline → actionable error with no Turtle bytes; stalled final write → nonzero exit without a completeness claim.
