# Spec 049: Agent Installation, Guidance, and Doctor Diagnostics Hardening

## Overview
This specification fixes agent installation scoping (`--global` vs project scope), eliminates false positives and false negatives in `detect_installed_agents`, resolves the `rgt doctor` exit code 1 error when project and user configurations coexist, and ensures `--force` is honored across all configuration writers.

## Requirements

### FR-001: Strict Global vs. Project Scope Separation
- `write_copilot`:
  - When `global` is `true`: configure global VS Code `settings.json` and user Copilot CLI `AGENTS.md`. Do NOT write project-local `.github/copilot-instructions.md`.
  - When `global` is `false`: configure project `.vscode/settings.json` and project `.github/copilot-instructions.md`. Do NOT mutate global VS Code `settings.json`.
- `write_gemini`:
  - When `global` is `true`: write to `~/.gemini/hooks.toml` and `~/.gemini/GEMINI.md`.
  - When `global` is `false`: write to project `.gemini/hooks.toml` and project `GEMINI.md`.
- `write_vibe`:
  - When `global` is `true`: write to `~/.vibe/hooks.toml` and `~/.vibe/AGENTS.md`.
  - When `global` is `false`: write to project `.vibe/hooks.toml` and project `AGENTS.md`.
- Guidance-only agents (`cursor`, `codex`, `windsurf`, `cline`, `antigravity`, `kilocode`):
  - When `global` is `true`: write rules/guidance to the agent's user-level configuration directory in `home`, NOT into the current project working directory.
  - When `global` is `false`: write rules/guidance to the project-relative path.

### FR-002: Accurate Agent Detection
- `detect_installed_agents`:
  - Copilot: do not trigger on bare `Code/User/settings.json` existence; require `copilot_cli_config_dir.exists()`, project `.github/copilot-instructions.md.exists()`, or the user settings file to mention `"github.copilot"`.
  - Codex CLI: do not trigger on bare `AGENTS.md`; require `.codex` directory to exist in home or project.
  - Antigravity: do not trigger on bare `.agents/`; require `.agents/rules/` or `~/.gemini/antigravity-cli/` to exist.
  - Windsurf: detect user installs in `~/.codeium/windsurf` as well as `~/.windsurf`.

### FR-003: Doctor Layered Registration Resolution
- In `inspect_surface`, when an agent has both a project-level and a user-level configuration, the project-level registration takes precedence over the user-level registration.
- Layered configuration is reported as `Active` (healthy), NOT `RegistrationHealth::Ambiguous` (`Error`, exit 1).

### FR-004: Universal `--force` Flag Support
- `rgt init --force` must overwrite conflicting or existing configuration blocks and rules without error, passing `force` through to all JSON, TOML, and text block modifiers.
