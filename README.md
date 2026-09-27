<p align="center">
  <img src="docs/rgt_crab.jpeg" alt="RGT: Rust Graph Tracker" width="200">
</p>

<p align="center">
  <strong>RGT: Rust Graph Tracker</strong>
</p>

<p align="center">
  <strong>Numeric and date provenance tracking for AI coding agents</strong>
</p>

<p align="center">
  <a href="https://github.com/rafael-bianchi/rgt/actions/workflows/ci.yml"><img src="https://github.com/rafael-bianchi/rgt/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
  <a href="https://github.com/rafael-bianchi/rgt/releases"><img src="https://img.shields.io/github/v/release/rafael-bianchi/rgt" alt="Release"></a>
  <a href="#license"><img src="https://img.shields.io/badge/License-Apache_2.0-blue.svg" alt="License: Apache-2.0"></a>
</p>

<p align="center">
  <a href="#installation">Install</a> &bull;
  <a href="#quick-start">Quick Start</a> &bull;
  <a href="#commands">Commands</a> &bull;
  <a href="#supported-ai-tools">Supported Agents</a> &bull;
  <a href="#how-it-works">How It Works</a> &bull;
  <a href="CONTRIBUTING.md">Contributing</a>
</p>

---

<p align="center">
  <a href="README.md">English</a> &bull;
  <a href="README_fr.md">Français</a> &bull;
  <a href="README_zh.md">中文</a> &bull;
  <a href="README_ja.md">日本語</a> &bull;
  <a href="README_ko.md">한국어</a> &bull;
  <a href="README_es.md">Español</a> &bull;
  <a href="README_pt.md">Português</a>
</p>

---

RGT gives AI coding agents a persistent, queryable graph of numeric and date values recorded from source files or derived through calculations, then **verifies each derivation is mathematically correct**. Single Rust binary, 13 supported AI coding tools, and integrations that never rewrite commands.

## What RGT Does

Agents reason from changing files and can make arithmetic mistakes. RGT tracks the provenance of every numeric value and re-checks the math.

| Operation | What RGT does |
|-----------|---------------|
| `rgt record <file>` | Extracts every number and date from a file into the provenance graph |
| `rgt status` | Reports total, active, and stale nodes, showing which values are still trustworthy |
| `rgt derive` | Verifies an agent-computed value against its parents **before** recording it |
| `rgt query <id>` | Traces a value's lineage back to its source files |
| `rgt graph` | Exports the dependency DAG (text, Mermaid, or DOT) |
| `rgt hook` | Passive capture for supported completed-read paths; other clients use manual recording guidance |

## Why Provenance Tracking Matters

RGT does not measure savings: it prevents silent errors. Two failure modes motivate it:

1. **Stale data**: an agent reads a file, later the file changes, and the agent keeps reasoning from the old numbers. RGT marks the affected root nodes **stale** and cascades staleness through every derived value that depends on them (`rgt status`).
2. **Wrong math**: an agent computes `revenue = price * quantity` and gets it wrong. RGT re-computes the expression from the parent values in the database and **rejects** mismatched derivations (exit 1) before they enter the graph.

When a supported client returns a complete read result that matches the current source snapshot, its hook can record the values with source lineage. Other clients may need `rgt record <file>`. Hooks never rewrite, filter, or block the agent's tool calls.

## Installation

### Quick Install (Linux/macOS)

```bash
curl -fsSL https://raw.githubusercontent.com/rafael-bianchi/rgt/main/install.sh | sh
```

> Installs to `~/.local/bin`. Add to PATH if needed:
> ```bash
> echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.zshrc  # or ~/.bashrc
> ```

### Homebrew (tap)

```bash
brew install rafael-bianchi/rgt/rgt
```

> The tap formula is refreshed by each GitHub release.

### Cargo

```bash
cargo install --git https://github.com/rafael-bianchi/rgt
```

### Pre-built Binaries

Download from [releases](https://github.com/rafael-bianchi/rgt/releases):
- macOS: `rgt-aarch64-apple-darwin.tar.gz`
- Linux: `rgt-x86_64-unknown-linux-musl.tar.gz` / `rgt-aarch64-unknown-linux-gnu.tar.gz`
- Windows: `rgt-x86_64-pc-windows-msvc.zip`

> Pre-built binaries for macOS, Linux, and Windows are published with each release.

### Self-Update

```bash
rgt update          # atomically replace the binary with the latest release
rgt update --check  # check for a new version without applying
```

### Verify Installation

```bash
rgt status   # Shows the provenance graph state (fresh store starts at 0 nodes)
```

## Quick Start

```bash
# 1. Configure provenance hooks for your AI tool
rgt init -g                 # auto-detect every installed supported agent
rgt init -g --agent copilot # or target one: copilot, gemini, vibe, opencode, pi, hermes, ...
rgt init --agent cline      # Cline/Roo, Antigravity, and Kilo use project guidance
rgt init --agent codex      # native PostToolUse hook plus AGENTS.md guidance
rgt init --agent windsurf   # native post_read_code hook plus .windsurfrules guidance

# 2. Restart your AI tool, then:
rgt record budget.csv       # agent reads a data file and records its values
rgt status                  # inspect the graph
rgt derive --parents node_raw_X,node_raw_Y --operation EXPRESSION --expression "a - b" --result 60000
                            # agent records a verified derivation
rgt query node_drv_Z        # trace the derived value's lineage
rgt graph --format mermaid  # export the dependency graph
rgt graph --format ttl      # export interoperable PROV-O Turtle
```

## How It Works

```
  Agent reads a data file                        Agent computes a derived value
            |                                                 |
            v                                                 v
  rgt hook post (native hook/plugin)               rgt derive --parents <ids>
            |                                                 |
            v                                        (verify against parents)
  rgt record extracts {path, content}                        |
            |                                                 v
            v                                          math correct?
  provenance graph ── rgt status ── stale?  ── NO ──> value trusted
            |                                   |
            +---------------- YES -------------->  node + dependents marked stale
```

Three strategies keep the graph trustworthy:

1. **Capture**: hooks and plugins can record a read only when the client supplies a successful completed result with full content that matches the current file. Installed registrations do not prove that capture works; see the [harness compatibility matrix](docs/compatibility/support-matrix.md) for each client and read path. For unverified or instruction-only paths, use `rgt record <file>`.

> **Non-text sources (PDF, Excel, images)**: RGT cannot extract all model-visible binary content locally. Capture depends on a supported client/read path and a complete result that matches the current source. For any unverified path or non-text source without a supported adapter, report the values explicitly with `echo "Amount: 1234.56" | rgt record --stdin`. Writing an intermediate text file does not itself guarantee capture; use `rgt record <file>` unless the compatibility matrix confirms automatic capture for that path. `rgt record` on a binary file returns an unsupported-format error.
2. **Verification**: every `rgt derive` is trust-but-verify: RGT re-computes the result from parent values and rejects wrong ones before insertion.
3. **Staleness**: when a source file changes, its nodes (and everything derived from them) are flagged stale, so the agent can be told to re-read. Change detection is two-tier (mtime+size, then BLAKE3) with a time-bounded forced re-hash, and distinguishes a genuinely deleted file (`FILE_DELETED`) from a permission/lock error (`CHECK_FAILED`, which does not mark values stale).

## Commands

### Initialize & Hooks
```bash
rgt --version                               # print the running build version
rgt version                                 # equivalent version subcommand
rgt init [-g] [--force] [--agent <name>] [--number-format <us|eu|auto>]  # configure hooks, detect installed agents
rgt hook pre|post [--agent <name>]         # passive capture from agent event JSON (stdin)
```

`--number-format` persists the locale used to parse numbers (`.`/`,` separators and signs) for the passive hook path. Valid values: `us` (`.` decimal, `,` thousands), `eu` (`,` decimal, `.` thousands), or `auto` (inferred per file — the default).

`rgt init` repairs only recognized RGT-owned registrations and guidance. It
preserves unrelated hooks, keybindings, plugins, and notes; ambiguous ownership
or malformed configuration is reported with a recovery action and left intact.
Every existing file is backed up before a change. A repeated run is a no-op
after the registration is current. `--force` does not replace foreign or
ambiguous plugin files. These local setup results do not establish that a client
loaded the entry or that automatic capture works.
Exit codes: `0` = success, `1` = at least one agent failed, `2` = invalid
`--agent` name.

`rgt init` writes hook commands that invoke the CLI by **absolute path** (resolved
from the running binary), so capture does not depend on the PATH of the shell
your AI tool spawns. If you relocate the binary (e.g. a package-manager upgrade
installs to a new path), re-run `rgt init` to refresh the written paths.

### Doctor

```bash
rgt doctor   # diagnose hook installation and capture health
```

`rgt doctor` reports local store, per-surface registration, and RGT-owned
guidance-file health separately. Guidance findings name missing, incomplete,
unowned, managed, and hand-maintained instruction blocks. A complete
instruction-only file is reported as a warning because it cannot provide
automatic read capture. It does not infer client loading or live
capture from a settings file, instruction file, or graph values.
Codex trust review and Windsurf Restricted Mode/active-file precedence require
checks in their clients. Exit `0` for healthy or advisory findings and `1` for
unambiguous local problems.

### Update
```bash
rgt update [--check] [--yes] [--version <tag>] [--skip-checksum] [--skip-attestation]
```

`rgt update` installs only **verified** binaries: every download is checked
against the release's `checksums.txt` (SHA-256) **and** its GitHub Artifact
Attestation — a trust anchor issued independently of the release's own assets
(built-in Sigstore verification, no `gh` required). A release missing either
verification is refused with a non-zero exit; `--skip-checksum` /
`--skip-attestation` are explicit, documented-as-insecure opt-outs that are
never the default. Version comparison uses Semantic Versioning: an older or
equal tag is never installed (no silent downgrades), and `--version <tag>`
warns when the requested tag is older than the installed version.

### Provenance
```bash
rgt record <file> [--number-format <us|eu|auto>]   # extract and track values from a file
rgt derive --parents <ids> --operation <op> --expression <expr> --result <val>
                                           # verify and record a derivation
rgt verify --parents <ids> --operation <op> --result <val>
                                           # verify a derived value without recording
rgt status [--stale-only] [--json]         # graph state and staleness
rgt query <node_id> [--json]               # full lineage for a value
rgt graph [-f text|mermaid|dot|ttl] [--include-absolute-paths]
                                           # export the dependency graph
rgt gc [--vacuum]                          # remove obsolete (stale, dependent-free) nodes
```

`rgt record --number-format` overrides the persisted format for one call; otherwise the persisted setting (from `rgt init`) is used, falling back to `auto`. Number parsing is locale-aware: a leading `-` and accounting parentheses `(N)` are part of the value, thousands/decimal separators are honored per the active format, and values that can't be parsed under that format are skipped (with a warning) rather than split into bogus nodes.

#### Turtle provenance export

`rgt graph --format ttl` writes deterministic Turtle using PROV-O entities,
activities, usages, and coding-tool agents. Project-scoped value IRIs keep
matching local node IDs from independent stores distinct when exports are
combined as an RDF union; copying a store preserves its project identity. The
original node ID remains available as `rgt:localNodeId`.

By default, source documents have path-free IRIs. A safely verified project
relative path is included when the file is available; outside, missing, or
unclassifiable paths have no path literal. The opt-in
`--include-absolute-paths` flag adds available absolute source paths and may
expose usernames or local directory names. Preserved free-text expressions can
also contain path-like text.

Turtle export is limited to 10,000 recorded values, 30,000 derivation edges,
64 MiB of rendered UTF-8, and a one-second command deadline. A limit or
pre-write error produces no Turtle on stdout. Uncertain derivation groups keep
their direct parent links but omit the activity and consolidated operation or
expression claims, with a warning on stderr. Capture activities record which
of the nine event-capable hook/plugin agents captured a value; they do not
claim source authorship or derivation responsibility. Other graph formats
retain their existing output and have no new Turtle limits.

`rgt gc` removes **obsolete nodes** — values marked stale that nothing derives from — and reports how many it removed. It never touches non-stale nodes or anything still depended on. `rgt gc --vacuum` additionally reclaims freed disk pages with SQLite `VACUUM` (opt-in; the default run only deletes rows).

Operations: `EXPRESSION` (formulas like `a + b * c`), `DATE_DIFF`, `DURATION_SUM`, and `DURATION_AVG`. Parent variables map as `parent[0]=a, parent[1]=b, ...` (at most 26, `a`–`z`).

`--operation` is case-sensitive; unsupported operations are rejected with exit code 2 — verification is never silently skipped. `--expression` is limited to 4096 bytes and 256 levels of parenthesis nesting (over-limit input is rejected with exit 2, never a crash). Expressions evaluate with built-in functions disabled — only your parent variables and `+ - * / % ^` and parentheses are available, so evaluation is deterministic. Legacy `EXPRESSION` still returns a Number when given Date or Duration parents and warns on stderr: Dates are Unix timestamp seconds, Durations are elapsed seconds.

### Duration calculations and display

`DATE_DIFF` automatically computes the elapsed Duration between two ordered Date parents (`date2 - date1`). Negative or fractional-second gaps are rejected. `DURATION_SUM` and `DURATION_AVG` accept 2–26 distinct Duration parents; averages must resolve to a whole second. All three operations store exact signed whole seconds in a typed Duration node. Claims are optional for `derive` and use seconds by default; `--result-unit` qualifies a supplied claim without changing storage:

```bash
rgt derive --parents <date1>,<date2> --operation DATE_DIFF --result 45 --result-unit days --unit hours
rgt derive --parents <duration1>,<duration2> --operation DURATION_SUM --unit minutes
rgt verify --parents <date1>,<date2> --operation DATE_DIFF --result 45 --result-unit days
rgt query <duration_id> --unit days [--json]
```

The fixed elapsed-time units are `seconds`, `minutes`, `hours`, `days`, and `weeks`. Temporal values and claims must resolve to exact whole seconds; averages or claims that require fractional seconds are rejected without rounding. Calendar months and years are unsupported because their lengths vary. `--result-unit` only qualifies a claim, and `--unit` only selects a display. Neither unit is persisted as node metadata.

Query keeps its existing `value`, adds exact `duration_seconds` as a decimal string, and includes `selected_unit_display` only on the requested node. Repeating conversions are marked approximate; exact seconds remain available. Querying a Date or Number with `--unit` is rejected. Legacy `EXPRESSION` calculations with Date or Duration parents still return a Number and emit a warning: Dates are interpreted as Unix timestamp seconds and Durations as elapsed seconds. Use `DATE_DIFF`, `DURATION_SUM`, or `DURATION_AVG` when a typed Duration result is intended.

## Supported AI Tools

RGT writes a native registration, plugin, or instructions for 13 AI coding tools. Capture is verified per read path, not inferred from an installed artifact. See the [harness compatibility support matrix](docs/compatibility/support-matrix.md).

| Tool | Install | Method |
|------|---------|--------|
| **Claude Code** | `rgt init -g` | PreToolUse/PostToolUse hook (`settings.json`); full-content read paths are unverified |
| **Cursor** | `rgt init -g --agent cursor` | pre/postToolUse hook (`hooks.json`); automatic read capture unverified |
| **GitHub Copilot (VS Code)** | `rgt init -g --agent copilot` | Copilot Chat hook registration; automatic read capture unverified |
| **GitHub Copilot CLI** | `rgt init -g --agent copilot` | Instructions file (Copilot CLI config dir) |
| **Gemini CLI** | `rgt init -g --agent gemini` | `~/.gemini/hooks.toml` PostToolUse |
| **Mistral Vibe** | `rgt init -g --agent vibe` | `pre_tool` hook (`hooks.toml`) + loaded `AGENTS.md` guidance |
| **OpenCode** | `rgt init -g --agent opencode` | TypeScript plugin |
| **Pi** | `rgt init --agent pi` (or `-g`) | TypeScript extension + loaded project or user guidance |
| **Hermes** | `rgt init --agent hermes` | Python plugin + `plugins.enabled` + project context guidance |
| **Codex CLI** | `rgt init --agent codex` | Native `PostToolUse` hook plus `AGENTS.md`; trust review required and read capture unverified |
| **Windsurf** | `rgt init --agent windsurf` | Native `post_read_code` hook plus `.windsurfrules`; event supplies a path only, so value capture is instruction-only |
| **Cline / Roo Code** | `rgt init --agent cline` | `.clinerules` |
| **Google Antigravity** | `rgt init --agent antigravity` | `.agents/rules/antigravity-rgt-rules.md` |
| **Kilo Code** | `rgt init --agent kilocode` | `.kilocode/rules/rgt-rules.md` |

Accepted `--agent` values: `claude-code`, `cursor`, `codex`, `windsurf`, `copilot`, `gemini`, `vibe`, `opencode`, `pi`, `hermes`, `cline`, `antigravity`, `kilocode`, plus aliases `claude`, `roo-code`, `kilo`.

## Data & Storage

RGT stores its graph in `.rgt/store.db` (SQLite) in the project root, created by `rgt init`. No external services, no telemetry, no network calls during normal operation.

## Documentation

- **[AGENTS.md](AGENTS.md)**: instructions RGT installs for AI agents (how to record and derive)
- **[CONTRIBUTING.md](CONTRIBUTING.md)**: contribution guide
- **[CHANGELOG.md](CHANGELOG.md)**: release history

## Acknowledgments

RGT was inspired by [RTK (Rust Token Killer)](https://github.com/rtk-ai/rtk), a high-performance CLI proxy that compresses shell output for AI coding agents. RGT follows RTK's approach of a single binary with native, agent-specific hook integrations, and mirrors its 13-agent coverage. Where RTK filters command output, RGT tracks the numeric and date provenance of what agents read and derive, and verifies each derivation is mathematically correct.

RGT was built with assistance from [DeepSeek](https://www.deepseek.com/) AI coding tools.

## Contributing

Contributions welcome! Please open an issue or PR on [GitHub](https://github.com/rafael-bianchi/rgt).

## License

Licensed under the [Apache License, Version 2.0](LICENSE).
