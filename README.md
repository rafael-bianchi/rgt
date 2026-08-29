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

RGT gives AI coding agents a persistent, queryable memory of every number and date they read from source files or derive through calculations, then **verifies each derivation is mathematically correct**. Single Rust binary, 13 supported AI coding tools, hooks that record data only and never rewrite commands.

## What RGT Does

Agents reason from changing files and can make arithmetic mistakes. RGT tracks the provenance of every numeric value and re-checks the math.

| Operation | What RGT does |
|-----------|---------------|
| `rgt record <file>` | Extracts every number and date from a file into the provenance graph |
| `rgt status` | Reports total, active, and stale nodes, showing which values are still trustworthy |
| `rgt derive` | Verifies an agent-computed value against its parents **before** recording it |
| `rgt query <id>` | Traces a value's lineage back to its source files |
| `rgt graph` | Exports the dependency DAG (text, Mermaid, or DOT) |
| `rgt hook` | Passive capture via each agent's native hook/plugin mechanism |

## Why Provenance Tracking Matters

RGT does not measure savings: it prevents silent errors. Two failure modes motivate it:

1. **Stale data**: an agent reads a file, later the file changes, and the agent keeps reasoning from the old numbers. RGT marks the affected root nodes **stale** and cascades staleness through every derived value that depends on them (`rgt status`).
2. **Wrong math**: an agent computes `revenue = price * quantity` and gets it wrong. RGT re-computes the expression from the parent values in the database and **rejects** mismatched derivations (exit 1) before they enter the graph.

Hooks are **provenance-capture only**: they record `(path, content)` and never rewrite, filter, or block the agent's tool calls.

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
rgt init --agent cline      # project-scoped agents use project rules files
rgt init --agent codex      # Codex / Windsurf / Cline / Antigravity / Kilo use rules files

# 2. Restart your AI tool, then:
rgt record budget.csv       # agent reads a data file and records its values
rgt status                  # inspect the graph
rgt derive --parents node_raw_X,node_raw_Y --operation EXPRESSION --expression "a - b" --result 60000
                            # agent records a verified derivation
rgt query node_drv_Z        # trace the derived value's lineage
rgt graph --format mermaid  # export the dependency graph
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

1. **Capture**: native hooks/plugins push every file an agent reads through `rgt record`, so values land in the graph without the agent remembering to call it.

> **Non-text sources (PDF, Excel, images)**: RGT's passive hook cannot see inside binary files — PDF/image interpretation happens inside the model itself, never as a local text file. The RGT instructions RGT writes to agents tell them to explicitly report such values via `echo "Amount: 1234.56" | rgt record --stdin` (or an intermediate `.txt`/`.md` file). Plain-text/CSV/JSON/Markdown need no such action — they're captured automatically. `rgt record` on a binary file returns a clear "unsupported file format" error naming this path.
2. **Verification**: every `rgt derive` is trust-but-verify: RGT re-computes the result from parent values and rejects wrong ones before insertion.
3. **Staleness**: when a source file changes, its nodes (and everything derived from them) are flagged stale, so the agent can be told to re-read. Change detection is two-tier (mtime+size, then BLAKE3) with a time-bounded forced re-hash, and distinguishes a genuinely deleted file (`FILE_DELETED`) from a permission/lock error (`CHECK_FAILED`, which does not mark values stale).

## Commands

### Initialize & Hooks
```bash
rgt init [-g] [--force] [--agent <name>]   # configure hooks, detect installed agents
rgt hook pre|post [--agent <name>]         # passive capture from agent event JSON (stdin)
```

`rgt init` **never destroys your existing configuration**. It merges RGT's hook
entries into your agent config additively — JSON/JSONC settings (comments and
trailing commas are tolerated and preserved), TOML configs, and rules files keep
every other hook, keybinding, plugin, and note byte-for-byte. Re-running without
`--force` is a no-op; `--force` refreshes only RGT's own delimited block. If a
config file can't be parsed or safely merged, that agent is reported with a
recoverable error and a non-zero exit — other agents are still configured, and a
`<file>.rgt.bak` backup is kept before any existing file is rewritten.
Exit codes: `0` = success, `1` = at least one agent failed, `2` = invalid
`--agent` name.

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
rgt record <file>                          # extract and track values from a file
rgt derive --parents <ids> --operation <op> --expression <expr> --result <val>
                                           # verify and record a derivation
rgt verify --parents <ids> --operation <op> --result <val>
                                           # verify a derived value without recording
rgt status [--stale-only] [--json]         # graph state and staleness
rgt query <node_id> [--json]               # full lineage for a value
rgt graph [-f text|mermaid|dot]            # export the dependency DAG
```

Operations: `EXPRESSION` (formulas like `a + b * c`) and `DATE_DIFF` (date arithmetic, e.g. `date2 - date1`). Parent variables map as `parent[0]=a, parent[1]=b, ...` (at most 26, `a`–`z`).

`--operation` is case-sensitive (`EXPRESSION`/`DATE_DIFF`); anything else is rejected with exit code 2 — verification is never silently skipped. `--expression` is limited to 4096 bytes and 256 levels of parenthesis nesting (over-limit input is rejected with exit 2, never a crash). Expressions evaluate with built-in functions disabled — only your parent variables and `+ - * / % ^` and parentheses are available, so evaluation is deterministic. `DATE_DIFF` requires `--parents <date1>,<date2>` and computes `date2 - date1`; a negative result (possible swapped order) is rejected.

## Supported AI Tools

RGT configures provenance-capture hooks for 13 AI coding tools, using each agent's native mechanism:

| Tool | Install | Method |
|------|---------|--------|
| **Claude Code** | `rgt init -g` | PreToolUse/PostToolUse shell hook (`settings.json`); Bash tool reads (`cat`/`head`/`tail`/`grep`/`rtk read`, ...) are also captured |
| **Cursor** | `rgt init -g --agent cursor` | pre/postToolUse hook (`hooks.json`); Bash tool reads captured too |
| **GitHub Copilot (VS Code)** | `rgt init -g --agent copilot` | Copilot Chat hooks (`github.copilot.chat.hooks`) |
| **GitHub Copilot CLI** | `rgt init -g --agent copilot` | Instructions file (Copilot CLI config dir) |
| **Gemini CLI** | `rgt init -g --agent gemini` | `~/.gemini/hooks.toml` PostToolUse |
| **Mistral Vibe** | `rgt init -g --agent vibe` | `pre_tool` hook (`hooks.toml`) + prompt |
| **OpenCode** | `rgt init -g --agent opencode` | TypeScript plugin |
| **Pi** | `rgt init --agent pi` (or `-g`) | TypeScript extension |
| **Hermes** | `rgt init --agent hermes` | Python plugin + `plugins.enabled` |
| **Codex CLI** | `rgt init --agent codex` | `AGENTS.md` instructions |
| **Windsurf** | `rgt init --agent windsurf` | `.windsurfrules` |
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
