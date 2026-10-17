# Quickstart: CI Pipeline Hygiene

**Feature**: [spec.md](./spec.md) | **Date**: 2026-08-31

Runnable end-to-end validation. See [contracts/ci-hygiene-contract.md](./contracts/ci-hygiene-contract.md) for the authoritative invariants.

## Scenario 1 — test-presence check works locally

```bash
# Synthetic: a new source file with no tests
mkdir -p /tmp/diff-test/src && echo "fn main() {}" > /tmp/diff-test/src/foo.rs
scripts/check-test-presence.sh <(echo "src/foo.rs")   # exit non-zero
# Docs-only
scripts/check-test-presence.sh <(echo "README.md")     # exit zero
```

## Scenario 2 — coverage measured

```bash
cargo install cargo-llvm-cov   # once
cargo llvm-cov --all
```

Expected: a line-coverage percentage is printed.

## Scenario 3 — actions pinned

```bash
grep -E "uses: [a-zA-Z0-9_.-]+/[a-zA-Z0-9_.-]+@[0-9a-f]{40}" .github/workflows/release.yml .github/workflows/cd.yml
```

Expected: matches every third-party `uses:` line; no `@v\d` tags remain in those two files.

## Automated checks

```bash
cargo test   # no source changes, must stay green
```
