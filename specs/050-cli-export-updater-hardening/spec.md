# Feature Specification: Spec 050 - CLI, Turtle Export, Self-Updater, & Infrastructure Hardening

## User Scenarios

### User Story 1: `rgt status --stale-only` Filtering
**Given** a project repository with both active (fresh) and stale nodes,
**When** the user runs `rgt status --stale-only`,
**Then** RGT text output displays only stale nodes (or "✓ No stale nodes found." if none are stale) without printing the total/active node summary lines,
**And** when run with `rgt status --stale-only --json`, it outputs a JSON object containing `stale_nodes` and `stale_details`.

### User Story 2: `rgt query` Duration Plain-Text Display
**Given** a Duration node queried via `rgt query <node_id> --unit <unit>`,
**When** plain-text output is generated,
**Then** the human-readable text includes exact `duration_seconds` alongside `selected_unit_display` so users and agents can read both the display text and exact calculation seconds.

### User Story 3: Turtle Export Project Relative Path Preservation & Deadline Alignment
**Given** a project with recorded files,
**When** a tracked file has been deleted from disk (`is_stale = true`, `stale_reason = "FILE_DELETED"`),
**Then** `source_location` in Turtle export still preserves `rgt:relativePath` if the recorded path is relative and within the project without escaping parent directory components,
**And** `main.rs` sets `command_deadline` for graph export to 1000ms (1 second), matching `TURTLE_EXPORT_TIMEOUT`.

### User Story 4: Shell Installer & Platform Target Hardening
**Given** `install.sh` running in any POSIX environment,
**When** `checksums.txt` fails to download or cannot be verified,
**Then** `install.sh` exits with error code 2 immediately instead of silently continuing unverified,
**And** binary installation uses atomic replacement (`cp` to `.tmp` followed by `mv -f`) to prevent Linux `ETXTBSY` text file busy errors on active binaries,
**And** in `src/updater/platform.rs`, 32-bit `x86` is treated as unknown/unsupported rather than falsely aliased to `Arch::X86_64`, and the fallback target triple format uses `{arch}-{os}` instead of `{os}-{arch}`.

### User Story 5: Build Hygiene and Crate Re-compilation Elimination
**Given** the RGT project,
**When** compiling `rgt` binary,
**Then** `src/main.rs` imports library modules via `use rgt::*` instead of declaring separate file `mod` items, avoiding double-compilation of the entire crate.
