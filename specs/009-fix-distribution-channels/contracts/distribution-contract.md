# Distribution Interface Contract

**Feature**: `009-fix-distribution-channels` | **Date**: 2026-08-01

## Overview

RGT is distributed through three channels. This contract defines the canonical install commands and the expected behavior of each channel.

## Channel 1: Shell Installer

### Command
```bash
curl -fsSL https://raw.githubusercontent.com/rafael-bianchi/rgt/main/install.sh | sh
```

### Contract
- MUST detect the host platform (macOS arm64, macOS x86_64, Linux x86_64 musl, Linux aarch64 gnu)
- MUST download the correct `.tar.gz` archive from the latest GitHub Release
- MUST download and verify `checksums.txt` against the downloaded archive
- MUST extract the binary to `~/.local/bin/rgt`
- MUST print a warning if `~/.local/bin` is not on PATH
- MUST print `cargo install --git https://github.com/rafael-bianchi/rgt` as the fallback suggestion for unsupported platforms (NOT bare `cargo install rgt`)
- MUST exit non-zero on checksum mismatch

### Archive URL Pattern
```
https://github.com/rafael-bianchi/rgt/releases/download/v{version}/rgt-v{version}-{target}.tar.gz
```

### Supported Targets
| Platform | install.sh Target |
|---|---|
| macOS x86_64 | `x86_64-apple-darwin` |
| macOS arm64 | `aarch64-apple-darwin` |
| Linux x86_64 | `x86_64-unknown-linux-musl` |
| Linux aarch64 | `aarch64-unknown-linux-gnu` |

## Channel 2: Homebrew (Main Repo)

### Command
```bash
brew install rafael-bianchi/rgt/rgt
```

### Contract
- Formula location: `Formula/rgt.rb` in the main repo (`https://github.com/rafael-bianchi/rgt`)
- MUST reference release artifacts at `https://github.com/rafael-bianchi/rgt/releases/download/v{version}/rgt-v{version}-{target}.tar.gz`
- MUST include accurate SHA256 digests for all 4 Unix platforms
- SHA256 values MUST be updated by the CD workflow during release (auto-committed back to the repo)
- MUST support macOS arm64, macOS x86_64, Linux x86_64, Linux aarch64

### CD Integration
The `homebrew` job in `release.yml`:
1. Checks out the main repo
2. Downloads release artifacts from the current release
3. Computes SHA256 digests
4. Replaces placeholder strings in `Formula/rgt.rb`
5. Commits and pushes the updated formula

**Note**: The formula was previously served via `brew install rafael-bianchi/tap/rgt` from a separate `rafael-bianchi/homebrew-tap` repository. This is no longer supported. The `repository-dispatch` to the tap repo is removed.

## Channel 3: Cargo (Git Install)

### Command
```bash
cargo install --git https://github.com/rafael-bianchi/rgt
```

### Contract
- MUST clone the repo and compile from source
- MUST install the `rgt` binary to `~/.cargo/bin/rgt`
- MUST NOT install the unrelated `rgt` crate from crates.io

**Note**: The bare `cargo install rgt` command installs an unrelated crate (red/green syntax tree library). All documentation and scripts MUST use `cargo install --git https://github.com/rafael-bianchi/rgt`.

## Channel 4: Pre-built Binaries (Direct Download)

### URL
```
https://github.com/rafael-bianchi/rgt/releases
```

### Contract
- GitHub Release MUST contain assets for all 5 target platforms
- MUST include `checksums.txt`
- MUST include `.deb` and `.rpm` packages for Linux

## Cross-Cutting Requirements

- All install commands in README, AGENTS.md, and install.sh MUST match this contract
- No references to `rafael-bianchi/tap`, bare `cargo install rgt`, or `rafael-bianchi/homebrew-tap` in active documentation
