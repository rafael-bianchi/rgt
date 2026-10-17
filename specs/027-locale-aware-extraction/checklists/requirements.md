# Specification Quality Checklist: Locale-Aware Extraction

**Purpose**: Validate spec completeness and quality before `/speckit-plan`.
**Created**: 2026-08-29
**Feature**: `specs/027-locale-aware-extraction`

## Content Quality

- [x] No implementation details (languages, frameworks, APIs) leak into the spec
- [x] Requirements are testable and unambiguous
- [x] Success criteria are measurable and technology-agnostic
- [x] All acceptance scenarios are defined in Given/When/Then format
- [x] Edge cases are identified and enumerated
- [x] Scope is clearly bounded (in-scope vs. out-of-scope)

## Requirement Completeness

- [x] Every functional requirement has an acceptance scenario or success criterion
- [x] No [NEEDS CLARIFICATION] markers remain
- [x] User stories are prioritized (P1/P2) with independent tests
- [x] Assumptions are documented
- [x] Dependencies (internal/external) are identified (none — no new crates)

## Feature Readiness

- [x] All functional requirements trace to a user story or finding
- [x] The spec answers "what" not "how"
- [x] The feature is shippable in a single release
- [x] Non-goals (out of scope) are explicit

## Notes

- Persistence is a DB `project_settings` key/value table (decided) — both paths
  already open the SQLite store; spec stays "how"-free.
- `--number-format` flag surface on `rgt init` + `rgt record` matches §3.2.
- Default format is `auto` (decided) — a deliberate behavior change vs. today's
  `.`-only split; affected node IDs change (intended).
- FR-005: `rgt record` warns on stdout; passive hooks skip silently (never block).
