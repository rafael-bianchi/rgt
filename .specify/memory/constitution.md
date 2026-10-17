<!--
--- SYNC IMPACT REPORT ---
Version Change: 1.5.1 -> 1.6.0
Modified Principles:
  - Principle VI — updated Active surface to include the embedded stdio MCP server (`rgt mcp`, Spec 041) alongside CLI subcommands
Added Sections: None
Removed Sections: None
Templates Status:
  - .specify/templates/plan-template.md: ✅ verified
  - .specify/templates/spec-template.md: ✅ verified
  - .specify/templates/tasks-template.md: ✅ verified
  - .specify/templates/checklist-template.md: ✅ verified
Follow-up TODOs: None
--------------------------
-->

# RGT (Rust Graph Tracker) Constitution

## Core Principles

### I. Rust-Only Single Binary (Zero Runtime Dependencies)
RGT MUST be implemented purely in Rust and compiled into a single, standalone binary with zero runtime dependencies (no external dynamic system library dependencies outside standard libc/OS kernel, no runtime engines like Node.js or Python). Cross-compilation support MUST target macOS (x86_64, aarch64), Linux (x86_64, aarch64), and Windows (x86_64).

*Rationale*: Guarantees instantaneous startup times, maximum portability across environments, and effortless deployment without requiring pre-installed runtimes.

**Addendum (2026-08-15, v1.5.0) — Agent-Side Glue Exemption**: Principle I prohibits runtime engines **within the RGT binary**. Agent-side glue artifacts (thin TypeScript/Python plugin files, rules files) that RGT writes to an agent's config path at `rgt init` time, that invoke only the `rgt` CLI as a subprocess and contain zero business logic, are exempt — they are not part of RGT's runtime or distribution. The binary itself MUST remain pure Rust with zero runtime dependencies; glue MUST remain thin delegates per Principle VIII.

### II. Speed-First Two-Tier Detection & BLAKE3 Hashing
File-change detection MUST employ a strict two-tier verification strategy:
1. Tier 1: Evaluate file modification timestamp (`mtime`) and file size (`size`).
2. Tier 2: Compute a BLAKE3 content hash ONLY when Tier 1 indicates a potential change or when a full integrity scan is explicitly requested.

The "no-change" execution path MUST complete in under 1 millisecond (1ms) per file.

*Rationale*: High-frequency file monitoring and developer/agent feedback loops cannot tolerate file I/O bottlenecks; Tier 1 filtering eliminates unnecessary cryptographic hashing for unchanged files.

### III. Graph Correctness & Minimal Subgraph Invalidation
Dependency tracking MUST use `petgraph` with reverse-edge traversal capabilities. When node changes occur, cache/state invalidation cascades MUST touch strictly the affected downstream subgraph. Full graph rescans during incremental updates are strictly prohibited.

*Rationale*: Bounds invalidation performance to O(k) relative to the affected downstream subgraph size k, rather than O(N) full graph size N.

### IV. First-Class Rich Value Types & Temporal Derivations
Graph nodes MUST support native `Number`, `Date`, and `Duration` value types powered by `chrono`. Date differentials (`date-diffs`) MUST be implemented as first-class derived values within the computation graph rather than ad-hoc string formatting or external calculations.

*Rationale*: Dependency staleness, execution intervals, and time-aware graph transformations require exact temporal arithmetic natively in the graph engine.

### V. Multi-Channel Distribution & Frictionless UX (RTK Alignment)
Installation channels MUST mirror RTK UX by providing:
- Shell installer script (`curl -fsSL ... | sh`)
- Homebrew formula (main repo: `brew install rafael-bianchi/rgt/rgt`)
- Cargo install from Git (`cargo install --git https://github.com/rafael-bianchi/rgt`)

RGT MUST include a one-command initialization (`rgt init -g`) that automatically detects environment capabilities and configures integration hooks for Claude Code, Cursor, Codex CLI, and Windsurf.

*Rationale*: Zero-friction installation and automated developer environment configuration maximize adoption and ensure consistent telemetry across AI coding tools.

### VI. Dual Passive/Active Integration Surface (CLI-First & Embedded MCP)
RGT MUST expose two complementary operational interfaces:
1. Passive: `PreToolUse` and `PostToolUse` execution hooks for automatic background event and state capture without explicit manual triggers.
2. Active: CLI subcommands (`record`, `derive`, `query`, `status`, `graph`, `verify`, `gc`, `doctor`, `init`) and the embedded stdio MCP server (`rgt mcp`, Spec 041) invoked by hooks, agents, or directly by users for on-demand recording, analysis, lineage inspection, and verification.

*Rationale*: Passive hooks guarantee complete event capture in the background, while CLI subcommands and the embedded stdio MCP server provide clean, composable active surfaces backed by the same single-binary Rust implementation.

### VII. Permissive Open-Source Governance
RGT MUST be licensed under permissive open-source terms: MIT OR Apache-2.0. All source code, build scripts, and documentation MUST reside in a public GitHub repository. Third-party dependencies MUST use licenses compatible with MIT OR Apache-2.0.

*Rationale*: Encourages community contributions, downstream integration into open or proprietary tools, and transparent development.

### VIII. CLI-First Hook Architecture (RTK Pattern)
Hooks MUST be thin delegates — shell scripts or agent-native glue (e.g., TypeScript/Python plugins per the Principle I Agent-Side Glue Exemption) that parse agent-specific JSON from stdin, call RGT CLI subcommands, and format agent-specific responses to stdout. All business logic (verification, rewriting, filtering, provenance analysis) MUST reside in the Rust binary and be accessible via CLI subcommands. Hook scripts and glue MUST NOT contain computation, validation, or filtering logic beyond JSON parsing and CLI invocation.

*Rationale*: Following RTK's `hooks/` directory pattern, thin shell scripts ensure a single source of truth for all logic (the Rust binary), simplify testing (CLI commands can be unit-tested independently), and eliminate duplication across agent integrations. When an agent's API changes, only the JSON parsing glue in the hook script needs updating — the core logic remains stable.

### IX. Test-Driven Verification
New behavior MUST be covered by tests that demonstrate intended usage, edge cases, and error paths, following patterns already present in `tests/unit/`, `tests/contract/`, and `tests/integration/`. Tests MUST be deterministic — no flaky dependencies on wall-clock time, filesystem state, or network availability. When modifying behavior, existing tests MUST be updated to reflect new expectations; regressions MUST be caught before merge.

Any change that adds, removes, or alters user-facing behavior (commands, flags, APIs, error semantics) MUST include corresponding test coverage. Test-only PRs are encouraged for clarifying expected behavior before implementation.

*Rationale*: RGT's provenance graph and verification engine are correctness-critical. Deterministic, comprehensive tests are the only reliable guard against silent data corruption and regression.

### X. Documentation Matches Behavior
Documentation MUST track user-visible behavior. The README, AGENTS.md, and spec files under `specs/` are the canonical references for what RGT does. Any change that adds, removes, or modifies features, commands, flags, or APIs that users rely on MUST update the corresponding documentation in the same changeset.

When implementing such changes:
- Identify which documentation files describe the affected behavior.
- Update those files to match, or explicitly add TODO markers indicating required documentation updates.
- Never introduce or remove user-facing behavior without considering its documentation impact.

*Rationale*: RGT serves AI coding agents that rely on its documented behavior. Silent documentation drift causes agent errors, broken integrations, and user confusion. Docs and code are equally part of the interface contract.

### XI. Code Quality & Idiomatic Rust
Code MUST follow idiomatic Rust conventions: clear naming, strong typing, predictable error handling via `Result<T, E>`, and zero unhandled panics on malformed inputs. Formatting and linting MUST be enforced via `cargo fmt` and `cargo clippy` — CI MUST reject PRs that fail either check.

New dependencies MUST be justified: prefer standard library solutions, then existing crate ecosystem choices already in the project, before introducing new dependencies. Every new dependency MUST be licensed compatibly with MIT OR Apache-2.0.

Error messages MUST help users recover: explain what went wrong, what was expected, and how to fix it. CLI commands MUST exit with meaningful status codes (0 = success, 1 = expected error, 2 = invalid input). Silent failures are prohibited.

*Rationale*: A single-binary tool with no runtime dependencies must be self-contained and self-documenting. Idiomatic Rust and clear error messages are the user's only debugging tools at runtime.

## Architecture & Performance Standards

- **Memory Safety & Concurrency**: RGT MUST adhere to strict idiomatic Rust standards with zero unhandled panics on malformed graph inputs.
- **Latency Budget**: All CLI operations MUST complete within sub-second bounds, with graph query latency targeting <10ms for typical repositories (<100k nodes).
- **Cross-Platform Parity**: Path handling, file watching, and hook scripts MUST maintain strict cross-platform compatibility across macOS, Linux, and Windows.
- **Small, Composable Modules**: Code MUST be organized into focused modules with explicit data flows. Prefer explicit state passing over global mutable state. Each module in `src/` MUST have a single, clear responsibility.
- **CLI Consistency**: Commands and flags MUST be discoverable and consistent. Breaking changes to the CLI surface require strong justification and clear migration notes. Default behavior MUST be safe and predictable; advanced or destructive operations MUST require explicit opt-in flags.

## Integration & Tooling Policy

- **Hook Reliability**: Passive hooks (`PreToolUse`/`PostToolUse`) MUST execute asynchronously or fail open with non-blocking error handling to ensure host developer tool performance is never impacted.
- **Tooling Enforcement**: `cargo fmt --all -- --check` and `cargo clippy --all-targets` MUST pass on every PR. CI MUST reject PRs that fail either check.

## Development Workflow

- **Trunk-Based Development**: Feature and bugfix work MUST branch from `develop`, with PR targets always set to `develop`. Direct commits or PRs to `main` are prohibited — promotion from `develop` to `main` is handled automatically by the release-please CD workflow.
- **Branch Naming**: Branches MUST follow the pattern `<type>/<description>`, where type is one of: `fix`, `feat`, `style`, `docs`, `chore`. Examples: `fix/issue-1-provenance-query-bfs`, `feat/issue-2-agent-hooks`.
- **Merge Strategy**: PRs MUST use merge commits (`--merge`), with the source branch deleted after merge (`--delete-branch`). Squash and rebase strategies are prohibited for feature branches.
- **Commit Conventions**: All commits MUST follow Conventional Commits format: `type: description (#N)`. Accepted types: `feat`, `fix`, `docs`, `style`, `refactor`, `test`, `chore`. GitHub issues MUST be referenced via `(#N)` suffix in commit messages and auto-linked via `Closes #N` or `Fixes #N` in PR bodies.
- **GitHub Identity**: Commits MUST use the GitHub noreply email (`ID+USER@users.noreply.github.com`) for both author and committer fields to avoid push restrictions from private email settings. The email is obtained via `gh api user --jq '"\(.id)+\(.login)@users.noreply.github.com"'`.
- **CI/CD Gating**: All PRs MUST pass CI checks (fmt, clippy, test on ubuntu/macOS/Windows, security) before merge. The CD workflow requires the repository setting "Allow GitHub Actions to create and approve pull requests" to be enabled.
- **Incremental Changes**: PRs MUST be reviewable in size and scope. Large, unstructured refactors MUST be split into logical, incremental steps, each independently testable and mergeable.
- **Documentation in Changesets**: PRs that change user-facing behavior MUST include documentation updates in the same changeset, or explicitly tag follow-up work via linked issues.

## Governance

- **Supremacy**: This Constitution supersedes all other documentation, architectural decision records (ADRs), or PR suggestions for RGT.
- **Amendments**: Amendments require explicit documentation of rationale, a bump to the constitution version according to SemVer rules, and verification that dependent artifacts and templates remain aligned.
- **Compliance**: All pull requests, code reviews, and releases MUST be verified against these core principles. Reviewers MUST flag violations and request alignment before approval.
- **New Contributors**: The repository MUST remain approachable to new contributors. Project structure MUST be clear, entry points easy to find, and contribution guidelines visible in CONTRIBUTING.md.

**Version**: 1.6.0 | **Ratified**: 2026-07-31 | **Last Amended**: 2026-10-17
