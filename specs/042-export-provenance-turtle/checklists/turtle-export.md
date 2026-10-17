# Turtle Export Requirements Checklist: Export Provenance as Turtle

**Purpose**: Review the completeness, clarity, consistency, and measurability of the full Turtle export requirements before implementation.
**Created**: 2026-09-24
**Feature**: [spec.md](../spec.md)
**Audience and timing**: PR reviewers, before implementation approval.

**Review Ownership**: This is a reviewer-owned requirements-quality checklist. Mark an item `[x]` only when the reviewer determines the requirements-quality criterion is satisfied. An `[x]` does not mean implementation work is complete.

## Requirement Completeness

- [ ] CHK001 Are requirements defined for preserving the value, kind, stale state, source, and lineage of both root and derived values? [Completeness, Spec §FR-002–FR-004]
- [ ] CHK002 Are the required direct lineage and activity relationships specified for both supported and uncertain derivations? [Completeness, Spec §FR-003, §FR-005]
- [ ] CHK003 Are capture attribution requirements complete for known agents, repeated captures, multiple agents, and records with no agent evidence? [Completeness, Spec §FR-009–FR-011]
- [ ] CHK004 Are the CLI requirements explicit for Turtle selection, the full-path option, invalid flag combinations, and unchanged legacy formats? [Completeness, Spec §FR-001, §FR-004, §FR-012; Gap]

## Requirement Clarity

- [ ] CHK005 Is “unambiguous derivation” defined with criteria that distinguish a complete calculation from a set of stored parent links? [Clarity, Spec §FR-003, §FR-005]
- [ ] CHK006 Is the rule for classifying a source as inside, outside, or unclassifiable relative to the project clear enough for symlinks and paths containing `..`? [Clarity, Spec §FR-004; Gap]
- [ ] CHK007 Is the set of agent identities eligible for attribution clear, including aliases and the boundary between event-capable and instructions-only integrations? [Clarity, Spec §FR-009, §Assumptions]
- [ ] CHK008 Is “available absolute source path” defined for stored relative paths and sources that no longer exist? [Ambiguity, Spec §FR-004, §SC-007]

## Requirement Consistency

- [ ] CHK009 Are identity rules consistent for repeated exports, independently initialized projects, and physical copies of one store? [Consistency, Spec §FR-006–FR-008; Gap]
- [ ] CHK010 Do the source-identity and path-disclosure requirements agree across the default and explicit full-path modes? [Consistency, Spec §FR-004, §SC-007]
- [ ] CHK011 Are the required number, date, and duration representations consistent between the specification and the Turtle contract? [Consistency, Spec §FR-002, §SC-001]
- [ ] CHK012 Are capture activity claims consistent with the prohibition on inferring source authorship or derivation responsibility from a hook event? [Consistency, Spec §FR-011, §SC-004]

## Acceptance Criteria Quality

- [ ] CHK013 Can an independent reader determine value type, source reference, parents, and known capturing agents using only the graph facts required by the acceptance criteria? [Measurability, Spec §SC-001, §SC-005]
- [ ] CHK014 Are stable within-project identity and distinct cross-project identity measurable when local node IDs match? [Measurability, Spec §SC-002]
- [ ] CHK015 Does the attribution criterion identify the event-capable integrations and define how one association per agent and value is counted? [Measurability, Spec §SC-003]
- [ ] CHK016 Does the 10,000-value latency criterion define the measurement boundary, reference hardware, and representative source and edge density? [Measurability, Spec §SC-006; Gap]

## Scenario Coverage

- [ ] CHK017 Are requirements present for empty graphs, escaped text, Unicode, and repeated exports of unchanged data? [Coverage, Spec §FR-013, §Edge Cases]
- [ ] CHK018 Are manual, historical, unknown-agent, rules-only, and failed-hook cases covered without unsupported attribution? [Coverage, Spec §FR-009–FR-011, §SC-004]
- [ ] CHK019 Are export error requirements complete for invalid options, malformed stored data, snapshot or render failure, and an output write failure? [Coverage, Spec §FR-014; Gap]
- [ ] CHK020 Are recovery and rollback requirements defined for an interrupted store migration or failed capture transaction? [Coverage, Gap]

## Edge Case Coverage

- [ ] CHK021 Are path disclosure requirements defined for symlinks, traversal components, unavailable files, and sources outside the selected project? [Edge Case, Spec §FR-004, §Edge Cases]
- [ ] CHK022 Are missing source references and missing or invalid line information addressed without inventing provenance facts? [Edge Case, Spec §FR-004; Gap]
- [ ] CHK023 Are partial, duplicate, inconsistent, or broken derivation edges classified clearly as either supported lineage, omitted activity, or export error? [Edge Case, Spec §FR-005; Gap]
- [ ] CHK024 Are historical short node IDs and malformed project identity data covered with stable-identity and error requirements? [Edge Case, Spec §FR-006; Gap]

## Non-Functional Requirements

- [ ] CHK025 Does the default privacy requirement state whether absolute paths are forbidden only in identifiers and source-path fields or also in other exported literals such as expressions? [Ambiguity, Spec §FR-004, §SC-007]
- [ ] CHK026 Is deterministic ordering defined for all exported facts and warnings, including projects with multiple agents or derivation parents? [Clarity, Spec §FR-013]
- [ ] CHK027 Are performance expectations for graphs larger than 10,000 values reconciled with the constitution's general sub-second CLI latency bound? [Conflict, Spec §SC-006]
- [ ] CHK028 Are passive-hook failure requirements clear about preserving the host tool's progress while recording no unsupported attribution? [Clarity, Spec §FR-014, §User Story 3]

## Dependencies and Assumptions

- [ ] CHK029 Are the interoperability assumptions about independent Turtle readers, PROV-O terms, and ordinary RDF union stated precisely enough for a consumer to interpret combined exports? [Assumption, Spec §FR-001, §FR-008]
- [ ] CHK030 Is the prior assessment's named-consumer go/no-go condition distinguished from requirements readiness and implementation approval? [Assumption, Spec §Assumptions]

## Notes

- Mark items `[x]` only after a reviewer has evaluated the quality of the written requirements.
- Leave items unchecked when they need clarification, correction, or reviewer evaluation.
- `$speckit-implement` reads checklist state but does not change checkbox markers.
- [requirements.md](requirements.md) has a separate built-in lifecycle maintained by `$speckit-specify` and `$speckit-clarify`.
- Add findings inline and link to the relevant requirement or design artifact.
