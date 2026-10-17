# Quickstart & Validation Guide: CI/CD Pipeline Hardening

**Feature Branch**: `003-devops-enhancement` | **Date**: 2026-07-31

## Overview

This guide provides runnable validation scenarios to verify the hardened CI/CD pipeline: security scanning, conventional-commit releases, reusable workflows, MUSL binary, DEB/RPM packages, CI optimization, and branch protection.

---

## Scenario 1: Verify CI Fast-Failure Gates

### Objective
Confirm that formatting failures skip linting and tests.

### Execution
1. Create a branch with a deliberate `cargo fmt` violation (e.g., extra whitespace in `src/main.rs`)
2. Open a PR against `develop`
3. Observe CI workflow run

### Expected Outcome
- `fmt` job fails within 30 seconds
- `clippy` and `test` jobs are skipped (greyed out, not executed)

---

## Scenario 2: Verify Security Scan

### Objective
Confirm that adding a known-vulnerable dependency fails CI.

### Execution
1. Add a dependency with a published CVE to `Cargo.toml` (e.g., an old version of a crate with a known advisory)
2. Run `cargo update` to lock the version
3. Open a PR and observe the `security` job

### Expected Outcome
- `security` job fails with `cargo-audit` output identifying the vulnerable crate and CVE ID
- PR merge is blocked

---

## Scenario 3: Verify Reusable Release Workflow

### Objective
Confirm that manual and automated releases use the same build pipeline.

### Execution
1. Trigger a manual release via `workflow_dispatch` on `release.yml` with `tag: v0.1.0-rc.1` and `prerelease: true`
2. Observe the build output — verify all 5 platform binaries + DEB + RPM are produced
3. Compare the build job logs with an automated release triggered by `cd.yml`

### Expected Outcome
- Both releases produce identical artifacts (same target triples, same package formats, same `checksums.txt` format)
- Pre-release is marked as "pre-release" on GitHub; stable release is not

---

## Scenario 4: Verify MUSL Static Binary

### Objective
Confirm the Linux MUSL binary runs on both glibc and musl-based distributions.

### Execution
```bash
# Test on Ubuntu (glibc)
docker run --rm -v $(pwd):/test ubuntu:latest /test/rgt-x86_64-unknown-linux-musl --version

# Test on Alpine (musl)
docker run --rm -v $(pwd):/test alpine:latest /test/rgt-x86_64-unknown-linux-musl --version
```

### Expected Outcome
- Both containers print the version without "library not found" errors

---

## Scenario 5: Verify DEB and RPM Packages

### Objective
Confirm native packages install correctly.

### Execution
```bash
# DEB on Ubuntu
docker run --rm -v $(pwd)/*.deb:/tmp/rgt.deb ubuntu:latest sh -c \
  "dpkg -i /tmp/rgt.deb && rgt --version"

# RPM on Fedora
docker run --rm -v $(pwd)/*.rpm:/tmp/rgt.rpm fedora:latest sh -c \
  "rpm -i /tmp/rgt.rpm && rgt --version"
```

### Expected Outcome
- Both install the binary to a system PATH directory and execute successfully

---

## Scenario 6: Verify PR Target Check

### Objective
Confirm PRs targeting `main` from non-`develop` branches are flagged.

### Execution
1. Create a branch from `main` (e.g., `test-direct-to-main`)
2. Open a PR targeting `main`
3. Observe `pr-target-check` workflow

### Expected Outcome
- PR is automatically labeled `wrong-base`
- A comment is posted explaining the `develop` branch policy
- The check does not close the PR (soft enforcement)
