<!--
--- SYNC IMPACT REPORT ---
Version Change: 0.0.0 -> 1.0.0
Modified Principles:
  - Initialized initial constitution template with 7 non-negotiable principles for RGT (Rust Graph Tracker).
Added Sections:
  - Core Principles (Principles I through VII)
  - Architecture & Performance Standards
  - Integration & Tooling Policy
  - Governance
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
- Homebrew tap formula
- Standard `cargo install rgt`

RGT MUST include a one-command initialization (`rgt init -g`) that automatically detects environment capabilities and configures integration hooks for Claude Code, Cursor, Codex CLI, and Windsurf.

*Rationale*: Zero-friction installation and automated developer environment configuration maximize adoption and ensure consistent telemetry across AI coding tools.

### VI. Dual Passive/Active Integration Surface
RGT MUST expose two complementary operational interfaces:
1. Passive: `PreToolUse` and `PostToolUse` execution hooks for automatic background event and state capture without explicit manual triggers.
2. Active: A Model Context Protocol (MCP) server enabling AI agents to query the graph on demand, inspect node lineage, and analyze dependency subgraphs.

*Rationale*: Passive hooks guarantee complete event capture in the background, while the MCP server equips AI tools to actively reason over dependency structures.

### VII. Permissive Open-Source Governance
RGT MUST be licensed under permissive open-source terms: MIT OR Apache-2.0. All source code, build scripts, and documentation MUST reside in a public GitHub repository.

*Rationale*: Encourages community contributions, downstream integration into open or proprietary tools, and transparent development.

## Architecture & Performance Standards

- **Memory Safety & Concurrency**: RGT MUST adhere to strict idiomatic Rust standards with zero unhandled panics on malformed graph inputs.
- **Latency Budget**: All CLI operations MUST complete within sub-second bounds, with graph query latency targeting <10ms for typical repositories (<100k nodes).
- **Cross-Platform Parity**: Path handling, file watching, and hook scripts MUST maintain strict cross-platform compatibility across macOS, Linux, and Windows.

## Integration & Tooling Policy

- **Hook Reliability**: Passive hooks (`PreToolUse`/`PostToolUse`) MUST execute asynchronously or fail open with non-blocking error handling to ensure host developer tool performance is never impacted.
- **MCP Standards**: The MCP server MUST follow official Model Context Protocol specifications and JSON-RPC transport standards over stdio.

## Governance

- **Supremacy**: This Constitution supersedes all other documentation, architectural decision records (ADRs), or PR suggestions for RGT.
- **Amendments**: Amendments require explicit documentation of rationale, a bump to the constitution version according to SemVer rules, and verification that dependent artifacts and templates remain aligned.
- **Compliance**: All pull requests, code reviews, and releases MUST be verified against these core principles.

**Version**: 1.0.0 | **Ratified**: 2026-07-31 | **Last Amended**: 2026-07-31
