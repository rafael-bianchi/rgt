# Quickstart: Documentation Reconciliation

**Feature**: `012-documentation-reconciliation` | **Date**: 2026-08-08

## Prerequisites

- Rust toolchain, repository on feature branch

## Validation Scenarios

### Scenario 1: All Public Functions Documented

```bash
grep -rn "^pub fn" src/ --include="*.rs" | while read line; do
  file=$(echo "$line" | cut -d: -f1); lineno=$(echo "$line" | cut -d: -f2)
  prev=$(($lineno - 1))
  if ! sed -n "${prev}p" "$file" | grep -q "///"; then echo "MISSING: $line"; fi
done
```

**Expected**: No `MISSING:` lines.

### Scenario 2: Cargo Doc Builds Clean

```bash
cargo doc --no-deps 2>&1
```

**Expected**: No warnings (especially `broken_intra_doc_links`).

### Scenario 3: README Covers All 8 CLI Commands

```bash
for cmd in init status query graph hook mcp update verify; do
  echo -n "$cmd: "; grep -ci "$cmd" README.md
done
```

**Expected**: Each command has ≥1 mention in README.

### Scenario 4: No Outdated Install Paths

```bash
grep -rn "rafael-bianchi/tap\|cargo install rgt " README.md AGENTS.md CONTRIBUTING.md || echo "CLEAN"
```

**Expected**: `CLEAN` (zero matches).

### Scenario 5: No Untracked Tooling Dirs

```bash
git status --short
```

**Expected**: No `.kilocode/` or `.kilo/` untracked.

### Scenario 6: Full Test Suite

```bash
cargo test --all -- --test-threads=1
```

**Expected**: All tests pass (98).
