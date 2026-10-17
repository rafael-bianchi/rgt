# Quickstart: Fix Agent Hook Formats

**Feature**: `010-fix-agent-hook-formats` | **Date**: 2026-08-02

## Prerequisites

- Rust toolchain, repository cloned, on the feature branch

## Validation Scenarios

### Scenario 1: Claude Code Hook — Correct File + Schema

**Steps**:
```bash
# Test in a temp directory with isolated HOME
TMP=$(mktemp -d)
HOME=$TMP cargo run -- init -g --agent claude-code
```

**Expected**:
- `.claude/settings.json` exists (NOT `.claude/hooks.json`)
- Content matches: `{"hooks":{"PostToolUse":[{"hooks":[{"command":"rgt hook post","type":"command"}],"matcher":""}],"PreToolUse":[{"hooks":[{"command":"rgt hook pre","type":"command"}],"matcher":""}]}}`

### Scenario 2: Claude Code Settings Merge

**Steps**:
```bash
TMP=$(mktemp -d)
echo '{"permissions":{"allow":["Bash(npm *)"]}}' > $TMP/.claude/settings.json
HOME=$TMP cargo run -- init -g --agent claude-code
cat $TMP/.claude/settings.json | python3 -m json.tool
```

**Expected**: Output contains both `"permissions"` and `"hooks"` keys. The `"permissions"` key with its `"allow": ["Bash(npm *)"]` value is preserved.

### Scenario 3: Cursor Hook — Correct Event Names

**Steps**:
```bash
TMP=$(mktemp -d)
HOME=$TMP cargo run -- init -g --agent cursor
cat $TMP/.cursor/hooks.json | python3 -m json.tool
```

**Expected**:
- `"version": 1`
- `"hooks"."preToolUse"` and `"hooks"."postToolUse"` (camelCase, NOT PascalCase)
- Each entry has `"command"` and `"matcher": "Shell"`

### Scenario 4: Windsurf Rules File

**Steps**:
```bash
TMP=$(mktemp -d)
cd $TMP
cargo run -- init -g --agent windsurf
cat .windsurfrules
```

**Expected**: File contains text instructions about using `rgt` prefix. No `.codeium/windsurf/hooks.json` created.

### Scenario 5: Codex CLI Instructions

**Steps**:
```bash
TMP=$(mktemp -d)
cd $TMP
cargo run -- init -g --agent codex
cat AGENTS.md
```

**Expected**: `AGENTS.md` exists with an RGT Integration section. No `.codex/hooks.json` with `rgt_hook` key created.

### Scenario 6: Integration Tests

**Steps**:
```bash
cargo test test_hooks_installer -- --test-threads=1
```

**Expected**: All 4 agent tests pass with assertions on the corrected formats.

### Scenario 7: All Agents Mode

**Steps**:
```bash
TMP=$(mktemp -d)
HOME=$TMP cd $TMP
cargo run -- init -g
```

**Expected**: All 4 agents configured. Output shows tier classification per agent. No errors.
