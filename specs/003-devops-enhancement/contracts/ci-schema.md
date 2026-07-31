# CI Job Interface Contract: `ci.yml`

**Feature Branch**: `003-devops-enhancement` | **Date**: 2026-07-31

## Workflow Trigger

```yaml
on:
  pull_request:
    branches: [develop, main]
```

## Job Dependency Contract

```
check-test-presence (optional, develop only)
         │
    ┌────▼────┐
    │  fmt    │  Must complete in <30s
    └────┬────┘
         │ (if success)
    ┌────▼────┐
    │ clippy  │  Must complete in <1min
    └────┬────┘
         │ (if success)
    ┌────┴──────────────┐
    ▼                   ▼
┌────────┐     ┌──────────────┐
│  test  │     │  security    │
└────────┘     └──────────────┘
```

## Job Specifications

### fmt
- **Command**: `cargo fmt --all -- --check`
- **Failure behavior**: Blocks clippy and test jobs
- **Timeout**: 30s

### clippy
- **Command**: `cargo clippy --all-targets`
- **Needs**: `fmt` (success)
- **Failure behavior**: Blocks test and security jobs
- **Timeout**: 2min

### test
- **Command**: `cargo test --all`
- **Needs**: `clippy` (success)
- **Matrix**: `ubuntu-latest`, `windows-latest`, `macos-latest`
- **Parallel**: Runs concurrently with `security`
- **Timeout**: 15min

### security
- **Needs**: `clippy` (success)
- **Steps** (in order):
  1. `cargo-audit` — CVE check (fail-open if database unreachable)
  2. Critical files change detection — git diff analysis
  3. Dangerous code pattern scan — grep-based diff scan
  4. New dependency detection — Cargo.toml diff analysis
  5. Clippy security lints — `-W clippy::unwrap_used -W clippy::panic`
- **Parallel**: Runs concurrently with `test`
- **Timeout**: 5min

## Exit Codes

- `0`: All checks passed
- `1`: Security check failed (vulnerability, dangerous pattern, or clippy security lint)
- `2`: Infrastructure error (dependency database unreachable)
