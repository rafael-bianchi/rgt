# Installer Script Interface Contract: `install.sh`

**Feature Branch**: `002-distribution-installation` | **Date**: 2026-07-31

## Usage Interface

```bash
# Standard one-liner execution:
curl -fsSL https://raw.githubusercontent.com/rafael-bianchi/rgt/main/install.sh | sh

# With custom parameters via environment variables or flags:
curl -fsSL https://raw.githubusercontent.com/rafael-bianchi/rgt/main/install.sh | sh -s -- [OPTIONS]
```

---

## Supported Flags & Options

```text
Options:
  -b, --bin-dir <DIR>    Directory to install binary [default: ~/.local/bin]
  -v, --version <VER>    Specific version tag to install [default: latest]
  --system               Install globally to /usr/local/bin (requires sudo write permissions)
  -h, --help             Print help information
```

---

## Environment Variables

| Variable | Description | Example |
| :--- | :--- | :--- |
| `RGT_INSTALL_DIR` | Custom installation directory override | `RGT_INSTALL_DIR=/opt/bin` |
| `RGT_VERSION` | Specific release version tag | `RGT_VERSION=v0.1.0` |
| `GITHUB_TOKEN` | GitHub API token to bypass rate limits | `GITHUB_TOKEN=ghp_...` |

---

## Exit Statuses

- `0`: Installation completed successfully, binary verified in target directory.
- `1`: Unsupported OS or architecture detected.
- `2`: Network download or checksum verification failed.
- `3`: Permission denied when writing to target binary directory.
