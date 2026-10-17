# Data Model & Schema: Distribution & Installation Channels

**Feature Branch**: `002-distribution-installation` | **Date**: 2026-07-31

## Overview

This document specifies the data structures, release asset naming conventions, and manifest schemas for RGT deployment and installation channels.

---

## Release Asset Naming Scheme

Release artifacts published on GitHub Releases MUST follow this exact naming pattern:

```text
rgt-v{VERSION}-{TARGET_TRIPLE}.{EXT}
```

### Supported Platform Mapping

| OS | Architecture | Target Triple | Archive Extension | Asset Filename Example |
| :--- | :--- | :--- | :--- | :--- |
| macOS | Apple Silicon (arm64) | `aarch64-apple-darwin` | `.tar.gz` | `rgt-v0.1.0-aarch64-apple-darwin.tar.gz` |
| macOS | Intel (x86_64) | `x86_64-apple-darwin` | `.tar.gz` | `rgt-v0.1.0-x86_64-apple-darwin.tar.gz` |
| Linux | x86_64 (GNU) | `x86_64-unknown-linux-gnu` | `.tar.gz` | `rgt-v0.1.0-x86_64-unknown-linux-gnu.tar.gz` |
| Linux | ARM64 | `aarch64-unknown-linux-gnu` | `.tar.gz` | `rgt-v0.1.0-aarch64-unknown-linux-gnu.tar.gz` |
| Windows | x86_64 | `x86_64-pc-windows-msvc` | `.zip` | `rgt-v0.1.0-x86_64-pc-windows-msvc.zip` |

---

## Checksum Manifest Format (`checksums.txt`)

Published alongside release assets, formatted compatible with standard `shasum -a 256` / `sha256sum`:

```text
e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855  rgt-v0.1.0-aarch64-apple-darwin.tar.gz
a665a45920422f9d417e4867efdc4fb8a04a1f3fff1fa07e998e86f7f7a27ae3  rgt-v0.1.0-x86_64-apple-darwin.tar.gz
2c26b46b68ffc68ff99b453c1d30413413422d706483bfa0f98a5e886266e7ae  rgt-v0.1.0-x86_64-unknown-linux-gnu.tar.gz
4b227777d4dd1fc61c6f884f48641d02b4d121d3fd328cb08b5531fcacdabf8a  rgt-v0.1.0-aarch64-unknown-linux-gnu.tar.gz
11f3c3a9f074d2847990c7499641737e9545ca873ec89df4fa02efca52c0a4d0  rgt-v0.1.0-x86_64-pc-windows-msvc.zip
```

---

## Rust Struct Definitions for `rgt update`

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubReleaseAsset {
    pub name: String,
    pub browser_download_url: String,
    pub size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubRelease {
    pub tag_name: String,
    pub name: Option<String>,
    pub draft: bool,
    pub prerelease: bool,
    pub published_at: Option<String>,
    pub assets: Vec<GitHubReleaseAsset>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct UpdateCheckResult {
    pub current_version: String,
    pub latest_version: String,
    pub is_update_available: bool,
    pub download_url: Option<String>,
    pub checksum_url: Option<String>,
}
```

---

## Homebrew Formula Template (`Formula/rgt.rb`)

```ruby
class Rgt < Formula
  desc "Rust Graph Tracker: Numeric and date provenance tracking for LLM coding agents"
  homepage "https://github.com/rafael-bianchi/rgt"
  version "0.1.0"
  license "MIT OR Apache-2.0"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/rafael-bianchi/rgt/releases/download/v0.1.0/rgt-v0.1.0-aarch64-apple-darwin.tar.gz"
      sha256 "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    else
      url "https://github.com/rafael-bianchi/rgt/releases/download/v0.1.0/rgt-v0.1.0-x86_64-apple-darwin.tar.gz"
      sha256 "a665a45920422f9d417e4867efdc4fb8a04a1f3fff1fa07e998e86f7f7a27ae3"
    end
  end

  on_linux do
    if Hardware::CPU.intel?
      url "https://github.com/rafael-bianchi/rgt/releases/download/v0.1.0/rgt-v0.1.0-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "2c26b46b68ffc68ff99b453c1d30413413422d706483bfa0f98a5e886266e7ae"
    elsif Hardware::CPU.arm?
      url "https://github.com/rafael-bianchi/rgt/releases/download/v0.1.0/rgt-v0.1.0-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "4b227777d4dd1fc61c6f884f48641d02b4d121d3fd328cb08b5531fcacdabf8a"
    end
  end

  def install
    bin.install "rgt"
  end

  test do
    assert_match "rgt", shell_output("#{bin}/rgt --version")
  end
end
```
