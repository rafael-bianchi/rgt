# Technical Research: Distribution & Installation Channels

**Feature Branch**: `002-distribution-installation` | **Date**: 2026-07-31

## Overview

This document records the architectural research and technical decisions for RGT's deployment, installation scripts, release automation, Homebrew packaging, and binary self-updater.

---

## 1. POSIX Shell One-Liner Installer (`install.sh`)

### Decision
Implement `install.sh` using strict POSIX shell syntax (`#!/bin/sh`) with fallback logic for `curl` / `wget`, `tar` / `unzip`, and `shasum` / `sha256sum`.

### Rationale
- **Universal Portability**: POSIX `sh` runs on all Unix-like platforms (macOS, Ubuntu, Debian, Fedora, Arch, Alpine Linux) without requiring Bash.
- **Environment Detection**:
  - `OS`: `uname -s` mapped to `darwin` or `linux`.
  - `ARCH`: `uname -m` mapped to `x86_64` or `aarch64`.
- **Target Installation Directory**:
  - Default: `$HOME/.local/bin` (no root/sudo required).
  - Fallback: `/usr/local/bin` if `~/.local/bin` is not writable or `--system` flag is passed.
- **PATH Check**: Detects if installation directory is present in `$PATH`. If missing, emits customized export instructions for `.zshrc`, `.bashrc`, and `.config/fish/config.fish`.

### Alternatives Evaluated
- *Bash-Specific Installer*: Fails on minimal containers (Alpine/musl with `dash`/`ash`).
- *Python/Node.js Installer Script*: Violates Constitution Principle I by requiring a pre-installed language runtime.

---

## 2. GitHub Actions Release Cross-Compilation Matrix

### Decision
Use `.github/workflows/release.yml` with native runners (`ubuntu-latest`, `macos-latest`, `macos-13`, `windows-latest`) and `cross-rs` for ARM64 Linux targets.

### Rationale
- **Target Triples**:
  1. `x86_64-apple-darwin` (macOS Intel)
  2. `aarch64-apple-darwin` (macOS Apple Silicon)
  3. `x86_64-unknown-linux-gnu` (Linux x86_64)
  4. `aarch64-unknown-linux-gnu` (Linux ARM64)
  5. `x86_64-pc-windows-msvc` (Windows x86_64)
- **Binary Optimization**: Run `strip` (or `cargo build --release` with `panic = "abort"`, `lto = true`, `codegen-units = 1`, `strip = true`) to reduce binary asset size from ~30MB to <10MB.
- **Archive Generation**:
  - Unix targets packaged as `.tar.gz`.
  - Windows targets packaged as `.zip`.
- **Checksum Manifest**: Produce `checksums.txt` containing SHA-256 digests for all generated archives.

---

## 3. Homebrew Tap Formula (`Formula/rgt.rb`)

### Decision
Maintain an automated Homebrew formula `rgt.rb` supporting both macOS Intel and Apple Silicon pre-compiled release bottles/archives.

### Rationale
- **Developer UX**: Enables `brew install rafael-bianchi/tap/rgt` or `brew install rgt`.
- **Automation**: A post-release step in `.github/workflows/release.yml` uses `mislav/bump-homebrew-formula-action` or custom GitHub API dispatch to automatically update `rgt.rb` with the new version string, archive URL, and SHA-256 hash.

---

## 4. Built-in Self-Updater (`rgt update`)

### Decision
Implement `rgt update` inside the Rust binary utilizing GitHub Releases REST API (`https://api.github.com/repos/rafael-bianchi/rgt/releases/latest`).

### Rationale
- **Atomic File Replacement**:
  - On Unix: Download new binary to `rgt.tmp`, set executable permissions (`chmod 0755`), and atomically rename over the current binary path via `std::fs::rename`.
  - On Windows: Rename running binary to `rgt.old`, write new binary to `rgt.exe`, and schedule `rgt.old` deletion.
- **Verification**: Verify SHA-256 hash against `checksums.txt` before replacing the binary.
- **Flags**:
  - `rgt update`: Download and apply update.
  - `rgt update --check`: Check for updates and exit `0` (update available) or `1` (up to date).
