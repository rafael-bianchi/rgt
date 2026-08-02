#!/bin/sh
# install.sh - RGT (Rust Graph Tracker) one-liner shell installer
# Usage: curl -fsSL https://raw.githubusercontent.com/rafael-bianchi/rgt/main/install.sh | sh
#   or:  ./install.sh [OPTIONS]

set -u

RGT_GITHUB_REPO="rafael-bianchi/rgt"
RGT_INSTALL_DIR_DEFAULT="${HOME}/.local/bin"
RGT_BIN_NAME="rgt"

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

BIN_DIR="${RGT_INSTALL_DIR:-}"
VERSION="${RGT_VERSION:-latest}"
SYSTEM_INSTALL=0
DRY_RUN=0

main() {
    parse_args "$@"

    if [ "${BIN_DIR}" = "" ]; then
        if [ "${SYSTEM_INSTALL}" -eq 1 ]; then
            BIN_DIR="/usr/local/bin"
        else
            BIN_DIR="${RGT_INSTALL_DIR_DEFAULT}"
        fi
    fi

    detect_os
    detect_arch
    build_target_triple

    info "RGT installer"
    info "  OS:        ${RGT_OS} (${RGT_OS_RAW})"
    info "  Arch:      ${RGT_ARCH} (${RGT_ARCH_RAW})"
    info "  Target:    ${RGT_TARGET_TRIPLE}"
    info "  Version:   ${VERSION}"
    info "  Dest:      ${BIN_DIR}"

    if [ "${DRY_RUN}" -eq 1 ]; then
        info "Dry-run complete."
        exit 0
    fi

    check_deps

    fetch_release
    download_and_verify
    extract_archive
    install_binary
    check_path

    echo ""
    success "RGT ${RGT_RELEASE_TAG} installed successfully to ${BIN_DIR}/${RGT_BIN_NAME}"
}

parse_args() {
    while [ $# -gt 0 ]; do
        case "$1" in
            -b|--bin-dir)
                if [ $# -lt 2 ]; then
                    err "Missing argument for $1"
                    exit 2
                fi
                BIN_DIR="$2"
                shift 2
                ;;
            -v|--version)
                if [ $# -lt 2 ]; then
                    err "Missing argument for $1"
                    exit 2
                fi
                VERSION="$2"
                shift 2
                ;;
            --system)
                SYSTEM_INSTALL=1
                shift
                ;;
            --dry-run)
                DRY_RUN=1
                shift
                ;;
            -h|--help)
                print_help
                exit 0
                ;;
            --)
                shift
                break
                ;;
            -*)
                err "Unknown option: $1"
                print_help
                exit 2
                ;;
            *)
                shift
                ;;
        esac
    done
}

print_help() {
    cat <<EOF
RGT (Rust Graph Tracker) one-liner shell installer

Options:
  -b, --bin-dir <DIR>    Directory to install binary [default: ~/.local/bin]
  -v, --version <VER>    Specific version tag to install [default: latest]
  --system               Install globally to /usr/local/bin
  --dry-run              Validate environment without downloading
  -h, --help             Print this help

Environment Variables:
  RGT_INSTALL_DIR        Custom installation directory
  RGT_VERSION            Specific release version tag
  GITHUB_TOKEN           GitHub API token to bypass rate limits

Exit Codes:
  0  Success
  1  Unsupported OS or architecture
  2  Network download or checksum failure
  3  Permission denied
EOF
}

detect_os() {
    RGT_OS_RAW="${RGT_INSTALL_TEST_OS:-$(uname -s)}"
    case "${RGT_OS_RAW}" in
        Darwin)  RGT_OS="darwin" ;;
        Linux)   RGT_OS="linux" ;;
        *)
            err "Unsupported OS: ${RGT_OS_RAW}"
            err "Install from source: cargo install --git https://github.com/rafael-bianchi/rgt"
            exit 1
            ;;
    esac
}

detect_arch() {
    RGT_ARCH_RAW="${RGT_INSTALL_TEST_ARCH:-$(uname -m)}"
    case "${RGT_ARCH_RAW}" in
        x86_64|amd64)  RGT_ARCH="x86_64" ;;
        aarch64|arm64) RGT_ARCH="aarch64" ;;
        *)
            err "Unsupported architecture: ${RGT_ARCH_RAW}"
            err "Install from source: cargo install --git https://github.com/rafael-bianchi/rgt"
            exit 1
            ;;
    esac
}

build_target_triple() {
    case "${RGT_OS}" in
        darwin) RGT_TARGET_TRIPLE="${RGT_ARCH}-apple-darwin" ;;
        linux)  case "${RGT_ARCH}" in
                     x86_64) RGT_TARGET_TRIPLE="x86_64-unknown-linux-musl" ;;
                     *)      RGT_TARGET_TRIPLE="${RGT_ARCH}-unknown-linux-gnu" ;;
                 esac ;; 
    esac
}

check_deps() {
    if ! has curl && ! has wget; then
        err "Missing required tool: curl or wget. Install one and retry."
        exit 2
    fi
    if ! has tar; then
        err "Missing required tool: tar"
        exit 2
    fi
    if ! has shasum && ! has sha256sum; then
        err "Missing required tool: shasum or sha256sum"
        exit 2
    fi
}

fetch_release() {
    local api_url
    if [ "${VERSION}" = "latest" ]; then
        api_url="https://api.github.com/repos/${RGT_GITHUB_REPO}/releases/latest"
    else
        api_url="https://api.github.com/repos/${RGT_GITHUB_REPO}/releases/tags/${VERSION}"
    fi

    info "Fetching release info..."

    local auth_header=""
    if [ -n "${GITHUB_TOKEN:-}" ]; then
        auth_header="Authorization: Bearer ${GITHUB_TOKEN}"
    fi

    local response
    if has curl; then
        response=$(curl -fsSL ${auth_header:+-H "${auth_header}"} "${api_url}" 2>/dev/null) || {
            err "Failed to fetch release info from GitHub API."
            err "If rate-limited, set GITHUB_TOKEN environment variable."
            exit 2
        }
    else
        response=$(wget -qO- ${auth_header:+--header="${auth_header}"} "${api_url}" 2>/dev/null) || {
            err "Failed to fetch release info from GitHub API."
            exit 2
        }
    fi

    RGT_RELEASE_TAG=$(echo "${response}" | grep '"tag_name"' | head -1 | sed 's/.*"tag_name": *"\([^"]*\)".*/\1/')
    [ -z "${RGT_RELEASE_TAG}" ] && { err "Could not determine release tag from response."; exit 2; }

    local version_no_v="${RGT_RELEASE_TAG#v}"
    local ext="tar.gz"
    RGT_ARCHIVE_NAME="rgt-v${version_no_v}-${RGT_TARGET_TRIPLE}.${ext}"

    RGT_DOWNLOAD_URL=$(echo "${response}" | grep '"browser_download_url"' | grep "${RGT_ARCHIVE_NAME}" | head -1 | sed 's/.*"browser_download_url": *"\([^"]*\)".*/\1/')
    [ -z "${RGT_DOWNLOAD_URL}" ] && { err "No release asset found for ${RGT_TARGET_TRIPLE}"; exit 2; }

    RGT_CHECKSUMS_URL=$(echo "${response}" | grep '"browser_download_url"' | grep 'checksums.txt' | head -1 | sed 's/.*"browser_download_url": *"\([^"]*\)".*/\1/')

    info "Release: ${RGT_RELEASE_TAG}"
    info "Asset:   ${RGT_ARCHIVE_NAME}"
}

download_and_verify() {
    TMPDIR="${TMPDIR:-/tmp}"
    RGT_TMP_DIR=$(mktemp -d "${TMPDIR}/rgt-install.XXXXXX")
    trap 'rm -rf "${RGT_TMP_DIR}"' EXIT

    RGT_ARCHIVE_PATH="${RGT_TMP_DIR}/${RGT_ARCHIVE_NAME}"
    RGT_CHECKSUMS_PATH="${RGT_TMP_DIR}/checksums.txt"

    info "Downloading ${RGT_ARCHIVE_NAME}..."

    if has curl; then
        curl -fsSL -o "${RGT_ARCHIVE_PATH}" "${RGT_DOWNLOAD_URL}" || { err "Download failed."; exit 2; }
        if [ -n "${RGT_CHECKSUMS_URL}" ]; then
            curl -fsSL -o "${RGT_CHECKSUMS_PATH}" "${RGT_CHECKSUMS_URL}" 2>/dev/null || true
        fi
    else
        wget -qO "${RGT_ARCHIVE_PATH}" "${RGT_DOWNLOAD_URL}" || { err "Download failed."; exit 2; }
        if [ -n "${RGT_CHECKSUMS_URL}" ]; then
            wget -qO "${RGT_CHECKSUMS_PATH}" "${RGT_CHECKSUMS_URL}" 2>/dev/null || true
        fi
    fi

    if [ -f "${RGT_CHECKSUMS_PATH}" ]; then
        info "Verifying SHA-256 checksum..."
        local expected_hash
        expected_hash=$(grep "${RGT_ARCHIVE_NAME}" "${RGT_CHECKSUMS_PATH}" | awk '{print $1}')

        if [ -z "${expected_hash}" ]; then
            warn "No checksum entry for ${RGT_ARCHIVE_NAME}. Skipping verification."
            return
        fi

        local actual_hash
        if has shasum; then
            actual_hash=$(shasum -a 256 "${RGT_ARCHIVE_PATH}" | awk '{print $1}')
        else
            actual_hash=$(sha256sum "${RGT_ARCHIVE_PATH}" | awk '{print $1}')
        fi

        if [ "${actual_hash}" != "${expected_hash}" ]; then
            err "Checksum verification FAILED."
            err "  Expected: ${expected_hash}"
            err "  Got:      ${actual_hash}"
            err "Corrupted download. Aborting."
            rm -f "${RGT_ARCHIVE_PATH}"
            exit 2
        fi
        info "Checksum OK."
    else
        warn "checksums.txt not available. Skipping verification."
    fi
}

extract_archive() {
    info "Extracting..."
    case "${RGT_ARCHIVE_NAME}" in
        *.tar.gz)
            tar -xzf "${RGT_ARCHIVE_PATH}" -C "${RGT_TMP_DIR}" || { err "Extraction failed."; exit 2; }
            ;;
        *.zip)
            if has unzip; then
                unzip -qo "${RGT_ARCHIVE_PATH}" -d "${RGT_TMP_DIR}" || { err "Extraction failed."; exit 2; }
            else
                err "Missing required tool: unzip"
                exit 2
            fi
            ;;
        *)
            err "Unknown archive format: ${RGT_ARCHIVE_NAME}"
            exit 2
            ;;
    esac
}

install_binary() {
    local extracted
    extracted=$(find "${RGT_TMP_DIR}" -name "${RGT_BIN_NAME}" -type f 2>/dev/null | head -1)
    if [ -z "${extracted}" ]; then
        err "Could not find ${RGT_BIN_NAME} binary in extracted archive."
        exit 2
    fi

    mkdir -p "${BIN_DIR}" 2>/dev/null || {
        if [ "${SYSTEM_INSTALL}" -eq 1 ]; then
            warn "Cannot write to ${BIN_DIR}. Attempting sudo..."
            sudo mkdir -p "${BIN_DIR}" || { err "Permission denied: ${BIN_DIR}"; exit 3; }
        else
            err "Permission denied: ${BIN_DIR}"
            exit 3
        fi
    }

    local dest="${BIN_DIR}/${RGT_BIN_NAME}"
    if ! cp "${extracted}" "${dest}" 2>/dev/null; then
        if [ "${SYSTEM_INSTALL}" -eq 1 ]; then
            warn "Cannot write to ${dest}. Attempting sudo..."
            sudo cp "${extracted}" "${dest}" || { err "Permission denied."; exit 3; }
            sudo chmod 0755 "${dest}"
        else
            err "Permission denied: ${dest}"
            exit 3
        fi
    else
        chmod 0755 "${dest}" 2>/dev/null || true
    fi

    info "Installed: ${dest}"
}

check_path() {
    if echo "${PATH}" | tr ':' '\n' | grep -qxF "${BIN_DIR}"; then
        info "${BIN_DIR} is in PATH."
    else
        echo ""
        warn "${BIN_DIR} is not in your PATH."
        echo ""
        case "$(basename "${SHELL:-sh}")" in
            bash) echo "  Add to ~/.bashrc:  export PATH=\"${BIN_DIR}:\$PATH\"" ;;
            zsh)  echo "  Add to ~/.zshrc:   export PATH=\"${BIN_DIR}:\$PATH\"" ;;
            fish) echo "  Run: fish_add_path ${BIN_DIR}" ;;
            *)    echo "  Add ${BIN_DIR} to your shell's PATH." ;;
        esac
        echo ""
    fi
}

has() {
    command -v "$1" >/dev/null 2>&1
}

info()  { echo "  ${GREEN}[INFO]${NC}  $*"; }
warn()  { echo "  ${YELLOW}[WARN]${NC}  $*" >&2; }
err()   { echo "  ${RED}[ERROR]${NC} $*" >&2; }
success() { echo "${GREEN}[OK]${NC} $*"; }

main "$@"
