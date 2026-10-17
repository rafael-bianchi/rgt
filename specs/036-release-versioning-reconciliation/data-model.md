# Data Model: Release Versioning Reconciliation

**Feature**: [spec.md](./spec.md) | **Date**: 2026-08-31

No persisted application data. The "entities" are the version-bearing state files and the checksums they must agree with.

## 1. VersionState

The set of files/tags whose version must agree:

| Source | Field to read | Target value |
|--------|---------------|--------------|
| `Cargo.toml` | `version` | `0.5.0` |
| `.release-please-manifest.json` | `.` key | `0.5.0` (no `rgt` key) |
| `Formula/rgt.rb` | `version "..."` | `0.5.0` |
| `CHANGELOG.md` | latest `## [X.Y.Z]` heading | `0.5.0` |
| git tag | `v0.5.0` | exists on origin |
| GitHub release | v0.5.0 assets + `checksums.txt` | live, authoritative |

## 2. FormulaChecksum

Per supported arch, the sha256 of the release tarball:

- aarch64-apple-darwin → `b177700e...bfed5`
- x86_64-unknown-linux-musl → `9b4bcdf5...b6c`
- aarch64-unknown-linux-gnu → `08d9e553...2e1`

(Intel macOS is deliberately absent — spec 035's decision.)
