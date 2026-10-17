# Quickstart: Translated README Resync

**Feature**: [spec.md](./spec.md) | **Date**: 2026-08-31

Runnable end-to-end validation. See [contracts/readme-parity-contract.md](./contracts/readme-parity-contract.md) for the authoritative invariants.

## Prerequisites

- A shell (POSIX `sh`).

## Scenario 1 — parity check passes

```bash
scripts/check-readme-parity.sh
```

Expected: exit 0 — every translated README has the Doctor/Update/`gc`/`--number-format`/non-text-source content, the language bar links all 7 variants, and markdown fences/tables are well-formed.

## Scenario 2 — spot-check a translation

```bash
grep -n "Doctor\|doctor" README_es.md    # Spanish Doctor section present
grep -n "gc\|number-format" README_fr.md # French Commands updated
```

Expected: hits in each file.

## Scenario 3 — no code regressions

```bash
cargo test   # must stay green (docs-only change)
```

## Automated checks

```bash
scripts/check-readme-parity.sh
git diff --stat -- src/   # empty
```
