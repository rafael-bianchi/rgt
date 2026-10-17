# Feature Specification: Remove Unused sha2 Dependency

**Feature Branch**: `007-remove-unused-sha2`

**Created**: 2026-08-01

**Status**: Draft

**Input**: User description: "chore: remove unused sha2 dependency. sha2 = \"0.10\" is listed as a dependency in Cargo.toml but RGT uses blake3 for all content hashing. Unused dependencies add binary size, increase compile time, and create unnecessary audit surface for supply-chain security reviews."

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Confirm dependency usage and remove if unused (Priority: P1)

A maintainer verifies whether `sha2` is actively used in the codebase. If confirmed unused, the dependency is removed to reduce build times, binary size, and audit surface. If confirmed in use, the dependency is retained and no changes are necessary.

**Why this priority**: Removing truly unused dependencies improves supply-chain security posture and build performance. However, removing an actively-used dependency would break the self-update checksum verification.

**Independent Test**: Run `grep -r "sha2\|Sha256\|Sha512" src/` before and after changes. If zero references remain, removal was applied. If references exist, no removal is performed.

**Acceptance Scenarios**:

1. **Given** the repository contains `sha2 = "0.10"` in `Cargo.toml`, **When** a maintainer searches for sha2 usage in `src/`, **Then** the search returns references in `src/updater/checksum.rs`.
2. **Given** sha2 references exist in source code, **When** a maintainer confirms sha2 is actively relied upon by the updater module for SHA-256 checksum verification, **Then** no changes to `Cargo.toml` are made and the dependency is retained.
3. **Given** sha2 is retained as a dependency, **When** `cargo build --release` runs, **Then** compilation succeeds with sha2 as a normal dependency.
4. **Given** sha2 is retained, **When** `cargo test --all` runs, **Then** all tests including `src/updater/checksum.rs` unit tests pass.

---

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: System MUST verify whether `sha2` has any references in source code under `src/` before considering removal.
- **FR-002**: System MUST NOT remove `sha2` from `Cargo.toml` if any source file under `src/` references it.
- **FR-003**: System MUST retain `sha2 = "0.10"` in `Cargo.toml` since it is actively used by `src/updater/checksum.rs` for release binary checksum verification (a core part of the `rgt update` command).

### Key Entities

N/A — investigation-only task.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: `grep -r "sha2\|Sha256\|Sha512" src/` returns at least one reference confirming the dependency is in active use.
- **SC-002**: `cargo build --release` succeeds with sha2 present in Cargo.toml (no regressions).
- **SC-003**: `cargo test --all` passes with all updater checksum tests green.
- **SC-004**: Issue #4 is updated with investigation findings and closed as "not a bug" since sha2 is actively used.

## Assumptions

- The `sha2` crate is used exclusively by `src/updater/checksum.rs` for SHA-256 hash computation during `rgt update` release verification.
- Constitution Principle I (Rust-Only Single Binary) is not violated by retaining sha2 — it is a compile-time dependency that produces a single binary with no additional runtime requirements.
- The dependency serves a distinct purpose from `blake3`: BLAKE3 is used for Tier 2 file-change detection (Constitution Principle II), while SHA-256 is used for release artifact integrity verification, which is an industry-standard hash required by GitHub Releases checksum files.
