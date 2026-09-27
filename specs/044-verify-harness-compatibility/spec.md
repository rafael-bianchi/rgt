# Feature Specification: Verify Harness Compatibility

**Feature Branch**: `feat/harness-compatibility-verification`

**Created**: 2026-09-26

**Status**: Draft

**Input**: User request to verify that RGT's integrations work in their respective agent clients, starting with a specification before changing behavior. The preceding comparison with RTK found registration, event-response, and plugin-contract gaps. Routine verification must remain local and must not run GitHub workflows or incur paid model calls.

## Clarifications

### Session 2026-09-26

- Q: If a client reports only a before-tool event, should RGT wait for proof of a successful read before recording values? → A: Yes. Record values only after evidence that the read succeeded; a before-tool event alone cannot create value nodes.
- Q: When normal `rgt init` finds an obsolete registration that is clearly owned by RGT, should it replace that registration automatically? → A: Yes. Normal initialization replaces clearly identified obsolete RGT entries and preserves unrelated settings; ambiguous ownership requires a diagnostic.
- Q: Should `rgt doctor` report only what it can inspect locally, or also show a separately recorded live-client verification status? → A: `rgt doctor` reports local registration health; versioned evidence and support documentation report live-client verification status.
- Q: If a client has no event that proves a file read succeeded, what support tier should RGT give its read-capture integration? → A: Classify read capture as instruction-only where the client can load guidance, and state the automatic-capture limit.
- Q: Should this feature add native hooks for Codex and Windsurf to satisfy the constitution? → A: Yes. Install and inspect both native registrations; only claim automatic value capture for a read path when its completed event supplies reconstructable source content and a real-client check proves it.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Automatic Capture Actually Runs (Priority: P1)

A developer enables RGT for an agent and read path advertised as having automatic value capture. After that path returns a complete successful read of a file containing a number or date, RGT records the source value and identifies the client that delivered the capture event. The original tool call completes normally.

**Why this priority**: A configuration file that exists but is ignored by its client leaves the user with silent gaps in the provenance graph.

**Independent Test**: For one declared automatic client read path and supported version/platform, initialize the integration, perform one real complete file-read event using a distinctive value, then confirm the value and capture agent in RGT. Also confirm the client tool result is unchanged.

**Acceptance Scenarios**:

1. **Given** a supported client read path, an initialized project, and a file with a distinctive number and date within the documented capture limits, **when** that path returns a complete successful result whose exact source bytes and lines match the current file snapshot, **then** the client invokes RGT and both values appear in the project graph with that client's capture attribution.
2. **Given** a failed or canceled read, or only a before-tool event, **when** RGT receives the event, **then** it does not present the read as completed or create value nodes without evidence of a successful read.
3. **Given** a client event with no capturable file or value, **when** RGT processes it, **then** the tool call proceeds unchanged and no value is added.
4. **Given** a completed successful read that reports only a path or a partial/filtered result, **when** RGT cannot reconstruct and match the exact source snapshot and source lines, **then** it creates no source-backed value nodes and states that read path's automatic-capture limit.

---

### User Story 2 - Existing Installations Can Be Corrected Safely (Priority: P1)

A developer who already ran `rgt init` can update an integration after its client changes its hook format or location. RGT corrects its own registration while preserving unrelated client settings and plugins.

**Why this priority**: Most affected users already have an installation. A fix that works only in an empty test home would leave their capture broken.

**Independent Test**: Start with a client configuration containing an older RGT registration and an unrelated user entry. Reinitialize; confirm the active registration works, the unrelated entry remains, and repeating initialization adds nothing.

**Acceptance Scenarios**:

1. **Given** a clearly identified obsolete RGT registration and unrelated client configuration, **when** the developer runs normal `rgt init`, **then** the obsolete entry is replaced by one current RGT registration and the unrelated configuration remains intact.
2. **Given** an already current registration, **when** the developer reinitializes, **then** the client has no duplicate RGT callbacks.
3. **Given** an unsafe or ambiguous migration, **when** initialization runs, **then** it reports the specific conflict and recovery action without deleting user-owned content.
4. **Given** Codex or Windsurf, **when** `rgt init -g` runs, **then** it configures one native RGT hook in the location loaded by that client version, preserves unrelated hooks, and reports any host trust or restricted-mode step needed before invocation.

---

### User Story 3 - Support Status Is Truthful (Priority: P2)

A developer runs `rgt doctor` to inspect local registration health and checks the support documentation for verified automatic value capture by read path, client version, and platform. The two reports make their different evidence sources clear.

**Why this priority**: The current existence check can look healthy even when a client never loads the RGT artifact.

**Independent Test**: Inspect diagnostic output for a valid registration, a stale or malformed registration, and an instruction-only client; then compare support documentation against the versioned live-client evidence. Neither a valid registration nor an old live test is presented as proof of current live capture.

**Acceptance Scenarios**:

1. **Given** a client settings file with no active RGT registration, **when** `rgt doctor` runs, **then** it does not report that client as having a working hook merely because the file exists.
2. **Given** instruction-only guidance, **when** support status is shown, **then** it does not promise automatic capture or client-enforced execution.
3. **Given** no recorded live verification for a client read path, version, and platform, **when** support is documented, **then** that read path remains marked unverified even if static checks pass or another read path was verified.
4. **Given** a client that cannot report a successful read but can load guidance, **when** its read-capture support is shown, **then** it is classified as instruction-only with the automatic-capture limit stated.

---

### User Story 4 - Maintainers Can Recheck Compatibility Locally (Priority: P2)

A maintainer can reproduce the compatibility checks after a client or RGT release, using recorded client evidence and local test runs. Routine checks do not require paid model calls or GitHub Actions.

**Why this priority**: Agent hook contracts change. A repeatable check prevents an old synthetic fixture from being mistaken for current client behavior.

**Independent Test**: On a fresh local workspace, run the documented checks for one supported client without a paid model request or hosted workflow; the report identifies the client version, platform, event tested, and result.

**Acceptance Scenarios**:

1. **Given** a captured and redacted client event, **when** its compatibility check runs, **then** the expected value, no-op behavior, and response format are checked without network or model access.
2. **Given** an available client that can generate a no-cost event, **when** its smoke check runs locally, **then** the result records the client version, platform, event, and observed graph outcome.
3. **Given** a client that cannot be exercised without cost or access, **when** checks run, **then** the result says unverified and does not silently count it as a pass.

### Edge Cases

- Client input contains an empty body, malformed data, a UTF-8 byte-order mark, or an unexpected field shape.
- The client requires a valid response even when RGT has nothing to record.
- One tool call emits both before and after events, or multiple registrations invoke RGT for the same action.
- A read command references a missing, changed, binary, or inaccessible file; a relative path resolves outside the expected working directory, or the completed result is truncated or filtered.
- The RGT executable is missing, moved, or unavailable when a client invokes a registered hook; a direct command-hook process stalls or its stdin stream never reaches EOF.
- The client supports a project registration, a user registration, or both; an existing user configuration has unrelated entries and comments.
- A client exposes only an event before tool execution, so completion of a read cannot be proven from that event alone.
- A client version or platform changes after an integration was verified.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: RGT MUST maintain an explicit support classification for each of its 13 named agents, separating native hook registration, automatic value capture by read path, and instruction-only read guidance, and distinguishing separately tested surfaces of one product, such as Copilot Chat and Copilot CLI. A read path identifies a client tool and the result form an adapter accepts; support documentation MUST name verified paths and exclusions rather than treating one verified tool as proof of every read route. This feature MUST add native hook registration for Codex CLI and Windsurf as well as the existing eight hook/plugin-capable agents.
- **FR-002**: Every read path advertised as having automatic value capture MUST be shown to be loaded and invoked by the named client on each platform/version combination claimed as verified.
- **FR-003**: For a supported qualifying successful read path, an automatic integration MUST record the observed numeric and date values in the active project and attach capture-agent attribution. A source-backed root MUST use exact source bytes and source line positions reconstructed from the completed result and confirmed against the current file snapshot before storing its file fingerprint; changed, truncated, filtered, or unmappable results MUST NOT create source-backed roots. Attribution MUST mean the canonical agent that delivered the event, not who authored the source or calculated a derivation; distinct client surfaces sharing that canonical agent are distinguished in compatibility evidence, not graph attribution.
- **FR-004**: RGT MUST create value nodes from automatic capture only after evidence that the client successfully read the source and provided interpretable completed content. An explicit success outcome proves completion but, without content, does not justify any value node. Event timing, a path, or a disk reread alone is insufficient. A before-tool event alone MUST NOT create value nodes or count as a completed read. If the client cannot provide adequate completion and content evidence for a read path, the support description MUST state that limit and classify read capture as instruction-only where guidance loads; it MUST NOT claim verified automatic capture for that path.
- **FR-005**: RGT hooks MUST leave the client's command, tool result, approval flow, and error outcome unchanged. Unexpected input or RGT failure, including a missing or moved RGT executable, MUST fail open with the response the client expects or a documented, tested host fail-open outcome.
- **FR-006**: Each automatic integration MUST use a registration and response format accepted by its client. A no-op response MUST still meet the client's protocol, including clients that require structured output.
- **FR-007**: RGT MUST handle empty, malformed, and byte-order-mark-prefixed events without blocking the client or creating spurious values.
- **FR-008**: Normal `rgt init` and `rgt init -g` MUST configure native Codex and Windsurf hooks, replace clearly identified obsolete RGT-owned registrations automatically, preserve unrelated user configuration, and avoid duplicate callbacks. Codex project hook trust and Windsurf restricted mode or precedence of hook files MUST be reported accurately. If ownership or migration safety is ambiguous, initialization MUST leave the affected entry unchanged and produce an actionable diagnostic.
- **FR-009**: Instruction-only integrations MUST place guidance where the named client can load it and MUST be identified as guidance, with no automatic-capture claim. A client without an event that proves a successful read MUST have instruction-only read-capture support where it can load guidance; its automatic-capture limit MUST be stated.
- **FR-010**: `rgt doctor` MUST report locally inspectable registration health and MUST NOT claim live-client verification. It MUST validate that RGT's expected registration is present and usable; the existence of a general client settings file alone MUST NOT count as a healthy hook. Versioned compatibility evidence and support documentation MUST carry live-client verification status.
- **FR-011**: A compatibility evidence record MUST identify the client surface, read path (tool plus accepted result form, or `none` for guidance-only observations), version, platform, event, registration-load observation, completion and content evidence, observed numeric/date values with node IDs, capture agent, host response, and verification method. A success and no-op/error pair used for an automatic value-capture claim MUST refer to the same non-`none` read path. Evidence derived only from another project's implementation MUST be labeled comparative, not client verification.
- **FR-012**: Routine compatibility checks MUST run locally without GitHub workflows or paid model calls. When a live client check cannot meet that constraint, it MUST remain explicitly unverified until a separately authorized check is possible.
- **FR-013**: Support documentation MUST state the verified platforms and client versions, distinguish unverified integrations, and be updated when behavior or support tier changes.
- **FR-014**: The same successful action MUST NOT produce duplicate tracked values solely because RGT receives repeated callbacks or both before and after events.
- **FR-015**: Passive processing MUST complete within one second for 64 KiB text events and 1 MiB binary-envelope events, including response framing. Hook input MUST be capped at 8 MiB and subject to an elapsed deadline even if a slow stream never reaches EOF. For plugin and direct command-hook launch paths, any RGT child or hook process MUST be bounded by an 800 ms deadline from invocation; timeout MUST stop processing and return the host-valid neutral response or use a documented, tested host fail-open outcome within the one-second budget. Oversized or stalled events MUST create no values.
- **FR-016**: The CLI MUST expose the exact running build version through both `rgt --version` and `rgt version`. Both forms MUST report the same version sourced from package build metadata. Compatibility evidence MUST record this RGT version alongside the harness client version.

### Key Entities

- **Client surface**: A named agent application or distinct host within it, with a version, platform, and native registration status.
- **Read path**: A stable identifier for one client tool and accepted completed-result form; value-capture status and evidence attach to this path, version, and platform.
- **Integration registration**: The client-owned configuration that invokes RGT or loads RGT guidance, including its scope and active status.
- **Capture event**: A client notification about a tool call, its outcome, and any file path or content RGT may observe.
- **Compatibility evidence**: A reproducible observation tied to a client surface, version, platform, event, expected response, and graph result.
- **Support status**: Verified automatic value capture, configured but unverified, instruction-only value recording, unsupported, or broken for each known read path, based on the available evidence; guidance-only clients have a surface-level guidance status without an invented native read path, and native registration health is separate.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: All 13 named agents have an explicit native-registration or guidance status; every known native read path has a capture tier, evidence status, and named exclusions. Guidance-only clients need no invented read path. Codex and Windsurf have inspectable native hook registrations, and Copilot Chat and Copilot CLI are evaluated as separate client surfaces.
- **SC-002**: Every read path labeled “verified automatic value capture” has at least one successful real-client full-content read and one no-op/error observation for the same read path, version, and platform claimed, with exact-snapshot content evidence, the captured value, node ID, client attribution, and loaded registration recorded in evidence and checked in RGT.
- **SC-003**: Local checks cover successful, empty, malformed, repeated, and obsolete-registration cases for every automatic integration; all cases pass without changing or blocking the original client tool call.
- **SC-004**: In diagnostic checks for all 13 agents, a settings file lacking an active RGT registration never produces a “healthy hook” result.
- **SC-005**: A maintainer can run the routine compatibility checks entirely on a developer's machine with zero paid service requests and zero hosted automation runs.
- **SC-006**: For 64 KiB text and 1 MiB binary-envelope client events, passive processing and response framing complete within one second; oversized or stalled events fail open within that budget and leave the client tool call able to proceed.
- **SC-007**: No published support claim describes an instruction-only or unverified integration as automatic capture.
- **SC-008**: `rgt --version` and `rgt version` both exit successfully and print the package version exactly; compatibility evidence requires a nonempty `rgt_version`.

## Assumptions

- This feature covers the 13 canonical agents RGT currently names. Additional agents present in newer RTK releases are recorded as a separate scope decision, not silently included.
- Cline/Roo, Antigravity, and Kilo remain guidance-only registrations. Codex and Windsurf gain native hooks in this feature. Codex's documented `PostToolUse` response may support capture for a qualifying full read. Windsurf's documented `post_read_code` payload reports the path but not source bytes, so its documented read path remains instruction-only for value capture until a suitable result-bearing client event is observed. Hook installation alone does not promote either client's value-capture tier.
- RTK's pinned source is comparative evidence. Vendor documentation and observed client behavior are required before marking RGT's integration verified.
- Existing provenance meaning, including capture-agent attribution, remains unchanged.
- Real event examples used in checks are redacted so private paths, source contents, credentials, and session identifiers are not committed.
- The user's existing preference against GitHub Actions and paid model calls applies to routine validation. A client that cannot be exercised under those constraints remains unverified.

## Out of Scope

- Adding integrations for agents not currently named by RGT, such as Trae, OpenClaw, Oh My Pi, or Factory Droid.
- Rewriting client commands or altering client permission decisions.
- Changing the provenance graph's value or derivation model.
- Running hosted CI/CD or paid agent sessions as part of routine compatibility checks.
