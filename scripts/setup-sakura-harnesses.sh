#!/usr/bin/env bash
# ==============================================================================
# Sakura Server Setup & Harness Test Runner for RGT (Ubuntu 24.04)
# ==============================================================================
# Sets up the complete testing environment on Sakura Server (or any Ubuntu 24.04
# host) and runs the comprehensive RGT agent harness verification suite.
#
# Supports two modes:
#   1. Container mode (--docker, recommended): builds and runs isolated Docker container
#   2. Native mode (--native): installs tools directly onto Ubuntu host
# ==============================================================================

set -euo pipefail

MODE="docker"

for arg in "$@"; do
    case "$arg" in
        --docker)
            MODE="docker"
            ;;
        --native)
            MODE="native"
            ;;
        --help|-h)
            echo "Usage: $0 [--docker | --native]"
            echo "  --docker: (Default) Run tests inside isolated Ubuntu 24.04 Docker container"
            echo "  --native: Install dependencies directly on Ubuntu host and run tests"
            exit 0
            ;;
        *)
            echo "Unknown argument: $arg"
            echo "Run '$0 --help' for usage."
            exit 1
            ;;
    esac
done

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

echo "=== RGT Harness Test Setup on Sakura Server ==="
echo "Working directory: $REPO_ROOT"
echo "Execution mode:    $MODE"

if [ "$MODE" = "docker" ]; then
    if ! command -v docker >/dev/null 2>&1; then
        echo "Docker not found on host. Falling back to native mode or install Docker."
        MODE="native"
    else
        echo "Building and running harness verification container..."
        docker build -t rgt-harness-env -f docker/Dockerfile.harnesses .
        docker run --rm -v "$REPO_ROOT:/workspace/rgt" rgt-harness-env ./scripts/test-harnesses-e2e.sh
        echo "=== Sakura Docker test execution finished successfully! ==="
        exit 0
    fi
fi

if [ "$MODE" = "native" ]; then
    echo "Setting up native Ubuntu 24.04 environment..."

    if [ "$EUID" -ne 0 ]; then
        SUDO="sudo"
    else
        SUDO=""
    fi

    # 1. Update and base packages
    $SUDO apt-get update -qq
    $SUDO apt-get install -y -qq \
        build-essential \
        ca-certificates \
        curl \
        git \
        gnupg \
        jq \
        libssl-dev \
        pkg-config \
        python3 \
        python3-pip \
        python3-venv \
        sqlite3

    # 2. Rust
    if ! command -v cargo >/dev/null 2>&1; then
        echo "Installing Rust toolchain..."
        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable --profile minimal
        source "$HOME/.cargo/env"
    fi

    # 3. Node.js 22 LTS
    if ! command -v node >/dev/null 2>&1; then
        echo "Installing Node.js 22 LTS..."
        curl -fsSL https://deb.nodesource.com/setup_22.x | $SUDO bash -
        $SUDO apt-get install -y -qq nodejs
    fi

    # 4. NPM Agent Harnesses
    echo "Installing NPM AI coding agent harnesses..."
    $SUDO npm install -g --no-fund --no-audit \
        @anthropic-ai/claude-code \
        opencode-ai \
        @openai/codex \
        @earendil-works/pi-coding-agent \
        @kilocode/cli \
        @github/copilot \
        @google/gemini-cli || true

    # 5. Python Agent Harnesses in dedicated venvs
    echo "Installing Python AI coding agent harnesses..."
    $SUDO mkdir -p /opt/harnesses
    $SUDO python3 -m venv /opt/harnesses/vibe
    $SUDO /opt/harnesses/vibe/bin/pip install --no-cache-dir mistral-vibe || true
    $SUDO ln -sf /opt/harnesses/vibe/bin/vibe /usr/local/bin/vibe || true

    $SUDO python3 -m venv /opt/harnesses/hermes
    $SUDO /opt/harnesses/hermes/bin/pip install --no-cache-dir hermes-agent || true
    $SUDO ln -sf /opt/harnesses/hermes/bin/hermes /usr/local/bin/hermes || true

    # 6. Build RGT
    echo "Building RGT release binary..."
    cargo build --release --locked --bin rgt
    export RGT_BIN="$REPO_ROOT/target/release/rgt"

    # 7. Execute tests
    echo "Running end-to-end harness verification suite..."
    ./scripts/test-harnesses-e2e.sh
    echo "=== Sakura Native test execution finished successfully! ==="
fi
