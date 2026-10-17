# Technical Research: CI/CD Pipeline Security & Delivery Hardening

**Feature Branch**: `003-devops-enhancement` | **Date**: 2026-07-31

## Overview

This document records architectural research and technical decisions for hardening RGT's CI/CD pipeline: automated security scanning, conventional-commit-driven release automation, reusable build workflows, Linux MUSL support, native packages, CI fast-failure optimization, and branch protection.

---

## 1. Dependency Vulnerability Scanning

### Decision
Use `cargo-audit` integrated into the CI pipeline as a dedicated security job.

### Rationale
- `cargo-audit` is the de-facto standard Rust tool for checking dependencies against the RustSec advisory database.
- It parses `Cargo.lock` and cross-references with known CVEs, producing machine-readable and human-readable output.
- On HIGH/CRITICAL findings, the CI job fails and blocks merge. On database unavailability, the job warns but passes (fail-open for availability).

### Alternatives Considered
- *Dependabot/Cargo dependency updates*: Complements but doesn't replace vulnerability scanning. Dependabot updates versions; cargo-audit checks for known CVEs.
- *OSV-Scanner*: Broader scope (supports multiple ecosystems) but heavier. `cargo-audit` is Rust-native and lighter weight.
- *Snyk*: Proprietary, requires external service, not suitable for permissive open-source.

---

## 2. Dangerous Code Pattern Detection

### Decision
Implement a bash-based pattern scanner in the CI security job that greps the PR diff for known-dangerous Rust patterns.

### Rationale
- Patterns to detect: `Command::new`, `std::net::TcpStream`, `UdpSocket`, `unsafe {`, `.unwrap()`, `panic!(`, `expect(`, `std::process::Command`.
- The scanner runs on the git diff against the base branch, so only changed code is flagged.
- It provides a human-readable summary with severity categorization and recommended actions.

### Alternatives Considered
- *Semgrep*: Powerful AST-aware scanning but requires additional `.semgrep.yml` rules config and container image in CI. Added complexity for a Rust CLI project where grep-based pattern matching covers the most common dangerous patterns.
- *Clippy security lints*: Used as a secondary layer (`-W clippy::unwrap_used` etc.) but doesn't catch all patterns (e.g., `Command::new("sh")` is valid code that clippy won't flag).

---

## 3. Conventional-Commit Release Automation

### Decision
Use `googleapis/release-please-action@v4` for automated versioning and `CHANGELOG.md` generation on the main branch, with a custom pre-release computation script for the development branch.

### Rationale
- `release-please` parses conventional commit history, computes semantic version bumps, generates changelogs, and creates GitHub Releases — all automated.
- Pre-releases from the development branch use a custom script that analyzes commits since the last stable tag and computes a `dev-{version}-rc.{run}` tag.
- A floating `latest` tag is maintained to always point to the most recent stable release, enabling the installer script to reference `/releases/latest`.

### Alternatives Considered
- *semantic-release*: JavaScript-based, requires Node.js runtime. Violates the project's Rust-only ethos for tooling.
- *Manual version tags*: Current approach. Error-prone, no changelog automation, no pre-release channel.
- *cargo-release*: Good for local use but doesn't integrate with GitHub Releases and doesn't support pre-release channels.

---

## 4. Reusable Release Workflow

### Decision
Refactor `release.yml` to use the `workflow_call` trigger, making it callable from both `cd.yml` (automated) and `workflow_dispatch` (manual).

### Rationale
- A single workflow definition eliminates duplication. Both automated CD and manual releases use identical build steps, packaging, and checksum generation.
- The workflow accepts `tag` and `prerelease` inputs, with conditional steps for Discord notifications and Homebrew updates gated on `prerelease == false`.

### Alternatives Considered
- *Composite actions*: More granular but harder to reason about end-to-end. A reusable workflow is the idiomatic GitHub Actions pattern for this use case.
- *Separate release-manual.yml and release-automated.yml*: Duplicates build logic. The reusable pattern is superior.

---

## 5. Linux MUSL Static Binary

### Decision
Add `x86_64-unknown-linux-musl` as a Linux release target, replacing `x86_64-unknown-linux-gnu` for the primary Linux binary.

### Rationale
- MUSL produces a fully statically-linked binary with zero runtime C library dependencies. It runs on any Linux distribution (Alpine, Ubuntu, Debian, Fedora, Arch) without glibc version conflicts.
- The `aarch64-unknown-linux-gnu` target remains for ARM64 Linux (MUSL cross-compilation for aarch64 is more complex and less tested).
- Installing `musl-tools` on the CI runner is a one-line `apt-get install` step.

### Alternatives Considered
- *Keep glibc only*: Current approach. Fails on Alpine and musl-based containers. A significant fraction of Docker/CI users run Alpine.
- *Build both glibc and musl*: Doubles the Linux build matrix for marginal benefit. MUSL alone covers all use cases since it's fully static.
- *AppImage/Flatpak*: Overkill for a CLI tool. Adds complexity and large bundle sizes.

---

## 6. Native DEB and RPM Packages

### Decision
Use `cargo-deb` for DEB packages and `cargo-generate-rpm` for RPM packages, both built in the release pipeline alongside the binary archives.

### Rationale
- `cargo-deb` reads `[package.metadata.deb]` from `Cargo.toml` and produces a standard `.deb` package installable via `dpkg -i` or `apt install`.
- `cargo-generate-rpm` similarly reads `[package.metadata.generate-rpm]` and produces `.rpm` packages for Fedora/RHEL.
- Both tools are Rust-native (`cargo install`), no additional language toolchain needed.

### Alternatives Considered
- *nfpm*: Go-based, supports DEB/RPM/APK. More flexible but requires a Go toolchain and separate config file.
- *Manual packaging scripts*: Error-prone, hard to maintain across distro versions.
- *Homebrew-only for Linux*: Homebrew on Linux has limited adoption. DEB/RPM cover the vast majority of Linux users.

---

## 7. CI Dependency Chain Optimization

### Decision
Restructure CI jobs into a dependency chain: `fmt → clippy → (test | security) in parallel`.

### Rationale
- Formatting check completes in under 10 seconds. If it fails, no point running clippy or tests.
- Clippy completes in ~30 seconds. If it fails, no point running the 5-10 minute test matrix.
- Test and security jobs run in parallel after clippy passes, maximizing throughput for clean PRs.
- Uses GitHub Actions `needs` to define the DAG.

### Alternatives Considered
- *Flat matrix* (current approach): All jobs run regardless of failures. Wastes CI minutes.
- *Separate workflow files*: Over-engineering. A single `ci.yml` with dependency chains is cleaner.

---

## 8. Branch Protection Enforcement

### Decision
Add a `pr-target-check.yml` workflow using `pull_request_target` trigger that automatically labels and comments on PRs targeting `main` from non-`develop` branches.

### Rationale
- `pull_request_target` runs with read-only permissions by default, safe for automated labeling.
- The check skips PRs from `develop` → `main` (maintainer release merges).
- This is a soft enforcement (label + comment) rather than a hard block (PR close), giving maintainers override capability.

### Alternatives Considered
- *GitHub branch protection rules*: Require repository admin access to configure, not version-controlled, less visible to contributors.
- *Hard PR closure*: Too aggressive for an open-source project. Soft enforcement with labels is more welcoming.
