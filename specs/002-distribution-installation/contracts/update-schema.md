# CLI Self-Update Contract: `rgt update`

**Feature Branch**: `002-distribution-installation` | **Date**: 2026-07-31

## Subcommand Interface

```text
rgt update [OPTIONS]

Options:
      --check       Check if a newer version is available without installing
  -y, --yes         Skip interactive confirmation prompt and apply update
  -v, --version     Target specific version tag (e.g. --version v0.2.0)
  -h, --help        Print help information
```

---

## Behavior Specification

1. `rgt update --check`:
   - Fetches `https://api.github.com/repos/rafael-bianchi/rgt/releases/latest`.
   - Compares `tag_name` with `env!("CARGO_PKG_VERSION")`.
   - Prints: `A new version of rgt is available: vX.Y.Z (current: vA.B.C)` or `rgt is up to date (vA.B.C)`.
   - Exits with `0` if update available, `1` if up to date.

2. `rgt update`:
   - Checks latest release on GitHub.
   - Downloads platform asset matching running OS and CPU architecture.
   - Verifies SHA-256 hash against `checksums.txt`.
   - Atomically replaces current executable binary in-place.
