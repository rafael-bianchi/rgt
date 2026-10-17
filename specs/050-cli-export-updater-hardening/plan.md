# Implementation Plan: Spec 050 - CLI, Turtle Export, Self-Updater, & Infrastructure Hardening

## Overview
Hardens the remaining CLI ergonomics, Turtle export resilience, installer safety, platform mappings, and compilation hygiene identified in the review.

## Architecture & Implementation Steps

### 1. `src/cli/status.rs`
- In `execute_status_with`:
  - When `stale_only` is true:
    - If `json_output`: output `json!({ "stale_nodes": summary["stale_nodes"], "stale_details": summary["stale_details"] })`.
    - If text output:
      - If `stale_nodes.is_empty()`: print `"✓ No stale nodes found."`.
      - Else: print `"⚠️  Stale Nodes Found ({}):"` followed by the formatted list of stale nodes.

### 2. `src/cli/query.rs`
- In `render_lineage_text`:
  - If `step` contains `"duration_seconds"`, render `  duration_seconds: <secs>\n`.

### 3. `src/export/turtle.rs` & `src/main.rs`
- In `source_location`:
  - If `canonical` is `None` (file deleted from disk), but `path.is_relative()` and contains no `ParentDir` or `Prefix` components, set `relative_path = Some(path_to_turtle_string(path))`.
- In `src/main.rs`:
  - Pass 1000ms deadline to `execute_graph_with_options` instead of 400ms.

### 4. `install.sh` & `src/updater/platform.rs`
- In `install.sh`:
  - Make `checksums.txt` download mandatory unless explicitly skipped: failure to download exits 2.
  - In `install_binary`: write to temporary file `${dest}.tmp.$$` and use `mv -f` to replace destination atomically.
- In `src/updater/platform.rs`:
  - In `detect_arch()`: remove `"x86"` alias to `Arch::X86_64`.
  - In `build_target_triple()` fallback: format as `{arch}-{os}`.

### 5. `src/main.rs` & `Cargo.toml`
- Replace module declarations in `src/main.rs` with `use rgt::*;`.
- Run tests to verify single compilation unit and correctness.
