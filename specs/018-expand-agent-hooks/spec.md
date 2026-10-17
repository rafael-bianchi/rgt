# Feature Specification: Expand Agent Hook Coverage to Match RTK

**Feature Branch**: `feat/018-expand-agent-hooks`

**Created**: 2026-08-15

**Status**: Draft

**Input**: User description: "we need to, at least, match the number of agents that the rtk covers https://github.com/rtk-ai/rtk/tree/develop/hooks"

## Clarifications

### Session 2026-08-15

- Q: What coverage target for agent integrations? → A: Full parity — integrate with all 13 RTK-covered agents (Option A), each using its native mechanism.
- Q: Hook mechanism for plugin-based agents (OpenCode, Pi, Hermes)? → A: Ship the same runtime plugins RTK uses (thin TS for OpenCode/Pi, Python for Hermes) that only call the RGT CLI — real per-tool-call capture, zero business logic (Option A).
- Q: Scope boundary — command rewriting? → A: Provenance capture only — RGT hooks record data and never rewrite or filter the agent's tool commands (Option A); RTK is the reference only for agent set and hook mechanisms.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Every RTK-Supported Agent Can Initialize RGT (Priority: P1)

A developer uses an AI coding tool that RTK supports (e.g., Copilot, OpenCode, Kilo, Cline, or Mistral Vibe). After installing RGT, they run `rgt init --agent <their-agent>` and RGT configures provenance tracking hooks for that specific agent, mirroring the coverage RTK provides. Users of any of the 13 RTK-supported agents must be able to onboard RGT with a single targeted command.

**Why this priority**: Feature parity with RTK is the core request. RTK currently covers 13 agents (Claude Code, Copilot, Cursor, Gemini CLI, Cline/Roo Code, Windsurf, Codex CLI, OpenCode, Pi, Hermes, Mistral Vibe, Antigravity, Kilo); RGT covers only 4 (Claude Code, Cursor, Codex CLI, Windsurf). Missing agent support blocks RGT adoption for users of the other tools and is the direct gap this feature closes.

**Independent Test**: Run `rgt init --agent <name>` for each of the 13 supported agent names; each run must succeed, write the agent's hook configuration, and be re-runnable idempotently.

**Acceptance Scenarios**:

1. **Given** a developer using any of the 13 RTK-supported agents, **When** they run `rgt init --agent <that agent>`, **Then** RGT configures provenance-tracking hooks for that agent and confirms success.
2. **Given** an agent name not in the supported set, **When** `rgt init --agent <unknown>` runs, **Then** RGT exits with code 2 and a message listing the valid agent names.
3. **Given** an already-configured agent, **When** `rgt init --agent <that agent>` runs again, **Then** RGT makes no duplicate or destructive change unless `--force` is passed.

---

### User Story 2 - One-Command Setup Configures Every Installed Agent (Priority: P1)

A developer installs RGT on a machine where several AI coding tools are installed (e.g., Claude Code, Cursor, and OpenCode). They run `rgt init` with no agent argument. RGT detects every supported agent present on the machine and configures hooks for all of them in one run, so the developer gets full provenance coverage without enumerating tools.

**Why this priority**: The existing `rgt init` (no `--agent`) already auto-detects and configures multiple agents. Extending detection to the full RTK set keeps the frictionless one-command experience while achieving coverage parity. This is part of the same parity goal as US1.

**Independent Test**: Install/emulate two or more supported agents (via their config-file locations), run `rgt init`, and confirm each detected agent's configuration is written in a single invocation.

**Acceptance Scenarios**:

1. **Given** multiple supported agents installed, **When** `rgt init` runs without `--agent`, **Then** RGT configures every detected supported agent and lists each in its output.
2. **Given** no supported agent installed, **When** `rgt init` runs, **Then** RGT still initializes the database and reports that no hook configs were written (no error).
3. **Given** an agent's config directory exists but is empty/unconfigured, **When** `rgt init` runs, **Then** that agent is detected and configured.

---

### User Story 3 - New Hooks Never Block the Agent (Priority: P2)

A developer's agent invokes an RGT hook during a tool call. Even if the RGT binary is missing, input is malformed, or the agent's runtime for the hook is unavailable, the developer's tool execution must proceed exactly as if RGT were absent. Provenance capture is best-effort background behavior.

**Why this priority**: RGT hooks run inside the agent's tool-call lifecycle. A hook that blocks or errors would degrade the very developer experience RGT exists to instrument. The constitution mandates fail-open, non-blocking hooks, and RTK's hooks follow the same exit-code contract.

**Independent Test**: Unit-test every hook error path (missing binary, malformed JSON, unsupported runtime) and assert each exits 0 with no output and never modifies the command.

**Acceptance Scenarios**:

1. **Given** a hook runs and the RGT binary is not on PATH, **When** the agent invokes it, **Then** the hook exits 0 with no output and the tool call proceeds.
2. **Given** a hook receives malformed input, **When** the agent invokes it, **Then** the hook passes through unchanged and exits 0.
3. **Given** a plugin-based agent hook requires a runtime that is absent, **When** the agent invokes it, **Then** execution is a no-op with no error surfaced to the agent.

---

### User Story 4 - Documentation Lists All Supported Agents (Priority: P3)

A developer reads RGT's documentation (help text, README, AGENTS.md) to decide whether their agent is supported. The documented list of supported agents matches the actual `--agent` accept list, including aliases (e.g., `claude` / `claude-code`).

**Why this priority**: Documentation drift is a known failure mode (constitution Principle X). Users and agents rely on the documented contract; an undocumented agent silently fails onboarding, and an over-documented one breaks the user's trust.

**Independent Test**: Diff the `--agent` accepted names against the documented supported-agent list; they must match exactly.

**Acceptance Scenarios**:

1. **Given** RGT documentation, **When** a user looks up supported agents, **Then** the list includes all 13 RTK-covered agents with their accepted names and aliases.
2. **Given** a documented agent name, **When** passed to `rgt init --agent`, **Then** it is accepted by the CLI.

---

### Edge Cases

- **Existing user config**: Agents' config files (settings, rules, plugin dirs) may already contain user-authored content; RGT must merge or append without clobbering unless `--force`.
- **Agent aliases**: `claude` and `claude-code`, and `cline`/`roo-code` naming variants must resolve to the same integration.
- **Mechanism differences**: Some agents expose full tool-hook APIs, others only accept rules/instructions files; each agent's integration must use the mechanism that agent actually supports.
- **Missing runtimes**: Plugin/extension-based agents may require runtimes (e.g., Node, Python); absence must degrade to a no-op, not an error.
- **Cross-platform paths**: Agent config locations differ across macOS, Linux, and Windows; detection and writes must respect the host platform.
- **Idempotent re-init**: Re-running `rgt init` must not duplicate hook entries or corrupt existing config.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: `rgt init --agent <name>` MUST accept every agent name in the RTK-supported set (13 agents: Claude Code, Copilot, Cursor, Gemini CLI, Cline/Roo Code, Windsurf, Codex CLI, OpenCode, Pi, Hermes, Mistral Vibe, Antigravity, Kilo), and MUST reject unknown names with exit code 2 and a message listing valid names.
- **FR-002**: For each supported agent, `rgt init --agent <name>` MUST write the agent-appropriate hook configuration so provenance capture activates automatically for that agent's tool calls.
- **FR-003**: `rgt init` without `--agent` MUST auto-detect every supported agent installed on the machine and configure all of them in a single run.
- **FR-004**: Existing integrations (Claude Code, Cursor, Codex CLI, Windsurf) MUST continue to work unchanged; this feature MUST NOT regress their current behavior or config output.
- **FR-005**: All hooks (existing and new) MUST fail open: missing binary, malformed input, or unavailable runtime MUST result in no-op behavior that never blocks or alters the agent's tool execution.
- **FR-006**: Per-agent and auto-detected configuration MUST be idempotent and MUST NOT overwrite existing user-authored config content unless `--force` is passed.
- **FR-007**: All detection, validation, and provenance logic MUST live in the RGT binary; any per-agent glue (scripts, plugins, rule snippets) MUST be a thin delegate with no business logic. For agents whose RTK integration uses a runtime plugin (OpenCode, Pi, Hermes), RGT MUST ship equivalent thin plugins (Node/TypeScript, Python) that only invoke RGT CLI commands — confirmed during clarification (2026-08-15).
- **FR-008**: Each new agent integration MUST include test coverage demonstrating intended usage, edge cases, and error paths.

### Key Entities

- **Agent**: A supported AI coding tool (name, accepted aliases, integration mechanism — full hook, plugin/extension, or rules/instructions file, config artifact location).
- **Hook Configuration**: The agent-specific artifact (settings entry, rules file, plugin, extension) that causes the agent to invoke RGT on tool calls.
- **Detection Result**: The set of agents found installed on the host during `rgt init` auto-detection, each mapped to its hook configuration target.
- **Capture Event**: The normalized `{path, content}` model produced by the per-agent normalizer and consumed by the capture path (see data-model.md).

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: 100% of the 13 RTK-supported agent names are accepted by `rgt init --agent` (verified by running each).
- **SC-002**: A single `rgt init` run configures all supported agents detected on a machine (verified with 2+ agents installed).
- **SC-003**: Existing Claude Code, Cursor, Codex, and Windsurf configs are byte-identical before and after this change (no regression).
- **SC-004**: 100% of hook failure paths (missing binary, malformed input, absent runtime) result in non-blocking no-op behavior, verified by automated tests.
- **SC-005**: The documented supported-agent list and the `--agent` accepted names match exactly (no drift).

## Assumptions

- The coverage target is full parity with all 13 RTK-supported agents: the set of agents represented by RTK's `hooks/` directory (12 folders) plus the Gemini CLI integration documented in RTK's hooks README. This was confirmed during clarification (2026-08-15); RGT MUST integrate with the complete set, not a subset.
- Per-agent integration mechanisms mirror RTK's tiering: event-capture via native hook API (Claude Code, Copilot, Cursor, Gemini, Mistral Vibe) or thin runtime plugin (OpenCode, Pi, Hermes — TS/Python), versus rules/instructions files (Cline/Roo Code, Windsurf, Codex CLI, Antigravity, Kilo).
- `claude` is an accepted alias for `claude-code`, and `roo-code` is an accepted alias for `cline`.
- `vibe` is the canonical `--agent` value for Mistral Vibe (no `mistral`/`mistral-vibe` alias); documentation and the 13-agent list MUST use `vibe`.
- Auto-detection keys off each agent's standard config-file location on the host platform.
- The four existing integrations' current behavior is the correctness baseline; any change to their output is a regression.
- Command rewriting/filtering (RTK's `rtk rewrite` behavior) is explicitly OUT OF SCOPE: this feature only adds provenance-capture hooks that record data without modifying the agent's tool commands — confirmed during clarification (2026-08-15).
