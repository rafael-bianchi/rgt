# RGT Multi-Agent Harness & Provenance Testing Environment

This testing environment validates RGT across the full **13-agent AI coding harness set** (Claude Code, Cursor, Codex CLI, Windsurf, Copilot, Gemini CLI, Mistral Vibe, OpenCode, Pi, Hermes, Cline/Roo Code, Antigravity, and Kilo Code).

It verifies:
1. **Agent Setup & Diagnostics (`rgt init` & `rgt doctor`)**: Auto-detection, per-agent targeting, `--global` user configuration, and `--force` overwrite without corrupting unowned files.
2. **Passive Hook Processing (`rgt hook post`)**: Validates completed-read tool event ingestion across dialects (Claude Code, OpenCode, Codex, Pi, Hermes, Cursor, Windsurf, etc.), source snapshot byte matching, line mapping, and tamper rejection.
3. **PDF Provenance Capture (Spec 033)**: Base64 envelope decoding, local text extraction, provenance recording, and Claude Code `additionalContext` injection.
4. **Attribution & PROV-O Turtle (`rgt graph --format ttl`)**: Verification of `prov:wasAssociatedWith <agent>` activities.
5. **Core Provenance Operations**: Extraction (`rgt record`), verified arithmetic derivations (`EXPRESSION`), typed temporal gap computations (`DATE_DIFF`), duration operations (`DURATION_SUM`, `DURATION_AVG`), lineage ancestry queries (`rgt query`), and garbage collection (`rgt gc --vacuum`).

---

## 🚀 Running the Tests

### Option 1: Locally via Docker / OrbStack

Run the entire verification suite inside an isolated Ubuntu 24.04 container with all harnesses installed:

```bash
# Build and run the test container
docker compose -f docker/docker-compose.harnesses.yml run --rm test
```

To drop into an interactive shell inside the test environment:

```bash
docker compose -f docker/docker-compose.harnesses.yml run --rm shell
```

---

### Option 2: On the Sakura Server (Ubuntu 24.04)

The Sakura server runs Ubuntu 24.04. You can run either via Docker or natively:

#### A. Via Docker (Recommended)

```bash
ssh rafael@sakura-ubuntu-server-2404.local  # or your Tailscale / LAN IP
cd /path/to/rgt
./scripts/setup-sakura-harnesses.sh --docker
```

#### B. Bare Metal (Native Ubuntu 24.04)

```bash
./scripts/setup-sakura-harnesses.sh --native
```

The script will automatically install Node.js 22 LTS, Rust, NPM agent CLIs, Python agent harnesses in isolated virtual environments, build RGT, and run `./scripts/test-harnesses-e2e.sh`.

---

### Option 3: GitHub Actions Runner (Merging to `main`)

The workflow `.github/workflows/harness-matrix.yml` is configured to run automatically on:
- Every `push` to `main`
- Every `pull_request` targeting `main`
- Manual trigger via `workflow_dispatch` in the GitHub Actions tab

It provisions an Ubuntu 24.04 runner (`ubuntu-latest`), installs all 13 harnesses, compiles RGT, executes `./scripts/test-harnesses-e2e.sh`, and outputs a test summary to `$GITHUB_STEP_SUMMARY`.

---

## 🧪 Test Suite Structure (`scripts/test-harnesses-e2e.sh`)

| Suite | Description | Key Assertions |
|---|---|---|
| **Suite 1: Core CLI Contracts** | Subcommands and semver parity | `rgt --version` == `rgt version`, 9 core subcommands present |
| **Suite 2: Agent Configuration** | `rgt init` across all 13 agents + aliases | Verifies `.claude/settings.json`, `.cursor/hooks.json`, `.codex/hooks.json`, `.devin/hooks.json`, `.opencode/plugins/rgt.ts`, `.pi/extensions/rgt.ts`, `.hermes/plugins/rgt/plugin.py`, `.clinerules`, `.agents/rules/`, `.kilocode/rules/` |
| **Suite 3: Doctor Diagnostics** | Health diagnostics | `rgt doctor` exits 1 on uninitialized, exits 0 on configured project |
| **Suite 4: Passive Hook Ingestion** | Ingestion across dialects | Claude Code `PostToolUse`, OpenCode `tool.execute.after`, Codex `read_file`, PDF envelope decoding, fail-open on malformed/tampered payloads |
| **Suite 5: Provenance & Derivations** | Value extraction & typed arithmetic | `EXPRESSION` arithmetic verification, rejection of false claims, `DATE_DIFF` elapsed gaps, `rgt graph` (text, mermaid, dot, ttl), `rgt query`, `rgt gc` |
| **Suite 6: Native Live Execution** | CLI presence & smoke checks | Native execution of available CLIs (`opencode`, `codex`, `pi`, `claude`, `vibe`, `hermes`, `kilo`, `agy`) |
