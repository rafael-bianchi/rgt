# Quickstart & Validation Guide: Distribution & Installation Channels

**Feature Branch**: `002-distribution-installation` | **Date**: 2026-07-31

## Overview

This guide provides runnable validation scenarios to verify shell installation, Homebrew formula installation, `cargo install`, and binary self-updates for RGT.

---

## Scenario 1: Validate POSIX Shell Installer Script (`install.sh`)

### Objective
Verify that `install.sh` correctly detects OS and architecture, downloads the release binary, verifies SHA-256 checksum, and installs `rgt` to `~/.local/bin`.

### Execution
```bash
# Run local dry-run or live installer test:
./install.sh --bin-dir ./test-bin

# Verify installed binary execution
./test-bin/rgt --version
```

### Expected Outcome
- Installer detects current OS (`darwin`/`linux`) and CPU architecture (`x86_64`/`aarch64`).
- Binary is extracted into `./test-bin/rgt`.
- Executing `./test-bin/rgt --version` prints `rgt 0.1.0`.

---

## Scenario 2: Validate Self-Update Check (`rgt update --check`)

### Objective
Verify that `rgt update --check` queries GitHub Releases API and reports update status.

### Execution
```bash
rgt update --check
```

### Expected Outcome
- Queries GitHub API for `rafael-bianchi/rgt`.
- Prints current version vs latest tag and exits cleanly.

---

## Scenario 3: Validate Homebrew Formula (`rgt.rb`)

### Objective
Verify that `Formula/rgt.rb` passes Homebrew formula audit and test suite.

### Execution
```bash
brew audit --strict Formula/rgt.rb
brew install --build-from-source Formula/rgt.rb
brew test Formula/rgt.rb
```

### Expected Outcome
- Homebrew audit passes with zero errors.
- `brew test` verifies `rgt --version` execution.
