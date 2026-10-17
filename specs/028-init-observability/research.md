# Research: Init Observability

**Feature**: [spec.md](./spec.md) | **Date**: 2026-08-29

Decisions are grounded in the current `src/hooks/installer.rs` (matchers + command hooks), `src/hooks/glue/mod.rs` (plugin artifacts that spawn `rgt`), and `src/cli/status.rs`/`init.rs` (existing CLI shape). No external research needed; the `brew doctor`/`flutter doctor` precedent is documented in findings §2.5.

## 1. `rgt doctor` command shape (FR-001, FR-002)

**Decision**: Add a new `Doctor` subcommand with `src/cli/doctor.rs::execute_doctor()`. It is read-only and returns a result that the CLI maps to exit code 0 (healthy, and warnings) or 1 (errors). Checks, in order:
1. **Store initialized** — open `.rgt/store.db`; on failure this is an **error** ("RGT is not initialized in this project — run `rgt init`") and exits non-zero.
2. **Hooks present** — enumerate the RGT hook artifact paths for the current home/config dir (reusing the installer's agent detection + known paths); count how many exist and contain the RGT marker.
3. **Values recorded** — `list_all_nodes` count; if 0 and hooks are present, emit a **warning** ("capture may not be active — verify `rgt` is resolvable on the PATH of the shell your AI tool uses") and still exit 0.

**Rationale**: Severity tiers (errors → non-zero, warnings → zero) avoid false-positives on fresh installs and match Constitution XI's 0/1/2 exit semantics. Symptom-based PATH detection (hooks present + zero captures) is deterministic and cross-platform, avoiding a fragile `which` implementation.

**Alternatives considered**: overloading `rgt status` with diagnostics — rejected: `status` already evaluates staleness and would conflate two concerns; a dedicated `doctor` matches the established `brew doctor`/`flutter doctor` mental model.

## 2. Absolute-path hook commands (FR-003)

**Decision**: At `rgt init` time, resolve `std::env::current_exe()` to an absolute path and inject it into the command strings for the **eight** subprocess-spawning agents — Claude Code, Cursor, Copilot Chat, Gemini, Vibe (command hooks in `installer.rs`) and OpenCode, Pi, Hermes (plugin artifacts in `glue/mod.rs`). The three plugin constants become `fn` formatters that take the path. The five instructions-only agents (Codex, Windsurf, Cline, Antigravity, Kilo) are untouched — they emit instruction *text*, not subprocess spawns, so an absolute path would be wrong and machine-specific. (Copilot's CLI rules file and Vibe's prompt are instruction text *in addition to* their subprocess hooks and are likewise not path-injected.)

**Rationale**: Prevention at the source; the hook no longer depends on PATH resolution in whatever minimal environment spawns it. `current_exe()` needs no dependency and is the documented way to get the running binary's path.

**Alternatives considered**: keep bare `rgt` and only add `doctor` — rejected: findings §2.5 explicitly recommends both prevention and detection. A placeholder token replaced at write time — rejected in favor of format functions, which are type-checked and clearer.

**Known limitation (documented)**: if the binary is later relocated or a package manager upgrades to a new path, the written absolute path goes stale. `rgt update` replaces the binary in place (same path), but a `brew upgrade` to a new Cellar path requires re-running `rgt init`; this is noted in the README, not auto-healed.

## 3. Claude Code matcher scope (FR-004)

**Decision**: Change `write_claude_code`'s `"matcher": ""` to `"matcher": "Read|Edit|Write|Bash"` for both `PostToolUse` and `PreToolUse`.

**Rationale**: Claude Code's documented matcher syntax treats letters/digits/`|` as exact tool names with `|` as OR. `Read|Edit|Write|Bash` covers everything RGT captures (file reads/writes plus Bash-read capture from 026) while excluding WebFetch/WebSearch/TodoWrite/etc. `Bash` is kept so 026's Bash-read capture keeps working.

**Alternatives considered**: `Read|Edit|Write` only — rejected: it would silently break 026's Bash-read capture. Keeping `""` — rejected: that's the finding being fixed.

## 4. Restart reminder (FR-005)

**Decision**: In `execute_init` (`src/cli/init.rs`), after configuration, print an explicit restart line when `report` contains at least one `AgentOutcome::Configured` (i.e., `configured > 0`); a pure no-op run (all `Skipped`) does not repeat it.

**Rationale**: Hooks are read at session start; the reminder is the in-product reinforcement of the README note (findings §2.8). Gating on "newly configured" avoids nagging on idempotent re-runs.

**Alternatives considered**: always printing — rejected: redundant noise on no-op re-runs.

## 5. Copilot CLI dialect assumption (FR-006)

**Decision**: Add a unit test in `tests/unit/hook_parsers.rs` that pins the current `copilot_dialect` behavior (VS Code Chat snake_case default shape, and CLI `toolArgs`-as-JSON-string + `command_dialect` fallback) with synthetic payloads, and document in the spec/README that the CLI shape is an assumption pending live verification. No change to the dialect logic itself.

**Rationale**: The finding is "unverified assumption," not a proven bug; locking the behavior with a test makes any future shape divergence a deliberate, tested change.

**Alternatives considered**: attempt to capture a real Copilot CLI payload — deferred: requires the real tool; out of scope for this feature.
