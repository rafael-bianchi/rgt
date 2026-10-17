# Quickstart: Validate Turtle Provenance Export

This is a post-implementation validation guide for [the export contract](contracts/turtle-export-contract.md). It creates temporary local projects; it does not use a network service or modify a production store. The shell example is for macOS/Linux; the contract tests cover Windows behavior.

## Prerequisites

- Build the feature on a Rust toolchain supported by this repository.
- Have the planned `test_turtle_contract` and migration tests available after implementation.
- Use a disposable working directory. The commands below do not run `rgt init` or install agent hooks.

## 1. Record and export one project

From the repository root:

```sh
cargo build
RGT_BIN="$(pwd)/target/debug/rgt"
DEMO_DIR="$(mktemp -d)"
cd "$DEMO_DIR"
printf 'revenue,120\ncost,20\n' > data.csv
"$RGT_BIN" record data.csv
"$RGT_BIN" graph --format text
"$RGT_BIN" graph --format ttl > graph.ttl
```

Expected: the text graph shows the recorded root IDs. `graph.ttl` is valid Turtle, contains two value entities with the same local IDs, and shows `data.csv` as a project-relative source path. It contains no absolute project or source path.

Copy the two root IDs from the text graph into `REVENUE_ID` and `COST_ID`, then run:

```sh
"$RGT_BIN" derive --parents "$REVENUE_ID,$COST_ID" --operation EXPRESSION --expression 'a - b' --result 100
"$RGT_BIN" graph --format ttl > derived.ttl
```

Expected: the result entity has `prov:wasDerivedFrom` links to both parents, is `prov:wasGeneratedBy` a derivation activity, and the activity `prov:used` the parents. Qualified usages preserve the recorded operand order. The operation and expression appear as RGT terms. See the [data model](data-model.md) for the evidence rule that permits an activity.

## 2. Check capture attribution and deduplication

Run the same supported hook capture twice, then capture through a second agent:

```sh
printf '%s\n' '{"event":"PostToolUse","tool_name":"Read","tool_input":{"path":"data.csv"},"tool_response":{"content":"revenue,120\ncost,20\n"}}' | "$RGT_BIN" hook post --agent claude-code
printf '%s\n' '{"event":"PostToolUse","tool_name":"Read","tool_input":{"path":"data.csv"},"tool_response":{"content":"revenue,120\ncost,20\n"}}' | "$RGT_BIN" hook post --agent claude-code
printf '%s\n' '{"event":"PostToolUse","tool_name":"Read","tool_input":{"path":"data.csv"},"tool_response":{"content":"revenue,120\ncost,20\n"}}' | "$RGT_BIN" hook post --agent cursor
"$RGT_BIN" graph --format ttl > captured.ttl
```

Expected: each recorded root value has one capture activity associated with Claude Code and one with Cursor, even though Claude Code captured it twice. The agents are linked to capture activities, not asserted as source authors. A fresh export process still sees the associations.

## 3. Check path disclosure

Use a source outside the project, then compare default and explicit full-path output:

```sh
OUTSIDE_FILE="$(mktemp)"
printf 'external,7\n' > "$OUTSIDE_FILE"
"$RGT_BIN" record "$OUTSIDE_FILE"
"$RGT_BIN" graph --format ttl > private.ttl
"$RGT_BIN" graph --format ttl --include-absolute-paths > full-paths.ttl
```

Expected: `private.ttl` identifies the outside source without its absolute path in an identifier or literal. `full-paths.ttl` includes the available absolute source path as a literal. The value and source IRIs are unchanged between the two exports.

## 4. Run contract and migration checks

From the repository root after implementation:

```sh
cargo test --test test_turtle_contract
cargo test --test test_turtle_migration
cargo fmt --all -- --check
cargo clippy --all-targets
```

The contract test must parse generated output with an independent Turtle parser and compare graph facts for empty output, typed values, escaped text, ambiguous derivations, repeated captures, matching local IDs in two independent projects, unchanged repeated exports, each Turtle quota, and controlled deadline failures. The migration test must open a version-1 store, preserve its existing values, add the project token and capture table, and keep historical short IDs readable. Build the release binary and run the ignored real-pipe probe locally after implementation:

```sh
cargo build --release --locked
RGT_BENCH_BIN="$(pwd)/target/release/rgt" cargo test --test test_turtle_performance blocked_stdout_pipe -- --ignored
```

The release latency gate must include process startup and output to a local file. Run `release_latency_boundaries` on a standard GitHub-hosted x86_64 `ubuntu-24.04` runner and record its actual image, CPU, memory, and Rust version. One warm-up and ten measured runs of the contract's fixed 10,000-value graph and exact value, edge, and byte limit fixtures must each succeed in under one second. Fixtures one unit over each limit must fail within one second. The separate `blocked_stdout_pipe` probe must exit nonzero within the deadline on Linux, macOS, and Windows; run the planned on-demand `.github/workflows/turtle-performance.yml` matrix to cover all three. The [contract](contracts/turtle-export-contract.md) defines the fixture construction and expected outcomes.

## Cleanup

The demonstration files are under `DEMO_DIR` and `OUTSIDE_FILE`. Remove those disposable paths after inspection. No hook installation is performed by this guide.
