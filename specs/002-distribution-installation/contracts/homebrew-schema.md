# Homebrew Interface Contract: `Formula/rgt.rb`

**Feature Branch**: `002-distribution-installation` | **Date**: 2026-07-31

## Usage Interface

```bash
# Tap repository and install RGT:
brew install rafael-bianchi/tap/rgt

# Upgrade installed RGT formula:
brew upgrade rgt
```

---

## Automated Formula Update Workflow

1. Triggered on GitHub release tag creation (`v*`).
2. Computes SHA-256 hashes for `rgt-vX.Y.Z-aarch64-apple-darwin.tar.gz`, `rgt-vX.Y.Z-x86_64-apple-darwin.tar.gz`, and `rgt-vX.Y.Z-x86_64-unknown-linux-gnu.tar.gz`.
3. Dispatches pull request or direct commit to `rafael-bianchi/homebrew-tap` repository updating `Formula/rgt.rb`.
