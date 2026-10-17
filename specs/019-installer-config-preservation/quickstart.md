# Quickstart: Validating Installer Config Preservation

**Feature**: [spec.md](./spec.md) | **Date**: 2026-08-27

A runnable guide to prove the feature works end-to-end. Invariants are defined in [contracts/init-behavior.md](./contracts/init-behavior.md) and [contracts/config-file-invariants.md](./contracts/config-file-invariants.md); domain entities in [data-model.md](./data-model.md).

## Prerequisites

- Rust stable toolchain.
- Build in debug: `cargo build`.
- A throwaway scratch dir for manual scenarios (e.g. `mktemp -d`).

> **Manual-scenario caution**: `rgt init` resolves paths via the platform home API, NOT the `$HOME` env var (the `dirs` crate ignores `$HOME` on macOS/Windows). Project-scoped agents (claude-code, cursor, codex, windsurf, cline, antigravity, kilocode, opencode, pi) write into the **current working directory**; home-scoped agents (copilot, gemini, vibe, hermes) write into the real home. For isolated manual runs, `cd` into the scratch dir and only use project-scoped agents; for full isolation use the automated integration tests (`detect_and_configure_hooks_in_home`/`_with_config`).

## Automated validation

```bash
cargo fmt --all -- --check
cargo clippy --all-targets
cargo test
```

Expected:
- `test_hook_editors` (new) passes: byte-for-byte preservation, JSONC tolerance, malformed-input errors, paired-marker `--force` replacement.
- `test_hook_installer_contract` (extended) passes: per-writer merge/preserve/error invariants from `config-file-invariants.md`.
- `test_hooks_installer` (extended) passes: end-to-end `rgt init` against temp homes, including exit codes from `init-behavior.md`.
- No regressions in the rest of the suite; 018-era byte-identical installer assertions were updated to the new merge expectations.

## Manual scenario 1 — Claude Code settings preserved (US1/US3)

```bash
SCRATCH=$(mktemp -d)
cd "$SCRATCH"
mkdir -p .claude
# Seed a config with a foreign hook group + comments + trailing comma (JSONC)
cat > .claude/settings.json <<'EOF'
{
  // RTK's own hook group — must survive
  "hooks": {
    "PostToolUse": [
      { "matcher": "Bash", "hooks": [{ "type": "command", "command": "rtk hook post" }] },
    ],
  },
  "keybindings": { "esc": "stop" },
}
EOF
# claude-code is project-scoped, so it writes .claude/settings.json in the cwd
/path/to/target/debug/rgt init --agent claude-code
```

Expected:
- RGT's `PostToolUse`/`PreToolUse` groups added; RTK group, `keybindings`, the comment, and the trailing comma all intact (diff shows only RGT lines added).
- A `.claude/settings.json.rgt.bak` backup exists (the file already existed).
- Re-running without `--force` changes nothing (idempotent).

## Manual scenario 2 — malformed config fails loudly (US2)

```bash
# still inside $SCRATCH from scenario 1
cat > .claude/settings.json <<'EOF'
{ "hooks": {
EOF
/path/to/target/debug/rgt init --agent claude-code; echo "exit=$?"
```

Expected: error naming `.claude/settings.json` and the parse problem; file byte-identical; `exit=1`.

## Manual scenario 3 — Hermes TOML stays valid (US3)

> Hermes is home-scoped (the `dirs` platform API resolves the real home, ignoring `$HOME`). Use the automated `test_hermes_toml_stays_valid_after_reinit` integration test for isolated validation; a manual run edits the real `~/.hermes/config.toml`.

```bash
# manual, against the real home — only if you accept touching it
mkdir -p ~/.hermes/plugins/rgt
# seed ~/.hermes/config.toml with a comment, a [plugins] table, and other plugins
/path/to/target/debug/rgt init --agent hermes
cat ~/.hermes/config.toml
```

Expected: `"rgt"` added to `enabled`, single `[plugins]` table, comment preserved, TOML still parses.

## Manual scenario 4 — `--force` preserves content below the RGT block (US3)

```bash
# still inside $SCRATCH from scenario 1
cat > .clinerules <<'EOF'
# My project rules
EOF
/path/to/target/debug/rgt init --agent cline
printf '\n## My personal notes (below RGT block)\n' >> .clinerules
/path/to/target/debug/rgt init --agent cline --force
tail -5 .clinerules
```

Expected: the RGT block is refreshed between its markers; `## My personal notes (below RGT block)` is still present at the end.

## Manual scenario 5 — multi-agent partial failure (US2/FR-008)

With a malformed Claude config and a healthy Cline config in the scratch dir:
```bash
# still inside $SCRATCH; .claude/settings.json is still the malformed file from scenario 2
printf '# rules\n' > .clinerules
/path/to/target/debug/rgt init; echo "exit=$?"
```
Expected: cline configured, claude-code reported failed with file+reason, `exit=1`.

## Definition of done (link to contracts)

- All automated tests pass (this page, above).
- Manual scenarios 1–5 produce the stated expected outcomes.
- Every invariant in `contracts/init-behavior.md` and `contracts/config-file-invariants.md` has at least one automated test.
