# Data Model & Schema: CI/CD Pipeline Configuration

**Feature Branch**: `003-devops-enhancement` | **Date**: 2026-07-31

## Overview

This document specifies the configuration entities, job dependency graphs, and release versioning model for the hardened CI/CD pipeline. Since the implementation is GitHub Actions workflow files (YAML), the "data model" describes the logical structure of jobs, triggers, and artifacts rather than database schemas.

---

## CI Job Dependency Graph

```
pull_request → [develop, main]
         │
         ▼
    ┌─────────┐
    │  fmt    │  ~10s — rustfmt check
    └────┬────┘
         │ success only
         ▼
    ┌─────────┐
    │ clippy  │  ~45s — static analysis
    └────┬────┘
         │ success only
    ┌────┴───────────┐
    ▼                ▼
┌────────┐    ┌──────────────┐
│  test  │    │  security    │  parallel
│ matrix │    │ cargo-audit  │
│ 3 OS   │    │ patterns     │
│        │    │ new deps     │
│        │    │ clippy sec   │
└────────┘    └──────────────┘
```

**Entity: CI Job**
| Field | Type | Description |
|-------|------|-------------|
| `name` | string | Human-readable job name |
| `needs` | string[] | Upstream jobs that must succeed before this job runs |
| `runs-on` | string | Runner OS label |
| `if` | string | Conditional gate expression |
| `steps` | Step[] | Ordered execution steps |

---

## CD Release Orchestration

### Development Branch (`develop`)

```
push to develop
      │
      ▼
┌──────────────┐
│ pre-release  │  Compute next dev version from conventional commits
│ compute      │  tag = dev-{X}.{Y}.{Z}-rc.{run_number}
└──────┬───────┘
       │
       ▼
┌──────────────────┐
│ release.yml      │  prerelease = true
│ (reusable)       │
└──────────────────┘
```

### Main Branch (`main`)

```
push to main
      │
      ▼
┌──────────────────┐
│ release-please   │  Analyze conventional commits
│                  │  Create/update release PR
│                  │  On merge: create tag, generate changelog
└──────┬───────────┘
       │ (if release created)
       ▼
┌──────────────────┐
│ release.yml      │  prerelease = false
│ (reusable)       │
└──────┬───────────┘
       │
  ┌────┴────────────┐
  ▼    ▼             ▼
 Discord  Homebrew  latest
 notify   tap       tag
```

**Entity: Release Trigger**
| Field | Type | Description |
|-------|------|-------------|
| `source_branch` | enum | `develop` or `main` |
| `trigger_type` | enum | `push`, `workflow_dispatch` |
| `conventional_commits` | Commit[] | List of commits since last release tag |

**Entity: Release Version**
| Field | Type | Description |
|-------|------|-------------|
| `major` | integer | Breaking changes increment |
| `minor` | integer | Feature additions increment |
| `patch` | integer | Bug fixes increment |
| `tag` | string | Full tag: `v{major}.{minor}.{patch}` or `dev-{version}-rc.{n}` |
| `prerelease` | boolean | Whether this is a pre-release |
| `release_assets` | Artifact[] | Built binaries and packages |

---

## Release Artifact Matrix

| OS | Arch | Target | Archive | Package |
|----|------|--------|---------|---------|
| macOS | x86_64 | `x86_64-apple-darwin` | `.tar.gz` | — |
| macOS | aarch64 | `aarch64-apple-darwin` | `.tar.gz` | — |
| Linux | x86_64 | `x86_64-unknown-linux-musl` | `.tar.gz` | `.deb` |
| Linux | aarch64 | `aarch64-unknown-linux-gnu` | `.tar.gz` | `.rpm` |
| Windows | x86_64 | `x86_64-pc-windows-msvc` | `.zip` | — |

**Entity: Release Artifact**
| Field | Type | Description |
|-------|------|-------------|
| `name` | string | Filename: `rgt-v{version}-{target}.{ext}` or `rgt_{version}_{arch}.{deb|rpm}` |
| `target_triple` | string | Rust target triple |
| `archive_type` | enum | `tar.gz`, `zip`, `deb`, `rpm` |
| `checksum` | string | SHA-256 hex digest |
| `size_bytes` | integer | Compressed file size |

---

## Security Scan Report Structure

```text
## Security Scan Results

### Dependency Vulnerabilities
- [CVE-YYYY-XXXX] crate@version: description (severity: HIGH)
- No known vulnerabilities detected

### Critical Files Modified
- src/main.rs
- Cargo.toml

### Dangerous Code Patterns
+ unsafe { ... }  (src/foo.rs:42)
+ Command::new("sh") (src/bar.rs:15)

### Dependencies Changes
+ ureq = "2.10"
+ sha2 = "0.10"

### Clippy Security Lints
warning: used `unwrap()` on a `Result` value
```

**Entity: Security Scan Report**
| Field | Type | Description |
|-------|------|-------------|
| `vulnerabilities` | Vulnerability[] | CVEs found in dependencies |
| `critical_files` | string[] | Security-critical files modified in PR |
| `dangerous_patterns` | Pattern[] | Code patterns flagged for review |
| `new_dependencies` | Dependency[] | Newly added crate dependencies |
| `clippy_security_lints` | Lint[] | Security-related clippy warnings |
