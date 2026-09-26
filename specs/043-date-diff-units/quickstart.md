# Quickstart Validation: Safe Duration Calculations

**Feature**: [spec.md](spec.md) | **Contracts**: [CLI](contracts/duration-cli.md), [graph](contracts/duration-graph.md)

This is a local validation guide for after implementation. Use an isolated temporary RGT project and a locally built `rgt` binary; do not run CI/CD or GitHub workflows. The angle-bracket IDs below are obtained from `rgt graph` or `rgt query` after recording the fixture.

## Fixture and prerequisites

Create a text file `dates.txt` in the temporary project containing these four lines:

```text
first_start,2026-01-01T00:00:00Z
first_end,2026-02-15T00:00:00Z
second_start,2026-03-01T00:00:00Z
second_end,2026-03-01T00:45:00Z
```

Initialize and record locally:

```text
rtk rgt init --agent codex
rtk rgt record dates.txt
rtk rgt graph
```

Record the four Date node IDs as `<start_45d>`, `<end_45d>`, `<start_45m>`, and `<end_45m>`.

## 1. Calculate and assert a 45-day gap

```text
rtk rgt derive --parents <start_45d>,<end_45d> --operation DATE_DIFF --unit days
rtk rgt verify --parents <start_45d>,<end_45d> --operation DATE_DIFF --result 45 --result-unit days
rtk rgt derive --parents <start_45d>,<end_45d> --operation DATE_DIFF --result 3888000
```

Expected: the first command reports `45 days` and `3888000 seconds`; verify succeeds silently; the third command returns the same derived node ID as the first and keeps the old no-unit stdout form. Query the ID and confirm kind `DURATION` and both Date parents.

Try `--result 44 --result-unit days`: expect verification failure with no new node/edge. Try `--result 1.0000000000000001 --result-unit seconds` for a one-second fixture: expect invalid input, never acceptance through floating-point rounding.

## 2. Mix duration scales safely

```text
rtk rgt derive --parents <start_45m>,<end_45m> --operation DATE_DIFF --unit minutes
rtk rgt derive --parents <duration_45d>,<duration_45m> --operation DURATION_SUM --unit minutes
rtk rgt derive --parents <duration_45d>,<duration_45m> --operation DURATION_AVG --unit days
```

Expected: the 45-minute Duration is `2700 seconds`; sum is a Duration of `3890700 seconds`; average is a Duration of `1945350 seconds` and displays `22.515625 days`. Each aggregate has edges to both Duration parents. Reversing the two aggregate parent IDs yields the same respective result IDs. Listing either ID twice fails without writing. A Number or Date among aggregate parents fails with a type error.

For a signed-Duration fixture, use an existing negative Duration node and a positive one: sum/average may be negative if exact and representable. The `DATE_DIFF` reversed-date error remains in force.

## 3. Redisplay a stored node

```text
rtk rgt query <duration_45d> --unit days
rtk rgt query <duration_45d> --unit minutes --json
rtk rgt query <duration_45d> --json
```

Expected: the first two responses keep the requested node's existing `value` and add exact `duration_seconds` plus a separate selected-unit display only to that node's lineage step. The unselected query has its original shape. A `query --unit days` on a Date or Number node fails without changing the graph. A one-second Duration queried in minutes shows an approximation marker and exact `1` second.

## 4. Reject precision loss and preserve history

- Use Date nodes separated by a fractional second and then by a negative fractional second; `DATE_DIFF` must reject both without a node/edge write.
- Use one-second and two-second Duration parents; `DURATION_AVG` must reject their 1.5-second mean.
- Use signed and maximum-range Duration fixtures to confirm checked overflow and lossless `duration_seconds` strings.
- Open a fixture store containing a historical 12- or 32-character `DATE_DIFF` ID. An equivalent derivation must reuse it; a legacy-ID candidate with conflicting seconds or lineage must never be overwritten.
- Corrupt a copied fixture's Date or Duration typed column. A temporal derivation must report the corrupt row rather than substitute current time or zero.

These checks belong in deterministic automated tests, using copied fixtures and isolated databases rather than the user's live `.rgt/store.db`.

Implementation validation on 2026-09-25 ran the sample project flow in a fresh temporary directory. Initialization, recording, automatic 45-day and 45-minute gaps, correct and incorrect claims, typed sum/average output, selected-unit query fields, and Turtle Duration/activity output all matched the expectations above. The integration suite also passed historical 12- and 32-character ID reuse and conflicting-candidate preservation tests.

## 5. Export and local validation

```text
rtk rgt graph --format ttl
rtk cargo fmt --all -- --check
rtk cargo clippy --all-targets -- -D warnings
rtk cargo test duration
```

Expected: Turtle contains typed Duration values, parent relationships, and supported activities for new `DATE_DIFF`, `DURATION_SUM`, and `DURATION_AVG` IDs. Historical activities remain available. The Rust checks and relevant local tests pass before a future PR. No hosted workflow is invoked by this guide.

## 6. Local performance measurement (implementation validation)

The performance test is ignored by default, so ordinary `rtk cargo test` does not run its timing loop. Run the measurement explicitly with:

```text
rtk cargo test --release --test test_duration_performance -- --ignored --nocapture
```

Build a release binary and use an isolated fixture with 10,000 stored nodes. For each of `DATE_DIFF`, `DURATION_SUM`, and `DURATION_AVG`, time one valid and one invalid `derive` command and one valid and one invalid `verify` command; use 26 distinct Duration parents for aggregate cases. Also time a valid `query --unit` on a Duration and an invalid `query --unit` on a non-Duration node. Select invalid inputs that fail before writing, and use fresh copies of the fixture where a valid derive would otherwise change it.

For each command, run one warm-up and then 20 release-binary subprocess invocations. Measure elapsed time from process launch through captured output; exclude fixture construction. Sort the 20 measurements and use the 19th as the 95th percentile, which must be below one second. Record the OS, CPU, release profile, exact commands, and per-command percentile here when implementation is validated. The manual test reports these measurements without asserting on elapsed time; deterministic correctness tests must not depend on the wall clock. Do not run a hosted workflow for this measurement.

### Recorded local measurement

Measured locally on macOS 27.0 with an Apple M5 using Cargo's optimized `release` profile. The test used a 10,000-node fixture, made a fresh database copy for each invocation, ran one warm-up and 20 timed subprocesses per command, and included process startup and captured output. The exact performance command was:

```text
rtk proxy cargo test --release --test test_duration_performance -- --ignored --nocapture
```

The following shell variable expands to the exact 26 distinct aggregate parents used by all sum and average cases:

```bash
D26=duration-00,duration-01,duration-02,duration-03,duration-04,duration-05,duration-06,duration-07,duration-08,duration-09,duration-10,duration-11,duration-12,duration-13,duration-14,duration-15,duration-16,duration-17,duration-18,duration-19,duration-20,duration-21,duration-22,duration-23,duration-24,duration-25
```

Timed command forms and p95:

| Case | Command | p95 |
|---|---|---:|
| DATE_DIFF derive valid | `rgt derive --parents date-start,date-end --operation DATE_DIFF --unit days` | 5.408 ms |
| DATE_DIFF derive invalid | `rgt derive --parents date-start,date-end --operation DATE_DIFF --result 0` | 3.496 ms |
| DATE_DIFF verify valid | `rgt verify --parents date-start,date-end --operation DATE_DIFF --result 45 --result-unit days` | 3.497 ms |
| DATE_DIFF verify invalid | `rgt verify --parents date-start,date-end --operation DATE_DIFF --result 0` | 3.512 ms |
| DURATION_SUM derive valid | `rgt derive --parents "$D26" --operation DURATION_SUM --unit days` | 4.892 ms |
| DURATION_SUM derive invalid | `rgt derive --parents "$D26" --operation DURATION_SUM --result 0` | 3.632 ms |
| DURATION_SUM verify valid | `rgt verify --parents "$D26" --operation DURATION_SUM --result 702 --result-unit seconds` | 4.123 ms |
| DURATION_SUM verify invalid | `rgt verify --parents "$D26" --operation DURATION_SUM --result 0` | 5.799 ms |
| DURATION_AVG derive valid | `rgt derive --parents "$D26" --operation DURATION_AVG --unit days` | 5.311 ms |
| DURATION_AVG derive invalid | `rgt derive --parents "$D26" --operation DURATION_AVG --result 0` | 4.167 ms |
| DURATION_AVG verify valid | `rgt verify --parents "$D26" --operation DURATION_AVG --result 27 --result-unit seconds` | 3.913 ms |
| DURATION_AVG verify invalid | `rgt verify --parents "$D26" --operation DURATION_AVG --result 0` | 4.024 ms |
| query unit valid | `rgt query duration-one-second --unit minutes` | 3.767 ms |
| query unit invalid | `rgt query plain-number --unit minutes` | 3.636 ms |

All measured p95 values were below the one-second target; the maximum was 5.799 ms. This is a local measurement, not a timing assertion or hosted workflow result.
