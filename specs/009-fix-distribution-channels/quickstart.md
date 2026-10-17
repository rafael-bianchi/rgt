# Quickstart: Fix Distribution Channels

**Feature**: `009-fix-distribution-channels` | **Date**: 2026-08-01

## Prerequisites

- Rust toolchain installed
- Repository cloned and on the feature branch
- GitHub CLI (`gh`) authenticated with repo access
- Access to GitHub Actions tab for `rafael-bianchi/rgt`

## Validation Scenarios

### Scenario 1: Fix CI Clippy Errors

**Goal**: Verify `cargo clippy --all-targets` passes with zero errors.

**Steps**:
```bash
cargo clippy --all-targets
```

**Expected**: No `clippy::erasing_op` or `clippy::identity_op` errors. Only pre-existing warnings (if any) remain.

### Scenario 2: Fix Dead Code in Updater Module

**Goal**: Verify `src/updater/github.rs` has no dead-code clippy warnings.

**Steps**:
```bash
cargo clippy --all-targets -- -W dead_code 2>&1 | grep updater
```

**Expected**: No dead code warnings from `src/updater/github.rs`.

### Scenario 3: Verify Install Commands in Documentation

**Goal**: Verify README, AGENTS.md, and install.sh use correct commands.

**Steps**:
```bash
# Homebrew tap references — should be zero matches
grep -n "rafael-bianchi/tap" README.md AGENTS.md install.sh && echo "FAIL" || echo "PASS"

# Bare cargo install rgt (without --git) — should be zero matches
grep -n "cargo install rgt" README.md AGENTS.md install.sh | grep -v "\-\-git" && echo "FAIL" || echo "PASS"

# homebrew-tap references — should be zero matches
grep -n "homebrew-tap" README.md AGENTS.md && echo "FAIL" || echo "PASS"
```

**Expected**: All three checks return PASS (zero matches).

### Scenario 4: Verify Homebrew Job in release.yml

**Goal**: Verify the `homebrew` job updates `Formula/rgt.rb` in-repo (not via dispatch).

**Steps**:
```bash
grep -A5 "homebrew:" .github/workflows/release.yml
```

**Expected**: No `peter-evans/repository-dispatch` or `rafael-bianchi/homebrew-tap` reference. The job should use `actions/checkout`, compute SHA256s, and commit.

### Scenario 5: Full Test Suite

**Goal**: Verify all tests pass after changes.

**Steps**:
```bash
cargo fmt --all -- --check
cargo clippy --all-targets
cargo test --all -- --test-threads=1
```

**Expected**: All three pass with zero errors.

### Scenario 6: CD Workflow Validation (Manual)

**Goal**: Verify CD workflow runs after clearing the stuck queue.

**Steps**:
1. Go to GitHub Actions → CD workflow
2. Cancel the pending/queued runs on `main` (IDs: 30719536844, 30717436776)
3. Trigger a new `workflow_dispatch` on `main` or wait for the next release-please PR merge
4. Observe `build-release` job creates and uploads artifacts
5. Observe `homebrew` job commits updated SHA256s to `Formula/rgt.rb`

**Expected**: Release artifacts uploaded for all 5 platforms. `Formula/rgt.rb` has real SHA256 values.
