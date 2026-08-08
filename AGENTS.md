# RGT Development Guide

## Project Identity

**RGT (Rust Graph Tracker)** gives AI coding agents (Claude Code, Cursor, Codex CLI, Windsurf) a persistent, queryable memory of every number and date they read from source files or derive through calculations — then **verifies each derivation is mathematically correct** — ensuring they never silently reason from stale or incorrect data.

- **Repository**: https://github.com/rafael-bianchi/rgt
- **License**: MIT OR Apache-2.0
- **Version**: 0.1.0

## Tech Stack

| Category | Choice |
|---|---|
| Language | Rust Edition 2021 |
| Binary | Single binary with LTO (`lto=true`, `codegen-units=1`, `panic=abort`, `strip=true`) |
| Graph engine | `petgraph::graph::DiGraph` with reverse-edge traversal |
| Storage | `rusqlite` (bundled SQLite, WAL mode) → `.rgt/store.db` |
| Hashing | `blake3` (Tier 2 change detection) |
| Date/time | `chrono` (DateTime<Utc>, Duration, `serde` feature) |
| CLI | `clap` (derive mode) |
| Eval expression | `evalexpr` (MIT) for arithmetic evaluation |
| HTTP | `ureq` (sync, for self-update) |
| Archiving | `flate2`, `tar`, `zip` (self-update extraction) |
| Regex | `regex` (value extraction from source files) |
| Hashing (update) | `sha2` (SHA-256 for release verification) |

## Module Map

```
src/
├── cli/           # CLI subcommands: init, status, query, graph, update
├── query/         # Query module: provenance (BFS), stale listing
├── store/         # SQLite: db (connection), queries, schema
├── types/         # Data types: node (TrackedNode, SourceDocument), edge, value (ValueData)
├── graph/         # DAG: engine (petgraph), invalidation (cascade)
├── detection/     # File change: tier1 (mtime+size), tier2_blake3
├── hooks/         # Agent hooks: mod (PreToolUse/PostToolUse), installer, parser
├── updater/       # Self-update: atomic replace, checksum, github API, platform
├── lib.rs         # Library root
└── main.rs        # Binary entry point, Commands enum (7 variants)
```

```
tests/
├── contract/      # Schema validation: hooks, update (3 files)
├── integration/   # E2E flows: derivation_chain, file_invalidation, quickstart, distribution, installer (6 files)
└── unit/          # Isolated logic: date_diff, invalidation (2 files)
```

## Branching Model

Trunk-based development:

```
develop ──→ fix/issue-N-description ──→ PR to develop ──→ merge to develop
                                                                    │
                                                    release-please auto-promotes to main
```

| Rule | Details |
|---|---|
| Branch from | `develop` |
| PR target | `develop` (never `main` directly — enforced by `pr-target-check` workflow) |
| Promote to `main` | Handled automatically by release-please CD workflow |
| Branch naming | `fix/<desc>`, `feat/<desc>`, `style/<desc>`, `docs/<desc>` |

```bash
git checkout develop
git checkout -b fix/issue-N-short-description
# ... work ...
git push -u origin fix/issue-N-short-description
gh pr create --base develop --head fix/issue-N-short-description --title "..." --body "..."
gh pr merge N --merge --delete-branch
```

## Commit Conventions

Use **Conventional Commits**: `type: description (#N)`

| Type | Use case |
|---|---|
| `feat` | New feature |
| `fix` | Bug fix |
| `docs` | Documentation |
| `style` | Formatting, whitespace (no logic change) |
| `refactor` | Code restructuring (no behavior change) |
| `test` | Adding or updating tests |
| `chore` | Build, CI, tooling |

Reference GitHub issues with `(#N)` suffix. Use `Closes #N` or `Fixes #N` in PR body to auto-link.

## GitHub Configuration

Commits must use the GitHub noreply email to avoid push blocks from private email settings:

```bash
# Get your noreply email:
gh api user --jq '"\(.id)+\(.login)@users.noreply.github.com"'

# Commit with noreply (both author and committer):
GIT_COMMITTER_NAME="Your Name" \
GIT_COMMITTER_EMAIL="ID+USER@users.noreply.github.com" \
git commit --author="Your Name <ID+USER@users.noreply.github.com>"
```

## CI/CD

### CI (`ci.yml`) — triggers on PR to `develop`/`main`

| Job | Dependencies | Description |
|---|---|---|
| `fmt` | — | `cargo fmt --all -- --check` |
| `clippy` | fmt | `cargo clippy --all-targets` |
| `test` (×3) | clippy | `cargo test --all` on ubuntu, macos, windows |
| `security` | clippy | cargo-audit, critical files check, dangerous patterns, dependency audit |
| `test presence` | — | Verifies new `.rs` files have tests (PRs to `develop` only) |

### CD (`cd.yml`) — triggers on push to `develop`/`main`

| Job | Branch | Action |
|---|---|---|
| `pre-release` | `develop` | Builds pre-release binaries via `release.yml` |
| `release-please` | `main` | googleapis/release-please-action@v4 (Rust), creates release PRs |
| `build-release` | `main` (on release_created) | Builds cross-platform binaries, DEB, RPM, checksums |
| `update-latest-tag` | `main` | Updates `latest` tag pointer |

**Prerequisite**: Repository setting "Allow GitHub Actions to create and approve pull requests" must be enabled (Settings > Actions > General).

### Local checks before pushing

```bash
cargo fmt --all -- --check
cargo clippy --all-targets
cargo test --all
```

## Test Conventions

### CWD Isolation

All integration and contract tests must guard `std::env::set_current_dir` with a static `Mutex` held for the entire test duration. This prevents parallel tests from racing on `.rgt/store.db`:

```rust
use std::sync::Mutex;

static CWD_MUTEX: Mutex<()> = Mutex::new(());

fn set_cwd(dir: &std::path::Path) -> std::sync::MutexGuard<'static, ()> {
    let guard = CWD_MUTEX.lock().unwrap();
    std::env::set_current_dir(dir).unwrap();
    guard
}

#[test]
fn test_example() {
    let dir = tempdir().unwrap();
    let _guard = set_cwd(dir.path()); // lock held until test ends
    // ... test body uses handlers/CLI with CWD pointing to tempdir
}
```

- Every test binary that calls `set_current_dir` needs its own `static CWD_MUTEX`
- Use `tempfile::tempdir()` for isolated `.rgt/store.db`
- Run locally with `cargo test -- --test-threads=1` for deterministic execution
- Test categories: `tests/contract/` (schema validation), `tests/integration/` (E2E), `tests/unit/` (isolated logic)

## Constitution (Non-Negotiable)

All design decisions must comply with the 7 principles in `.specify/memory/constitution.md` v1.0.0:

| # | Principle | Constraint |
|---|---|---|
| I | Rust-Only Single Binary | Zero runtime deps, cross-compile macOS/Linux/Windows |
| II | Two-Tier Detection & BLAKE3 | Tier 1 (mtime+size) <1ms, Tier 2 (BLAKE3) only on change |
| III | Graph Correctness | `petgraph` reverse-edge DAG, O(k) invalidation, no full rescans |
| IV | Rich Value Types | Native `Number`/`Date`/`Duration` via `chrono` |
| V | Multi-Channel Distribution | Shell script, Homebrew, `cargo install`, `rgt init -g` |
| VI | Dual Integration Surface | Passive hooks (fail-open) + active CLI subcommands |
| VII | Permissive OSS | MIT OR Apache-2.0, public GitHub |

## CLI Commands

| Command | Description |
|---|---|
| `rgt init [-g] [--force]` | Auto-configure hooks for Claude Code, Cursor, Codex, Windsurf |
| `rgt status [--stale-only] [--json]` | Inspect provenance graph: total/active/stale nodes |
| `rgt query <node_id> [--json]` | Query complete derivation lineage for a value node |
| `rgt graph [-f, --format text\|mermaid\|dot]` | Visualize provenance DAG |
| `rgt hook <pre\|post>` | Execute passive hook (reads stdin for tool event JSON) |
| `rgt update [--check] [-y] [--version <tag>]` | Self-update from GitHub Releases |
| `rgt verify --parents <ids> --operation <op> --result <val> [--expression <expr>]` | Verify a derived value against parent nodes (EXPRESSION/DATE_DIFF); exit 0=match, 1=mismatch, 2=invalid |

## Speckit Workflow

Feature work follows the Speckit pipeline via `.kilo/commands/speckit.*.md`:

```
/speckit-specify → /speckit-plan → /speckit-tasks → /speckit-analyze → /speckit-implement
```

**Artifacts per feature** (`specs/<NNN>-<name>/`):

| File | Phase | Purpose |
|---|---|---|
| `spec.md` | specify | User stories, functional requirements, success criteria |
| `plan.md` | plan | Technical context, constitution check, project structure |
| `research.md` | plan | Architectural decisions, alternatives considered |
| `data-model.md` | plan | Entities, schema, state transitions |
| `contracts/` | plan | Interface schemas (CLI, hooks) |
| `quickstart.md` | plan | Validation scenarios |
| `tasks.md` | tasks | Implementation tasks by user story |
| `checklists/` | specify | Quality validation checklist |

**Configuration** (`.specify/init-options.json`):
- AI: kilocode
- Feature numbering: sequential (001, 002, ...)
- Template directory: `.specify/templates/`

## Distribution

| Method | Command |
|---|---|
| Shell | `curl -fsSL https://raw.githubusercontent.com/rafael-bianchi/rgt/main/install.sh \| sh` |
| Homebrew | `brew install rafael-bianchi/rgt/rgt` |
| Cargo | `cargo install --git https://github.com/rafael-bianchi/rgt` |
| From source | `cargo build --release` |

Release artifacts (generated by CD): macOS x86_64/aarch64, Linux musl/gnu x86_64/aarch64, Windows x86_64, `.deb`, `.rpm`, checksums, Homebrew formula.

## Suppressed Files

See `.gitignore` for the full list. Key entries:

| Pattern | Reason |
|---|---|
| `/target` | Rust build artifacts |
| `.rgt/store.db`, `.rgt/*.log` | Local provenance DB + logs |
| `.DS_Store`, `Thumbs.db` | OS files |
| `.specify/`, `specs/` | Excluded from crate via `Cargo.toml exclude` |
