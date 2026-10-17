# Tasks: CLI, Turtle Export, Self-Updater, & Infrastructure Hardening

**Input**: Feature specification `specs/050-cli-export-updater-hardening/spec.md` and plan `specs/050-cli-export-updater-hardening/plan.md`.

## Format: `[ID] [P?] [Story] Description`
- **[P]**: Parallelizable task.
- **[Story]**: [US1] through [US5] mapping to spec.md.

---

## Phase 1: Setup & Tests First

- [x] T001 [US1] Add unit tests in `src/cli/status.rs` verifying `--stale-only` behavior in text and JSON modes.
- [x] T002 [US2] Add unit test in `src/cli/query.rs` verifying `render_lineage_text` renders `duration_seconds` when present.
- [x] T003 [US3] Add unit test in `src/export/turtle.rs` verifying `source_location` retains `relative_path` for a relative deleted file.
- [x] T004 [US4] Add unit tests in `src/updater/platform.rs` verifying `x86` is not `Arch::X86_64` and fallback triple order is `{arch}-{os}`.

---

## Phase 2: Implementation

- [x] T005 [US1] In `src/cli/status.rs`, implement `stale_only` filtering for text and JSON output.
- [x] T006 [US2] In `src/cli/query.rs`, include `duration_seconds` in `render_lineage_text`.
- [x] T007 [US3] In `src/export/turtle.rs`, preserve `relative_path` in `source_location` when deleted file was stored as a clean relative path; in `src/main.rs`, align graph deadline to 1000ms.
- [x] T008 [US4] In `install.sh`, require checksums download or exit 2, and use atomic replacement (`cp` to `.tmp` + `mv -f`). In `src/updater/platform.rs`, fix `x86` architecture and fallback triple format.
- [x] T009 [US5] In `src/main.rs`, replace duplicate module declarations with `use rgt::*;`.

---

## Phase 3: Verification & Parity

- [x] T010 Run the complete test suite (`cargo test </dev/null`) and verify all tests pass.
- [x] T011 Run `./scripts/check-readme-parity.sh`, `./scripts/check-test-presence.sh`, and `./scripts/check-version-state.sh`.
