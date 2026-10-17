# Turtle Export Validation

Date: 2026-09-25

## Local validation

- Feature regression targets after convergence: 7 identity tests, 8 Turtle CLI
  contract tests, and the checked-duration unit regression passed.
- Full suite: `cargo test --locked` passed with 457 passed and 2 ignored across
  35 targets.
- Formatting: `cargo fmt --all -- --check` passed.
- Lint: `cargo clippy --locked --all-targets -- -D warnings` passed with no warnings.
- Release build: `cargo build --release --locked` passed.
- Convergence regressions passed: out-of-range `i64::MIN`/`i64::MAX` duration
  rows return checked errors with zero CLI stdout; identity initializes and
  validates on new, legacy, copied, and malformed stores; and an independent
  Turtle parser accepts the zero-edge derived-node export without activity claims.
- Quickstart scenarios passed in a temporary project: record values, retain
  text graph output, export Turtle, verify a two-parent derivation, capture
  repeated Claude Code and Cursor events, check default path privacy and
  explicit absolute-path output, and verify invalid flag exit code 2.
- Release latency and exact quota-boundary probe passed on local macOS for all
  ten measured runs per fixture, including 10,000 values, 30,000 edges, a
  representative graph, and exactly 64 MiB of Turtle. Every over-limit fixture
  failed with zero stdout in under one second.
- Release blocked-pipe probe passed on local macOS: 3,501,058 rendered bytes,
  688 ms until nonzero exit, 65,536 partial bytes, and the final-write deadline
  error. Hardware, toolchain, and per-run measurements are in
  [the performance report](../../docs/turtle-export-performance.md).

## Remaining cross-platform gate

The GitHub Actions workflow now has a push trigger scoped to
`feat/prov-o-turtle-export` plus its manual dispatch trigger. The branch has not
been published and no hosted run has been recorded. `T028`, `T033`, and `T034`
remain incomplete until the pinned x86_64 Ubuntu latency probe and blocked-pipe
probes on Ubuntu, macOS, and Windows are recorded from a hosted run. The local
Apple M5 results do not establish the hosted release decision.

## Checklist state

The built-in specification requirements checklist passes. The separate
reviewer-owned Turtle export requirements checklist remains unchecked and was
not modified by implementation.
