# Quickstart: Expand Agent Hook Coverage to Match RTK

**Phase 1 output** | **Date**: 2026-08-15 | **Spec**: [spec.md](./spec.md)

Validation guide for the expanded agent-hook coverage. Implementation details belong in `tasks.md`.

## Prerequisites

- Local build: `cargo build --release`
- Working `rgt` on PATH (or `alias rgt=$PWD/target/release/rgt`)
- For plugin agents, the agent's runtime: Node (OpenCode, Pi), Python 3 (Hermes)
- CI: `cargo test`, `cargo fmt --all -- --check`, `cargo clippy --all-targets`

## Scenario 1 — All 13 `--agent` names accepted (SC-001)

```bash
for agent in claude-code cursor codex windsurf copilot gemini vibe opencode pi hermes cline antigravity kilocode; do
  rgt init --agent "$agent" --force && echo "OK: $agent"
done
```

Expected: each prints `✓ Auto-configured AI coding agent hooks:` and `OK: <agent>`. Existing 4 configs byte-identical to pre-change output (SC-003) — diff before/after for `~/.claude/settings.json`, `.cursor/hooks.json`, `AGENTS.md`, `.windsurfrules`.

## Scenario 2 — Unknown agent rejected (FR-001)

```bash
rgt init --agent "nonsense-agent"
```

Expected: exit code 2, message listing the 16 valid `--agent` values (13 canonical + aliases `claude`, `roo-code`, `kilo`).

## Scenario 3 — Auto-detect all installed agents (SC-002)

Create/emulate two agent config dirs (e.g. `~/.vibe/hooks.toml` and `.kilocode/rules/`), then:

```bash
rgt init
```

Expected: both detected and listed in output; single run configures every installed supported agent. Re-run: idempotent, no duplicate sections (FR-006).

## Scenario 4 — Fail-open hooks (SC-004)

For each event-capture agent, pipe malformed JSON and confirm no-op exit 0:

```bash
echo 'not json' | rgt hook post --agent copilot; echo "exit=$?"   # → exit=0, no stdout
echo '{"tool_input":{"command":"cat /nonexistent.json"}}' | rgt hook post --agent gemini; echo "exit=$?"  # → exit=0
echo '{}' | rgt hook post --agent vibe; echo "exit=$?"
```

Expected: every `exit=0` with no stdout; tool call never blocked or altered.

## Scenario 5 — Per-agent event capture

Verify each dialect records values from a real file (contracts: `contracts/agent-hooks.md`):

```bash
printf '{"count": 42, "date": "2026-08-15"}' > /tmp/sample.json
echo '{"tool_name":"Read","tool_input":{"path":"/tmp/sample.json"},"tool_response":{"content":"{\"count\": 42}"}}' \
  | rgt hook post
rgt status   # → shows root node(s) with the extracted value
```

Then repeat with a command-style event (`--agent gemini` / `--agent vibe` / `--agent copilot`) reading the same file; expect the disk-read fallback to record the same values.

## Scenario 6 — Plugin artifacts (OpenCode / Pi / Hermes)

```bash
rgt init --agent opencode --force   # writes rgt.ts; load in opencode
rgt init --agent pi --force         # writes pi extension; load via `pi -e <file>` (or open Pi)
rgt init --agent hermes --force     # writes hermes plugin + enables it in hermes config
```

Expected: plugin loads, invokes `rgt hook post --agent <name>` on tool calls, exits 0 on any failure, and never alters the command (fail-open, verified in each agent's runtime).

## Scenario 7 — Rules-file agents

```bash
rgt init --agent cline --force && cat .clinerules   # RGT section present
rgt init --agent antigravity --force && cat .agents/rules/antigravity-rgt-rules.md
rgt init --agent kilocode --force && cat .kilocode/rules/rgt-rules.md
```

Expected: each file contains the `## RGT Integration` instructions (record/derive/status); re-running appends nothing (idempotent).

## Scenario 8 — Documentation parity (SC-005)

```bash
rgt init --help   # agent list in help text
grep -rl "copilot\|gemini\|hermes" README.md AGENTS.md
```

Expected: documented supported-agent list matches the 16 accepted `--agent` values exactly (13 canonical names + aliases `claude`, `roo-code`, `kilo`).

## Exit-code contract (all scenarios)

- `0` = success / fail-open no-op
- `1` = expected error (e.g., DB open failure)
- `2` = invalid input (unknown `--agent` name)
