# Implementation Plan: Agent Installation, Guidance, and Doctor Diagnostics Hardening

## Proposed Changes

### 1. `src/hooks/paths.rs`
- Add path helpers for user-level guidance:
  - `cursor_guidance_path(home: &Path, global: bool) -> PathBuf`
  - `codex_guidance_path(home: &Path, global: bool) -> PathBuf`
  - `windsurf_guidance_path(home: &Path, global: bool) -> PathBuf`
  - `cline_guidance_path(home: &Path, global: bool) -> PathBuf`
  - `antigravity_guidance_path(home: &Path, global: bool) -> PathBuf`
  - `kilocode_guidance_path(home: &Path, global: bool) -> PathBuf`
  - `copilot_settings_path(config_dir: &Path, global: bool) -> PathBuf`

### 2. `src/hooks/installer.rs`
- Update `write_copilot(home, config_dir, global, force, rgt_path)` to separate global and project files.
- Update `write_gemini` and `write_vibe` to use project `.gemini` / `.vibe` directories when `!global`.
- Update `write_cursor`, `write_codex`, `write_windsurf`, `write_cline`, `write_antigravity`, `write_kilocode` to respect `global`.
- Update `detect_installed_agents` to fix Copilot, Codex, Antigravity, and Windsurf triggers.
- Support `force: bool` in `ensure_json_hooks` and `write_cursor`.

### 3. `src/hooks/registration.rs`
- In `inspect_surface`:
  - When multiple matches exist across different scopes (Project and User), sort or select Project scope as primary with `RegistrationHealth::Active`.
  - Only flag `Ambiguous` if multiple conflicting unresolvable registrations exist within the *same* scope.

### 4. Tests
- Add tests in `tests/contract/hook_installer.rs` for global vs project scopes across all agents.
- Add tests in `tests/unit/detection.rs` verifying that `detect_installed_agents` does not trigger falsely on bare `Code/User/settings.json`, bare `AGENTS.md`, or bare `.agents/`.
- Add tests in `tests/contract/test_doctor_contract.rs` verifying that coexisting project and user registrations are `Active` and exit 0.
