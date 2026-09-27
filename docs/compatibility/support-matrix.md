# Harness Compatibility Support Matrix

This matrix distinguishes local registration from completed-read value capture. A settings file or plugin artifact does not prove that a client loaded it. The OpenCode tuple below has a native success and no-value read pair; every other client/version/platform combination remains unverified.

**Baseline**: comparative RTK source and repository inspection in [`specs/044-verify-harness-compatibility/evidence.md`](../../specs/044-verify-harness-compatibility/evidence.md), 2026-09-26. RTK findings are comparative only. See [`evidence/observations.json`](evidence/observations.json) for the native OpenCode observations.

| Canonical agent | Client surface | Local registration tier | Registration health evidence | Stable read-path ID and accepted result form | Exclusions / current capture tier | Client version / platform |
|---|---|---|---|---|---|---|
| `claude-code` | Claude Code | Native command hook configured in `.claude/settings.json` | Not yet inspected in a live client | `claude-read-post-content` — candidate tool `Read`, complete post-tool `tool_response.content`; `claude-bash-read-stdout` — candidate tool `Bash`, recognized read command with completed stdout; both require exact source bytes and lines to match the current file | Before events, missing content, non-read commands, writes/edits, PDF envelopes without a qualifying full-source match, truncated/filtered output: unverified | Unverified / unverified |
| `cursor` | Cursor | Native command hook configured in `.cursor/hooks.json`; project guidance in `.cursor/rules/rgt.mdc` | Not yet inspected in a live client | None established. RGT's current `Shell` callback is not evidence of a native file-read result contract. | Current event and no-op protocol unverified; instruction-assisted recording only | Unverified / unverified |
| `copilot` | Copilot Chat (VS Code) | Native command hook artifact is currently written to user settings; project guidance in `.github/copilot-instructions.md` | Registration location is not confirmed against the host's active hook discovery contract | None established | No automatic value-capture claim; instruction-assisted recording only | Unverified / unverified |
| `copilot` | GitHub Copilot CLI | RGT instruction file only | Local guidance file does not prove CLI loading | None established | No automatic value-capture claim; instruction-only | Unverified / unverified |
| `gemini` | Gemini CLI | Native command hook artifact currently written as TOML; project `GEMINI.md` or user `~/.gemini/GEMINI.md` guidance | Event name/schema is unverified; comparative evidence indicates a before-tool contract | None established for a successful completed read | Before-only or unknown completion cannot record values; instruction-only | Unverified / unverified |
| `vibe` | Mistral Vibe | Native command hook artifact in TOML; project `AGENTS.md` or user `~/.vibe/AGENTS.md` guidance | Event name/schema is unverified; comparative evidence indicates a before-tool contract; guidance loading needs project trust | None established for a successful completed read | Before-only or unknown completion cannot record values; instruction-only | Unverified / unverified |
| `opencode` | OpenCode CLI (`opencode`) | TypeScript project plugin in `.opencode/plugins/rgt.ts`; global plugin path is `~/.config/opencode/plugins/rgt.ts` | Native run loaded the project plugin; `rgt doctor` reported that registration active | `opencode-read-display-text` — `read` tool's `tool.execute.after` result with full, untruncated `metadata.display.text` matching the exact source snapshot | **Verified automatic value capture** for the stated tuple and result form. Before/failed/partial/truncated events, other tools, changed files, and newline-terminated files whose display omits the final newline remain excluded; global plugin loading is unverified. | 1.18.32 / macos/aarch64 |
| `pi` | Pi | TypeScript extension artifact plus project `.pi/APPEND_SYSTEM.md` or user `~/.pi/agent/AGENTS.md` guidance | Extension load and callback contract not verified; project guidance requires trust | None established | No automatic value-capture claim; instruction-only recording guidance | Unverified / unverified |
| `hermes` | Hermes | Python plugin artifact plus project context guidance | Plugin load and callback contract not verified; `--ignore-rules` bypasses guidance | None established | No automatic value-capture claim; instruction-only recording guidance | Unverified / unverified |
| `codex` | Codex CLI | Native hook registration plus guidance, when installed | Hook trust must be reviewed in Codex; file presence does not show the hook ran | `codex-post-tool-response` — candidate tools `read_file`, `ReadFile`, or `Read`; complete, untruncated `tool_response.content` must match the file snapshot | Truncated, partial, non-file, absent, or path-only result: no values; all live behavior unverified | Unverified / unverified |
| `windsurf` | Windsurf Cascade | Native hook registration plus guidance, when installed | Active hook-file precedence and Restricted Mode must be checked in the host | `windsurf-post-read-code-path-only` — documented `post_read_code` with `file_path` only; accepted for invocation evidence, not source values | Path-only payload cannot create values; automatic value capture is instruction-only for this path | Unverified / unverified |
| `cline` | Cline | Instruction guidance | Guidance artifact only | None; no native read path asserted | Instruction-only | Unverified / unverified |
| `cline` | Roo Code | Instruction guidance | Guidance artifact only; Cline loading does not verify Roo | None; no native read path asserted | Instruction-only | Unverified / unverified |
| `antigravity` | Google Antigravity | Instruction guidance | Guidance artifact only | None; no native read path asserted | Instruction-only | Unverified / unverified |
| `kilocode` | Kilo Code | Instruction guidance | Guidance artifact only | None; no native read path asserted | Instruction-only | Unverified / unverified |

## Claim rules

- “Native registration configured” describes an on-disk artifact only. Codex trust, Windsurf hook precedence and workspace mode, and each client's actual load behavior remain separate checks.
- A read-path ID names one client tool and one accepted completed-result form. IDs marked candidate are reserved for documenting that contract; they are not automatic-support claims.
- A path may be marked **verified automatic value capture** only with a native success and a native no-op/error record for that same read-path ID, client version, and platform. The success must include a loaded registration, full-snapshot match, accepted neutral response, unchanged host result, actual value/node IDs, source lines, and capture agent.
- Each verified table row names its canonical agent and a backticked client surface ID in the surface cell, and its read path must belong to that surface. It must format its final cell as `<client-version> / <os>/<architecture>` using the evidence schema's lowercase platform identifiers; separate rows represent separate verified tuples. The paired records must also share one RGT version.
- Client versions and platforms not present in the evidence ledger remain unverified. Fixture replay, documentation, and RTK comparisons never promote a row.
- Capture attribution names the client hook/plugin that delivered an event. It does not claim that client authored the source or calculated a derivation.

## Implementation acceptance audit

The local implementation, registration, capture-gating, schema, replay, version, and latency checks are recorded in [`verification.md`](verification.md). This audit distinguishes local implementation evidence from native client acceptance.

| Requirement | Local result | Status / remaining evidence |
|---|---|---|
| FR-001 | All 13 canonical agents and distinct host surfaces are listed; Codex and Windsurf registrations are implemented. | Implemented locally; client loading remains unverified. |
| FR-002 | A no-cost native OpenCode session supplied a successful `read` and a no-value `read` through the same plugin and tuple. | Met for `opencode-read-display-text` on OpenCode 1.18.32 / macos/aarch64; all other tuples remain unverified. |
| FR-003 | Exact result bytes and source lines are snapshot-checked; the native OpenCode read stored canonical attribution. | Implemented and observed for the stated OpenCode tuple. |
| FR-004 | Before-only, path-only, partial, failed, canceled, and unmappable results are gated from root creation. | Implemented and fixture-tested. |
| FR-005 | Hook failures and malformed events fail open; local tests check neutral behavior. | Implemented locally; the OpenCode callback returned no value and left both native tool results unchanged. Other hosts remain unverified. |
| FR-006 | Native registrations and neutral response adapters are covered by local contract tests. | OpenCode's native callback accepted the project plugin; other clients remain unverified. |
| FR-007 | Empty, malformed, and BOM-prefixed inputs are covered by parser and hook tests. | Implemented and locally tested. |
| FR-008 | Codex/Windsurf registration, ownership-safe migration, backups, and idempotent reruns are covered locally. | Implemented and locally tested; trust and active-host loading remain unverified. |
| FR-009 | Guidance-only surfaces and read paths without completion evidence are identified in the support docs. | Implemented locally. |
| FR-010 | `rgt doctor` reports local registration health without inferring host invocation. | Implemented and locally tested. |
| FR-011 | The versioned evidence schema and validator enforce same-path success/no-op pairing and redacted records. | Native OpenCode pair recorded and validated locally. |
| FR-012 | Recheck commands use local Cargo tests and no hosted workflow or paid session. | Satisfied for this implementation pass. |
| FR-013 | The matrix records read paths, exclusions, one proven tuple, and unverified combinations. | Implemented and updated from the native OpenCode pair. |
| FR-014 | Repeated callbacks reuse values and deduplicate capture associations in contract tests. | Implemented and locally tested. |
| FR-015 | Input, stdin, and child-process bounds plus 64 KiB/1 MiB paths are locally exercised. | Generated-plugin cold and repeated callback measurements passed under one second, including the 800 ms stalled-child case; host scheduling can still affect measurements. |
| FR-016 | Both version commands read the package build version and have contract tests. | Implemented and locally tested. |

| Success criterion | Local result |
|---|---|
| SC-001 | The surface table and local registration checks cover all 13 identifiers; no live claim is inferred. |
| SC-002 | Only `opencode-read-display-text` for OpenCode 1.18.32 / macos/aarch64 is marked verified, backed by the native success and no-value read pair. |
| SC-003 | Synthetic success, empty, malformed, repeated, no-op, and migration cases pass locally for the implemented paths. |
| SC-004 | Doctor contract tests distinguish active RGT registrations from generic settings files and stored values. |
| SC-005 | All recorded checks ran locally without GitHub workflows or paid sessions. |
| SC-006 | The generated plugin's first and repeated callback measurements passed the one-second bound; an earlier subprocess-only runner overrun remains historical context in [`verification.md`](verification.md). |
| SC-007 | Documentation labels unverified and instruction-only surfaces without automatic-capture claims. |
| SC-008 | `rgt --version` and `rgt version` print the package version in passing contract tests. |

**Native acceptance:** The redacted OpenCode pair meets US1's live-acceptance condition for the exact `read` result form and version/platform stated above. The evidence does not extend to other OpenCode versions, other platforms, global plugin loading, or other clients.
