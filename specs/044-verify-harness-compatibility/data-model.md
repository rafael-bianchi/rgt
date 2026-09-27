# Data Model: Harness Compatibility Verification

**Scope**: Design concepts and file-based verification metadata. RGT's existing `tracked_nodes`, `source_documents`, and `capture_associations` remain the provenance store; this feature adds no graph or SQLite table.

## ClientSurface

| Field | Meaning | Rule |
|---|---|---|
| `surface_id` | Stable name for a distinct client host | Distinguish `copilot-chat` from `copilot-cli`, and `cline` from `roo-code`, even though RGT groups aliases under 13 canonical agent identifiers. Map each surface to its canonical capture agent where applicable. |
| `client_version` | Version tested or claimed | Required for a verified automatic claim. |
| `platform` | OS and architecture tested or claimed | Required for a verified automatic claim. |
| `registration_kind` | Native hook, plugin, or instruction guidance | Describes how the host loads RGT; does not imply successful capture. |

The support matrix contains all 13 canonical agent identifiers and separate subrows for Copilot Chat/CLI and Cline/Roo. Each event-capable surface lists its known read paths and exclusions rather than promoting every path from one successful tool; guidance-only surfaces have no invented native read path. Codex and Windsurf gain native hook registrations; Cline/Roo, Antigravity, and Kilo retain guidance-only registration. Native hook presence and value-capture tier are separate: a path-only Windsurf post-read event cannot create a source-backed value node.

## ReadPath

| Field | Meaning | Rule |
|---|---|---|
| `read_path_id` | Stable support/evidence name for one host read route | Unique within a client surface and shared by its success and no-op/error observations. `none` is reserved for guidance-only or non-read observations and cannot support an automatic value-capture claim. |
| `tool` / accepted result form | Native tool name and the completed-result shape the Rust adapter can reconstruct | Document both in the surface contract; a truncated, filtered, or path-only variant is an excluded form unless separately verified. |
| `capture_tier` | Verified automatic, configured but unverified, instruction-only, unsupported, or broken for this route | Scoped to read path, client version, and platform; does not change graph node identity. |

## IntegrationRegistration

| Field | Meaning | Rule |
|---|---|---|
| `surface_id` | Target client surface | Must resolve to a known client. |
| `scope` | Project or user configuration | Use only a location the client actually loads. |
| `artifact_path` | Settings, hook, plugin, or guidance path | An existing path is not proof that RGT is registered. |
| `entrypoint` | Command or plugin export loaded by the client | Must match the tested host contract and identify the RGT executable where relevant. |
| `event_phases` | Before and/or after events actually registered | A before-only read path cannot record values. |
| `ownership` | Current RGT, recognized legacy RGT, user-owned, or ambiguous | Only the first two may be changed automatically. A mixed group is ambiguous. |
| `health` | Active, missing, malformed, obsolete, ambiguous, trust-pending, inactive-workspace, or guidance-only | Determined by local inspection for `rgt doctor` where observable. Codex trust and Windsurf restricted mode may require host confirmation; an on-disk registration alone does not mean live invocation. |

**Lifecycle**: missing → installed; recognized legacy RGT → migrated → active; active → active on a repeated `rgt init`; ambiguous → unchanged with recovery diagnostic. Before an edit to an existing artifact, preserve a backup. Never delete or replace user-owned content based on a broad `hook pre|post` substring.

## NormalizedCaptureEvent

This is transient Rust data produced from a host callback; it is not persisted as a graph node.

| Field | Meaning | Rule |
|---|---|---|
| `surface_id` / `agent_name` | Host delivering the event | Agent name is the existing canonical capture attribution, not source authorship. |
| `phase` | Before or after execution | Before is always ineligible to create value nodes. |
| `read_target` | Path and kind of attempted read | A path alone cannot prove successful consumption. |
| `outcome_evidence` | Explicit success, failure/cancellation, completed result, or unknown | Explicit success proves completion, but without completed content it cannot justify value nodes. An after-event name alone is unknown. |
| `completed_content` | Text result or supported binary envelope the client received | Reconstruct exact full source bytes and source line positions from a documented result format. Path-only, truncated, filtered, or unmappable results create no source-backed roots. |
| `source_metadata` | Metadata/hash needed for existing source staleness behavior | Read only after eligibility. Confirm reconstructed bytes match the current file snapshot before storing its fingerprint; a mismatch creates no source-backed roots. |

**Capture decision**: `Ineligible` for before, failed, canceled, malformed, absent-result, path-only success, partial/filtered/unmappable result, changed snapshot, or unknown-outcome events; `Eligible` only for a completed successful read whose exact full source content RGT can reconstruct and match to the current file. `Eligible` with no numeric/date value is a no-op. A client that provides no qualifying event uses instruction-only read capture where guidance is supported.

## ProvenanceAssociation (existing)

The existing relation is unique on `(node_id, agent_name)`. Repeated callbacks for the same root identity and same agent must leave one node and one association. A different agent observing the same root may add a distinct association. No event timing, claim unit, or host version is added to the graph; those belong to the evidence ledger.

## CompatibilityEvidence

One file record follows [compatibility-evidence.schema.json](contracts/compatibility-evidence.schema.json).

| Field group | Meaning | Rule |
|---|---|---|
| Client identity | Surface, client version, RGT build version, platform | Required for every native observation; RGT version is obtained from `rgt --version`. |
| Event identity | Stable read-path ID, event phase, read tool, completion and content evidence, fixture/reference, method | The ID resolves to the surface contract's tool and accepted result form. It distinguishes native observation, deterministic fixture replay, vendor documentation, and RTK comparison; `full-snapshot-match` is separate from path-only success. |
| Result | Successful-read or no-op/error outcome, loaded registration, host response, graph value records, capture agent | Each recorded value includes its node ID, number/date kind, normalized value, and source line. The value list is empty for a no-op/error. |
| Provenance | Evidence date, redaction state, source reference | Native payload fixtures are redacted before tracking. No secrets, private paths, source contents, or session IDs are committed. |

**Promotion rule**: A read-path/client-version/platform combination may be labeled verified automatic value capture only when its evidence includes a native-client success with `full-snapshot-match` and a native-client no-op/error observation for the same `read_path_id`, version, and platform, with loaded registration, accepted neutral response, unchanged tool result, observed values/node IDs, and capture agent confirmed in RGT. The read-path ID must resolve to a documented tool and accepted result form; other paths remain unverified or instruction-only as supported by their own evidence. Fixture replay, vendor documentation, and RTK comparison alone never promote the tier. A client version or platform change starts a new unverified combination.
