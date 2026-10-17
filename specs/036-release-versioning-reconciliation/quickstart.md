# Quickstart: Release Versioning Reconciliation

**Feature**: [spec.md](./spec.md) | **Date**: 2026-08-31

Runnable end-to-end validation. See [contracts/version-state-contract.md](./contracts/version-state-contract.md) for the authoritative invariants.

## Prerequisites

- A shell (POSIX `sh`).

## Scenario 1 — version state is consistent

```bash
scripts/check-version-state.sh
```

Expected: exit 0 — `Cargo.toml`, manifest `.` key, formula version, and latest changelog heading all `0.5.0`; no `rgt` manifest key.

## Scenario 2 — formula SHAs match the live release

```bash
RGT_CHECKSUMS_URL=https://github.com/rafael-bianchi/rgt/releases/download/v0.5.0/checksums.txt scripts/check-version-state.sh
```

Expected: exit 0 — all three formula SHAs match `checksums.txt` (network required; without it the script warns and still passes).

## Scenario 3 — no production code touched

```bash
git diff --stat -- src/
```

Expected: empty.

## Automated checks

```bash
scripts/check-version-state.sh
grep -E "doctor|gc|number-format" CHANGELOG.md   # 0.5.0 entry mentions them
git diff --stat -- src/
```
