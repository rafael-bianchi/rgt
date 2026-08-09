# Documentation Contract

**Feature**: `012-documentation-reconciliation` | **Date**: 2026-08-08

## Purpose

Defines the required relationship between RGT code and its documentation. All docs MUST describe actual behavior present in code — never speculative features.

## Doc Comment Contract (code)

Every `pub fn` in `src/` MUST have a `///` doc comment with:

1. A one-line summary of the function's purpose.
2. `# Arguments` section when parameters are non-obvious (referenced as `` `name` ``).
3. `# Returns` / error conditions when the function returns `Result`.

Array-index references (e.g., `parent[0]`) MUST be backticked: `` `parent[0]` ``.

**Verification**: `grep -rn "^pub fn" src/ --include="*.rs"` — every match must have a preceding `///` line. `cargo doc --no-deps` must build with zero warnings.

## CLI Documentation Contract (README/AGENTS)

The README MUST document all 8 subcommands from `src/main.rs` with matching flags:

| Command | README requirement |
|---|---|
| `init` | `-g`, `--force`, `--agent` |
| `status` | `--stale-only`, `--json` |
| `query` | `<node_id>`, `--json` |
| `graph` | `-f text\|mermaid\|dot` |
| `hook` | `<pre\|post>` |
| `mcp` | present |
| `update` | `--check`, `-y`, `--version` |
| `verify` | `--parents`, `--operation`, `--expression`, `--result` |

## Install Path Contract

All documentation MUST use the current install commands:

- `curl -fsSL https://raw.githubusercontent.com/rafael-bianchi/rgt/main/install.sh | sh`
- `brew install rafael-bianchi/rgt/rgt`
- `cargo install --git https://github.com/rafael-bianchi/rgt`

**Prohibited**: `rafael-bianchi/tap`, bare `cargo install rgt`, `rafael-bianchi/homebrew-tap`.

## Tooling Directory Contract

Local tooling directories (`.kilo/`, `.kilocode/`, `.specify/`, `.agents/`) MUST be in `.gitignore` and NEVER committed.

## Behavior-Change Contract

Any future change that adds/removes/modifies CLI behavior MUST update README (and AGENTS.md where relevant) in the same changeset. Unverified claims MUST be marked with `TODO: maintainer to confirm`.
