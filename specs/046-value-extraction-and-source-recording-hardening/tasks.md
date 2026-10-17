# Tasks: Value Extraction and Source Recording Hardening

**Input**: Feature specification `specs/046-value-extraction-and-source-recording-hardening/spec.md` and plan `specs/046-value-extraction-and-source-recording-hardening/plan.md`.

## Format: `[ID] [P?] [Story] Description`
- **[P]**: Parallelizable task.
- **[Story]**: [US1] through [US4] mapping to spec.md.

---

## Phase 1: Setup & Tests First

- [x] T001 [US1] Add unit tests in `tests/unit/hook_parsers.rs` for unspaced CSV rows (`10,200,300`, `1.5,200`, `10,20,30`, `1.5,2.5`, `12345,678`) and verifying `resolve_auto_format` on `1.5,2` does not switch to EU.
- [x] T002 [US2] Add unit tests in `tests/unit/hook_parsers.rs` for offset timestamps (`-03:00`, `+05:30`), local datetimes without `Z`, and invalid calendar dates (`2026-02-30`) asserting zero leaked `Number` nodes.
- [x] T003 [US3] Add unit tests in `tests/unit/hook_parsers.rs` for scientific notation (`1.5e10`, `1.5e-4`), unit suffixes (`12.5kg`, `99.9ms`), negative currencies (`-$1,200.00`, `($1,200.00)`, `-€500`), multi-hyphen CLI flags (`--100`), non-finite floats, and `-0.0`.
- [x] T004 [US4] Add contract tests in `tests/contract/test_cli_record_derive.rs` for `echo "Amount: 1234.56" | rgt record --stdin` without `<file>` and with an existing `<file>`, verifying `rgt status` staleness behavior.

---

## Phase 2: Implementation

- [x] T005 [US2] In `src/hooks/parser.rs`, update `iso_date_regex` to match optional fractional seconds and UTC offsets (`Z` or `[+-]\d{2}:\d{2}`), treat timezone-less ISO datetimes as UTC, and mask all matching spans in `date_spans` regardless of whether `chrono` validates them.
- [x] T006 [US1] In `src/hooks/parser.rs`, update number tokenization and `normalize_number` to split unspaced comma-separated numbers when they do not form a valid thousands-grouped number; disallow commas after decimal points; and update `resolve_auto_format` to require valid formatted numbers.
- [x] T007 [US3] In `src/hooks/parser.rs`, add support for scientific notation (`[eE][+-]?\d+`), lookahead for unit suffixes without backtracking across decimal points, update `candidate_context` to scan across currency symbols for negative signs while ignoring `--` prefixes, reject non-finite `f64`, and normalize `-0.0` to `0.0`.
- [x] T008 [US4] In `src/main.rs` and `src/cli/record.rs`, make `<file>` optional for `Commands::Record` when `--stdin` is set (defaulting to `"<stdin>"`), and when an existing disk file is passed with `--stdin`, compute metadata and `blake3_hash` from the disk file.
- [x] T009 [US4] In `src/graph/invalidation.rs`, ignore virtual sources (`"<stdin>"`) during filesystem staleness checks.
- [x] T010 [US4] In `src/hooks/mod.rs`, enforce `VALUE_LIMIT` (`10_000`) on passive hook captures.
- [x] T011 [US4] In `src/extract/pdf.rs`, enforce 10 MiB limit on decoded base64 and decompression output, and optimize `render_additional_context` character slicing.

---

## Phase 3: Verification & Parity

- [x] T012 Run the complete test suite (`cargo test </dev/null`) and verify all 46+ test targets pass.
- [x] T013 Run `./scripts/check-readme-parity.sh`, `./scripts/check-test-presence.sh`, and `./scripts/check-version-state.sh`.
