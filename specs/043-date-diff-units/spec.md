# Feature Specification: Safe Duration Calculations and Unit Display

**Feature Branch**: `develop` (specification only; no feature branch created)

**Created**: 2026-09-25

**Status**: Draft

**Input**: User description: "Let an agent choose the output unit for date differences, including days or minutes, without risking calculations or provenance when different durations are combined. RGT should compute date gaps, retain typed durations for sums and averages, and protect precision and graph identity."

## Clarifications

### Session 2026-09-25

- Q: If a Duration node appears twice in a sum or average, should RGT reject the repeated parent or count it twice? → A: Reject repeated parent IDs for typed duration operations.
- Q: Should an agent be able to choose a display unit when querying an existing Duration node, or only when first deriving it? → A: Support unit selection on both derive and queries for Duration nodes.
- Q: If an agent supplies the same Duration parents in a different order for a sum or average, should RGT reuse the same result node? → A: Reuse one node for the same set of distinct parents and operation; DATE_DIFF remains ordered.
- Q: When an agent queries a Duration with a chosen unit, should the response keep its existing value field and add a separate converted display, or replace value? → A: Preserve value and add a clearly labelled selected-unit display and exact seconds for the requested node.
- Q: Should duration sums and averages accept a stored negative Duration parent, or reject it? → A: Accept signed Duration parents and calculate from their exact seconds; the nonnegative rule applies only to DATE_DIFF.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Derive a date gap without calculating it first (Priority: P1)

An agent selects two recorded dates and asks RGT to derive their elapsed time. RGT calculates and records the gap, then reports it in the requested unit. The agent does not need to calculate seconds or supply a proposed result. If it does supply a result, RGT checks that claim before recording anything.

**Why this priority**: The observed attempt to record a 45-day gap as `45` failed because the existing command interpreted `45` as seconds. Letting RGT calculate the gap removes that source of error.

**Independent Test**: Record two dates 45 days apart, derive `DATE_DIFF` with `--unit days` and no `--result`, then inspect the response and recorded node.

**Acceptance Scenarios**:

1. **Given** two Date nodes exactly 45 days apart, **when** an agent derives `DATE_DIFF` with `--unit days` and no `--result`, **then** RGT reports `45 days`, records one Duration of 3,888,000 seconds, and links it to both dates.
2. **Given** the same dates, **when** an agent supplies `--result 45 --result-unit days`, **then** the claim passes and refers to the same Duration node as the automatically calculated result.
3. **Given** the same dates, **when** an agent supplies `--result 44 --result-unit days`, **then** the command reports the expected and supplied gaps and records no new node or edge.
4. **Given** the same dates, **when** an agent omits `--unit`, **then** an otherwise valid existing invocation keeps its current output contract and an omitted `--result-unit` still means seconds.

---

### User Story 2 - Average or add durations without losing their type (Priority: P2)

An agent has one gap of 45 days and another of 45 minutes. It asks RGT to add or average the Duration nodes. The result remains a Duration with lineage to both parents, regardless of the units used to display the original gaps.

**Why this priority**: General expressions currently turn Duration parents into seconds and record a Number. Typed operations prevent the resulting graph node from losing its time meaning.

**Independent Test**: Derive a 45-day gap and a 45-minute gap, then derive their sum and average. Inspect their values, kinds, edges, and selected-unit displays.

**Acceptance Scenarios**:

1. **Given** Duration parents of 3,888,000 and 2,700 seconds, **when** an agent derives `DURATION_SUM`, **then** RGT records a Duration of 3,890,700 seconds linked to both parents.
2. **Given** those same parents, **when** an agent derives `DURATION_AVG` with `--unit days`, **then** RGT records a Duration of 1,945,350 seconds and reports `22.515625 days`.
3. **Given** a Date or Number among the parents, **when** an agent requests either duration operation, **then** RGT rejects the command with an actionable type error and records nothing.
4. **Given** Duration parents of one and two seconds, **when** an agent requests their average, **then** RGT rejects the unrepresentable 1.5-second result without rounding or recording a node.
5. **Given** the same Duration node ID listed twice, **when** an agent requests a duration sum or average, **then** RGT rejects the repeated parent and records nothing.
6. **Given** the same distinct Duration parents in reverse order, **when** an agent repeats a duration sum or average, **then** RGT reuses the same result node and does not add duplicate lineage.
7. **Given** one negative and one positive Duration parent, **when** an agent requests a sum or average whose result is a representable whole number of seconds, **then** RGT accepts the signed result and retains both parent links.

---

### User Story 3 - Verify claims with explicit units (Priority: P3)

An agent can still check a result it calculated. The unit of its claim is explicit and independent of the unit chosen for display, so a displayed answer cannot silently change how verification interprets a number.

**Why this priority**: A single `--unit` serving both input and output would be ambiguous. Existing seconds-based claims also need to retain their meaning.

**Independent Test**: Verify equivalent claims in days and seconds for one date gap and one duration average, then try a wrong claim and invalid unit.

**Acceptance Scenarios**:

1. **Given** dates 45 days apart, **when** an agent verifies `DATE_DIFF` with `--result 45 --result-unit days`, **then** verification succeeds without recording a node.
2. **Given** the same dates, **when** an agent verifies with `--result 3888000` and no `--result-unit`, **then** verification succeeds with the existing seconds meaning.
3. **Given** a wrong but representable claim, **when** verification fails, **then** the diagnostic names the claim's unit and gives the exact expected duration in seconds.
4. **Given** `--result-unit months` or `--unit years`, **when** an agent submits the command, **then** it rejects the unsupported calendar unit and lists the supported choices.

---

### User Story 4 - Redisplay a stored duration (Priority: P3)

An agent can query a previously derived Duration and choose the unit needed for its current answer. This changes only that response, leaving the stored value and its lineage intact.

**Why this priority**: Agents may revisit a recorded gap after the original derivation and need a different unit without making a duplicate graph node.

**Independent Test**: Derive a Duration once, query its node ID with `--unit days` and then `--unit minutes`, and compare the returned value, identity, and lineage.

**Acceptance Scenarios**:

1. **Given** a stored Duration of 3,888,000 seconds, **when** an agent queries that node with `--unit days`, **then** the response identifies the same node, keeps its existing `value`, and adds `45 days` as a separate display alongside exact seconds.
2. **Given** that same node, **when** an agent queries it with `--unit minutes`, **then** the requested node reports `64,800 minutes` and no graph value or edge changes.
3. **Given** a Number or Date node, **when** an agent queries it with `--unit days`, **then** RGT rejects the inapplicable unit instead of assigning time meaning to the value.

### Edge Cases

- An exact zero gap is valid; a negative gap, including a negative gap shorter than one second, follows the existing reversed-date error rule.
- Date parents whose true elapsed time includes a fractional second are rejected until sub-second Duration storage is specified. Truncating such a gap to seconds is not allowed.
- Decimal claims such as `0.1 minutes` are accepted when they equal a whole number of seconds exactly; claims requiring a fractional second, non-finite numbers, and out-of-range values are rejected without rounding.
- A duration average that is not a whole number of seconds is rejected; it is never stored as a rounded Duration or a bare Number.
- Duration sums and averages accept signed parents and signed results when representable; the nonnegative convention applies only to `DATE_DIFF`.
- Sum and average require at least two Duration parents. Sum overflow and excessive parent counts are rejected without graph writes.
- Sum and average reject repeated parent node IDs because a single graph edge cannot show how many times one parent contributed.
- Repeating the same derivation with another display unit or an equivalent asserted result reuses its graph identity; the chosen display or claim unit does not create a second value.
- Parent order is significant for `DATE_DIFF` and insignificant for `DURATION_SUM` and `DURATION_AVG` identity.
- An existing historical node with the same parents, operation, and exact duration remains reachable; a new derivation must not replace it or create a conflicting duplicate.
- For a selected unit that needs a repeating decimal, the response labels the decimal as approximate and includes the exact seconds value. Display rounding never participates in verification or identity.
- `query --unit` adds a separate display and exact seconds for only the requested Duration node; the existing `value` field and lineage values remain unchanged. A query without `--unit` keeps its current response shape and values.
- A general `EXPRESSION` with Duration or Date parents retains its current behavior during compatibility transition but warns that temporal inputs are coerced to seconds and its result is a Number.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: `derive` MUST calculate `DATE_DIFF` from exactly two ordered Date parents when `--result` is omitted, verify that the elapsed time is nonnegative and representable as whole seconds, and record it as a Duration linked to both parents.
- **FR-002**: For `DATE_DIFF`, `DURATION_SUM`, and `DURATION_AVG`, `derive --unit` MUST select only the reported unit, while `derive` and `verify` MUST use `--result-unit` solely for an asserted `--result`. `query --unit` MUST select the display unit for the requested Duration node. Both unit choices MUST accept exactly `seconds`, `minutes`, `hours`, `days`, or `weeks`. Omitted `--result-unit` MUST mean seconds. An explicit `derive --unit` MUST report the value in that unit while retaining the derived node ID in the response; omitting `--unit` MUST preserve existing `DATE_DIFF` output behavior. `verify` does not accept `--unit` because a successful verification has no derived-value display.
- **FR-003**: A supplied `--result` on `derive` MUST be checked against the calculated value before any graph write. `verify` MUST continue to require `--result` and MUST apply the same claim validation for each supported operation without recording a node. `--result-unit` without `--result` MUST be invalid.
- **FR-004**: Unit conversion MUST use fixed elapsed-time scales: 60 seconds per minute, 3,600 per hour, 86,400 per day, and 604,800 per week. Calendar months and years MUST be rejected because their lengths depend on calendar context. Unit flags MUST be rejected for unrelated operations.
- **FR-005**: Accepted duration claims MUST represent exact whole seconds within the supported range. Validation MUST distinguish decimal inputs that differ beyond ordinary binary floating-point precision, reject non-finite and out-of-range input, and never truncate or round a claim to make it pass.
- **FR-006**: `DATE_DIFF` MUST check the full elapsed time between its Date parents before any conversion to whole seconds. A negative or fractional-second gap MUST NOT be accepted as a nonnegative whole-second Duration. Failed derivations MUST create no node or edge.
- **FR-007**: `DURATION_SUM` and `DURATION_AVG` MUST each require at least two distinct Duration parent IDs and produce a Duration linked to every parent. They MUST calculate from each parent's exact signed canonical seconds and MAY produce a negative Duration. Repeated parent IDs, overflow, or a fractional-second result MUST fail without recording. They MUST NOT silently coerce a Date or Number parent. The nonnegative rule MUST remain specific to `DATE_DIFF`.
- **FR-008**: The same ordered Date parents, `DATE_DIFF` operation, and exact canonical duration MUST refer to one derived value. For `DURATION_SUM` or `DURATION_AVG`, the same set of distinct parent IDs, operation, and exact canonical duration MUST refer to one derived value regardless of parent order. Display and claim units MUST NOT affect identity. Distinct canonical durations MUST remain distinguishable even when a human-readable display would look the same. Existing stored node identities MUST remain accessible and MUST not be silently overwritten or duplicated by an equivalent new derivation.
- **FR-009**: Duration nodes MUST retain their Duration type, exact canonical seconds in SQLite, and parent lineage. Turtle export MUST expose the canonical Duration value as a typed `xsd:duration` and preserve its supported lineage. `query --unit` MUST expose exact seconds in `duration_seconds`; a query without `--unit` and the existing text graph view MUST retain their current display contracts and need not add a seconds field. A user-selected display or claim unit MUST NOT be represented as a different measured duration or as evidence that the unit was stored with the node.
- **FR-010**: A `derive --unit` or `query --unit` display MUST include the relevant node ID and exact canonical seconds. If its decimal value in the selected unit is inexact, the display MUST mark it as approximate. `query --unit` MUST preserve the existing `value` field and add a separately labelled selected-unit display and lossless exact seconds for only the requested Duration node. It MUST reject a non-Duration target and leave lineage, stored values, and graph identity untouched. Queries without `--unit` MUST retain their existing response contract. Display precision MUST NOT change verification or identity.
- **FR-011**: Existing `EXPRESSION` behavior for Duration and Date parents MUST remain available for compatibility in this feature but MUST emit a diagnostic when used, stating the coercion and that its result is a Number. Agent-facing guidance MUST direct duration sums and averages to the typed operations. Any later change to reject legacy temporal expressions requires a separate migration decision.
- **FR-012**: Command help and agent-facing documentation MUST explain automatic `DATE_DIFF`, typed duration operations, derive/query display units versus claim units, whole-second limits, unsupported calendar units, legacy expression coercion, and the absence of per-node input-unit metadata.

### Key Entities *(include if feature involves data)*

- **Date difference**: A deterministic elapsed time calculated from two ordered Date parent nodes.
- **Duration**: A graph value with an exact whole-second elapsed amount, a derivation operation, and links to its parent nodes.
- **Duration claim**: An optional asserted decimal amount and its explicit or default unit, checked before recording; it is not the stored value.
- **Display unit**: A request-specific presentation choice for an already calculated Duration; it is not part of that value's graph identity.
- **Duration aggregate**: A sum or mean of two or more Duration parents that remains a Duration node.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: An agent can derive and report the observed 45-day gap in one command without calculating or supplying seconds, and can trace the resulting Duration to both dates.
- **SC-002**: Across derive and query fixtures for all five supported display and claim units, equivalent inputs yield one exact Duration identity; 100% of selected-unit displays include the exact seconds value and label any inexact decimal as approximate.
- **SC-003**: For a 45-day and a 45-minute gap, sum and average yield typed Duration nodes of 3,890,700 and 1,945,350 seconds with both parents in their lineage; the average displays as `22.515625 days` on request.
- **SC-004**: In fixtures for wrong claims, invalid units, wrong or repeated parents, reversed or fractional-second date gaps, overflow, and fractional-second averages, 100% fail with no new node or edge.
- **SC-005**: Existing seconds-based claims and stored historical durations retain their meaning and remain queryable in all regression fixtures. Equivalent new derivations, including reordered sum or average parents, do not overwrite or duplicate a node.
- **SC-006**: General expressions using temporal parents still run with their documented legacy semantics and emit a coercion warning in 100% of relevant fixtures; typed duration operations never produce a bare Number.
- **SC-007**: In a fixed local fixture with 10,000 stored nodes and the maximum 26 distinct Duration parents for aggregate operations, valid and invalid derive, verify, and query subprocess commands each have a 95th-percentile elapsed time below one second across 20 measured runs after one warm-up run per command. Timing includes process startup and output, but excludes fixture construction. Record the OS, CPU, release-build profile, commands, and measurements; keep wall-clock timing out of deterministic correctness test assertions.
- **SC-008**: Querying one Duration node in two units preserves the existing `value`, node identity, and lineage while adding the requested display and exact seconds; a unit request on a non-Duration target fails in 100% of relevant fixtures. Queries without a unit keep their existing response shape.
- **SC-009**: Signed Duration fixtures produce exact sums and averages when representable, while reversed-date `DATE_DIFF` fixtures still fail in 100% of relevant cases.

## Assumptions

- Only fixed elapsed-time units are in scope. Calendar month and year results need a separate specification that defines calendar arithmetic.
- A Duration node records the elapsed amount in seconds; an agent's original claim unit and requested display unit are not retained as node properties.
- The canonical seconds value is authoritative. A selected-unit decimal may be approximate when the conversion repeats, and the response includes exact seconds so a later calculation does not rely on the rounded display.
- Whole-second storage remains the feature boundary. Supporting fractional-second Duration values would require a separately reviewed data and exchange contract.
- Existing `EXPRESSION` semantics remain available for compatibility. A warning and typed alternatives reduce accidental misuse; completely preventing legacy temporal coercion requires a later migration decision.
- Existing graph formats and project-scoped Turtle identities continue to expose the recorded value and lineage. This feature does not add a new graph format or store per-command presentation preferences.
