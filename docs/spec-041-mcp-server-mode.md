# Spec 041: MCP Server Mode — Expose `record`/`derive`/`verify`/`query`/`status` as Model Context Protocol Tools

**Status:** Proposed — not yet executed, written for the `/speckit-specify` + plan/tasks cycle
**Priority:** P1 — highest single-item adoption leverage identified in `docs/weakpoints-and-growth-opportunities.md`
**Depends on:** none functionally, but should not block or be blocked by specs `030`–`040`/`042`. Complements, and does not replace, spec `023`'s instruction-based non-text-source capture and `033`'s `additionalContext`-based automatic PDF capture — those remain the fallback for agents/scenarios this spec doesn't cover.
**Author's confidence level:** High on protocol/ecosystem facts and the exact refactor scope (verified directly against current `develop` source, not inferred). Medium on the exact `rmcp` integration pattern (SDK verified to exist and its API surface fetched directly, but no working prototype built as part of writing this spec — flagged explicitly in Open Questions).

---

## 1. Problem statement

RGT integrates with 13 AI coding tools via 13 separate, largely bespoke hook/plugin mechanisms (`src/hooks/installer.rs`, `src/hooks/glue/mod.rs`). This project's own prior research has repeatedly found that mechanism to be fragile in ways that are structural, not incidental:

- **§2.7 of `docs/findings-hooks-and-value-extraction.md`**: the Copilot CLI dialect parser is an admitted, untested guess — confirmed still true as of `docs/verification-of-fixes-develop.md` §2 ("an assumption pending live verification").
- **§4**: native PDF reads are invisible to the passive hook entirely, because the tool-response schema for binary/multimodal content is, per Claude Code's own Agent SDK docs, deliberately typed `unknown`/`Any` — i.e., there is no documented contract to parse against, only whatever shape was empirically captured in one specific version at one specific point in time.
- **§8**: Bash-tool reads (`cat`/`head`/`grep`) were completely invisible to Claude Code/Cursor until a dedicated fix, and even that fix depends on recognizing specific wrapper-proxy command shapes (e.g. RTK's `rtk read`) that could change without notice.
- **`docs/spec-033-pdf-capture-via-additionalcontext.md`'s own Open Questions**: whether Cursor supports an equivalent `additionalContext` mechanism to Claude Code is explicitly unverified, and the PDF envelope shape itself is "an undocumented internal implementation detail... could change on any Claude Code update with no warning."

The pattern across all of these: RGT is currently in the business of **reverse-engineering undocumented, per-agent, version-fragile wire formats.** Every fix so far has required a live-capture-and-verify cycle per agent, and several gaps remain open specifically because that verification hasn't been done yet for every agent.

**MCP (Model Context Protocol) sidesteps this problem category entirely by construction.** Per the protocol's own site ([modelcontextprotocol.io](https://modelcontextprotocol.io/), fetched directly for this spec): "MCP is an open-source standard for connecting AI applications to external systems... MCP is an open protocol supported across a wide range of clients and servers" — explicitly naming Claude, ChatGPT, Visual Studio Code, and **Cursor** ([cursor.com/docs/context/mcp](https://cursor.com/docs/context/mcp), linked directly from the official MCP site) as clients that already speak this protocol today. An MCP tool has a documented JSON schema the client is contractually bound to honor — there is no "undocumented internal detail that could change on any update" category of risk for the wire format itself, because the wire format *is* the documented public contract, not a side effect of the agent's own implementation.

---

## 2. The opportunity, with supporting evidence

- **Adoption scale and trajectory.** Per research already gathered in `docs/weakpoints-and-growth-opportunities.md` §4 (secondary sources, not independently re-verified for this spec — flagged as such): 10,000+ public MCP servers by Dec 2025, adoption by OpenAI/Google DeepMind/Microsoft, and donation of protocol governance to the Linux Foundation's Agentic AI Foundation in Dec 2025 ([WorkOS](https://workos.com/blog/everything-your-team-needs-to-know-about-mcp-in-2026), [ChatForest](https://chatforest.com/guides/mcp-ecosystem-2026-state-of-the-standard/)). This spec's own direct fetch of the primary site independently confirms the "open-source standard, broad client support including Cursor and VS Code" claim, though not the specific Linux Foundation governance detail — that detail should be treated as secondary-sourced until independently confirmed, consistent with this project's own established citation standards.
- **Established prior art for "CLI ships an MCP server mode alongside its native interface."** Codex CLI itself ships this way, per third-party documentation of the pattern ([Codex Knowledge Base](https://codex.danielvaughan.com/2026/03/30/codex-cli-as-mcp-server/)) — RGT would not be inventing a novel integration shape, it would be following an already-normalized one.
- **A real, official Rust SDK exists** — not a third-party reimplementation. Confirmed directly via [docs.rs/rmcp](https://docs.rs/rmcp/latest/rmcp/), fetched for this spec: crate `rmcp`, current version `3.1.4`, with `server` and `macros` as default features, plus opt-in transport features `transport-io` (stdio) and `transport-streamable-http-server` (HTTP). Tools are defined via a `#[tool]` attribute macro on an `impl` block. This directly answers the feasibility question the earlier growth-research pass flagged as needing verification ("check if a real one exists rather than assuming") — it does.
- **Cursor support specifically resolves an open question from spec `033`.** Rather than needing to verify whether Cursor's *hooks* support an `additionalContext`-equivalent mechanism (spec `033`'s Open Question #1), MCP support is confirmed directly from Cursor's own docs, linked from the protocol's own site. This is a materially easier bar to clear than per-agent hook-schema verification.

---

## 3. Proposed solution: an `rgt mcp` (or `rgt serve`) subcommand exposing 5 tools

### 3.1 Tool schema, mapped to exact existing functions

| MCP tool | Wraps | Current signature | Refactor needed? |
|---|---|---|---|
| `query_provenance` | `query::query_provenance` (`src/query/mod.rs:10-69`) | `fn(node_id: &str) -> Result<serde_json::Value, String>` | **None.** Already a pure, clean, JSON-returning function. Directly callable as-is. |
| `get_status` | `cli::status::execute_status_with` (`src/cli/status.rs:26-92`) | `Result<(), String>`, builds a `json!(...)` value internally (lines 57-62) but only ever `println!`s it (line 64-67) rather than returning it | **Small.** Extract the existing `json!({...})` construction into its own return value; keep the CLI's `println!` wrapper as a one-line caller of the new pure function. |
| `record_values` | `cli::record::execute_record` (`src/cli/record.rs:42-148`) | `Result<(), String>`, no `process::exit` calls (good), but reports outcome via `println!`/`eprintln!` (lines 130-145) rather than a return value | **Small.** Introduce a `RecordOutcome { count, skipped_limit, skipped_malformed }` struct (or a `serde_json::Value`), return it instead of printing; CLI wrapper prints from the returned struct. |
| `derive_and_verify` | `cli::derive::execute_derive` (`src/cli/derive.rs:8-134`) | `Result<(), String>` **on paper**, but calls `process::exit(2)` directly at 8 call sites (lines 22, 28, 32, 44, 52, 65, 70, 82) for every validation failure (unknown operation, missing expression, empty/too-many parents, missing/unloadable parent node, DATE_DIFF arity) | **Required, not optional.** Calling this function today from any long-lived process (an MCP server included) would **terminate the entire server** on the very first invalid input it receives — this is not a style nitpick, it is a correctness blocker for this feature. Every `process::exit(2)` call site must become a `return Err(...)` instead; the CLI entry point (`main.rs:235-240`, currently just forwards the `Result`) is unaffected since it already just propagates `Err` to its own generic `eprintln!`+`exit(1)` handler — but a distinct exit-code-2-vs-1 distinction currently only exists via the direct `process::exit(2)` calls, so preserving CLI exit-code behavior means either introducing a typed error (see §3.3) or having `main.rs` inspect the error to decide the code. |
| `verify_only` | `verify::run_verify_cli` (`src/verify/mod.rs:55-129`) | Returns `()`, not `Result` at all — handles everything via `process::exit(0/1/2)` internally (7 call sites: lines 64, 70, 74, 86, 93, 104, 108, 115, 126) | **Required, not optional**, same reasoning as `execute_derive`. This function has no `Result`-returning form to fall back on at all today — it must be split into a pure `run_verify(...) -> Result<VerifyOutcome, VerifyError>` core, with the existing `run_verify_cli` becoming a thin wrapper that calls it and translates to `process::exit`. |
| (not exposed) | `cli::graph::execute_graph` (`src/cli/graph.rs:6-`) | `Result<(), String>`, builds output via direct `println!` per line rather than a string buffer (confirmed for the `mermaid`/`dot` branches, lines 13-45+) | Deliberately **not included** in the initial tool set — see Non-Goals §5. Larger refactor (buffer-then-return vs. line-by-line print) for a lower-value tool (an agent asking an MCP tool for a Mermaid diagram is a weaker use case than asking it to record/verify/query values). Candidate for a follow-up once the core 5 are shipped. |

### 3.2 Minimal `rmcp` integration shape (illustrative, verify exact macro ergonomics during implementation)

```rust
use rmcp::{tool, ServerHandler};

struct RgtMcpServer;

#[tool]
impl RgtMcpServer {
    /// Query the full provenance lineage for a tracked value.
    async fn query_provenance(&self, node_id: String) -> Result<serde_json::Value, String> {
        crate::query::query_provenance(&node_id)
    }

    /// Record numeric/date values extracted from file content into the provenance graph.
    async fn record_values(&self, file: String, content: Option<String>, number_format: Option<String>) -> Result<serde_json::Value, String> {
        // maps to a refactored execute_record returning structured output, per §3.1
        todo!()
    }

    // verify_only, derive_and_verify, get_status follow the same shape
}
```

Transport: start with `transport-io` (stdio) only — this matches how every one of RGT's 13 existing integrations already invokes it (as a subprocess via a hook/plugin command), so no new network-facing surface is introduced. HTTP transport (`transport-streamable-http-server`) is explicitly out of scope for the initial version (see Non-Goals).

### 3.3 Error typing (needed regardless of MCP, surfaced by this work)

To cleanly separate "exit code 2 (bad input)" from "exit code 1 (verification failed)" without `process::exit` calls scattered through the logic, introduce a small typed error, e.g.:

```rust
pub enum VerifyDeriveError {
    InvalidInput(String),   // maps to exit 2 in the CLI wrapper
    VerificationFailed(String), // maps to exit 1
    Internal(String),       // maps to exit 1 or 2 depending on context
}
```

This is a genuinely worthwhile cleanup independent of MCP — it directly fixes the "function signature looks clean but actually exits the process" trap identified in §3.1 above, which would be worth flagging even if MCP mode were never built.

---

## 4. Interaction with the existing hook-based capture

This is **additive, not a replacement.** The 13 existing hook/plugin integrations keep working exactly as they do today. MCP mode is a new, 14th integration surface, valuable specifically because:

- It requires the calling agent to *choose* to call an RGT tool explicitly — which is a strength for `verify_derivation`/`derive` (where explicit, intentional invocation is exactly the desired behavior: an agent should have to actively ask "check my math" rather than have arbitrary text pattern-matched for something that looks like a derivation) and a complementary path for `record_values` (an agent that already knows about the tool no longer needs a working hook payload guess to get captured).
- For **any MCP-compatible client**, no per-agent glue code is needed at all — one `rgt mcp` invocation configuration line, wherever that client's MCP config lives (e.g. `.cursor/mcp.json`, `claude_desktop_config.json`-equivalent for Claude Code, VS Code's `mcp.json`), replaces N different hook/rules-file installers for that specific client.
- Passive hook capture remains the right default for "an agent reads a file and might not think to call a tool" — MCP mode is the right complement for "an agent wants to intentionally verify a calculation or explicitly report a value," which is arguably the *more* valuable half of RGT's value proposition (mathematical re-verification) and the harder half to get right via passive observation alone.

---

## 5. Non-goals / explicitly out of scope for this spec

- **HTTP/SSE transport.** Stdio only for the initial version — matches the existing subprocess-invocation model every hook integration already uses, avoids introducing a new network-facing attack surface, and is the lower-effort transport per the `rmcp` feature flags confirmed in §2.
- **Exposing `rgt graph`/`rgt gc`/`rgt doctor`/`rgt init`/`rgt update` as MCP tools.** These are operator/maintenance actions, not something an agent should be invoking mid-task; `graph` specifically also needs a larger refactor (see §3.1) for uncertain value. Revisit only if concrete demand emerges.
- **Replacing or deprecating any of the 13 existing hook/plugin integrations.** Not proposed, not implied. They keep working.
- **A rewrite of the CLI argument-parsing layer.** `main.rs`'s `clap`-based dispatch is untouched; only the *internals* of `execute_derive`/`run_verify_cli` change (process::exit → typed Result), which is invisible to the CLI's own behavior as long as `main.rs`'s wrapper correctly re-translates the new error type back to the same exit codes.

---

## 6. Risks and mitigations

| Risk | Mitigation |
|---|---|
| **`execute_derive`/`run_verify_cli` refactor could subtly change existing CLI exit-code behavior** (e.g. if a `process::exit(2)` call site is missed or mistranslated to exit 1). | This is exactly the kind of regression this project's own existing contract tests (`tests/contract/test_cli_record_derive_contract.rs`) are built to catch — run the full existing suite, and add explicit exit-code assertions for every one of the 8 (`derive`) + 7 (`verify`) call sites enumerated in §3.1 before considering the refactor complete, not just a general "does it still work" smoke test. |
| **`rmcp`'s exact macro ergonomics weren't validated with a working prototype for this spec** — the code in §3.2 is illustrative based on the documented pattern, not a compiled example. | Build a minimal one-tool prototype (`query_provenance` alone, since it needs zero refactoring) as the very first implementation step, before touching `derive`/`verify`'s refactor — this validates the SDK integration cheaply and de-risks the rest of the work. |
| **Version drift**: `rmcp` is a fast-moving SDK (confirmed at v3.1.4 as of this writing) tracking an evolving protocol spec. | Pin an exact version in `Cargo.toml` (matching this project's existing practice — e.g. `attestation-verify` and `self_replace` are similarly pinned per `docs/verification-of-fixes-develop.md`), and treat SDK upgrades as their own reviewed change, not an incidental `cargo update`. |
| **Binary size / dependency footprint**, given the project's "single lightweight binary" positioning (already a stated concern for the web-dashboard feature that was explicitly declined in `docs/weakpoints-and-growth-opportunities.md` §5). | `rmcp` with only `server`+`macros`+`transport-io` (no HTTP transport, no client-side features) should be materially lighter than an HTTP server + JS graph renderer would have been; measure the actual binary size delta as part of the implementation and report it, rather than assuming it's negligible. |

---

## 7. Open questions requiring further work before/during implementation

1. **Exact `#[tool]` macro ergonomics** — validate against a real, compiling prototype (see Risks table); the code shown in §3.2 is representative, not verified to compile as written.
2. **Where does an MCP server's working directory/project-root resolution come from?** `DbStore::open_in_project(".")` is used unmodified everywhere in the current codebase, assuming the process's cwd is the project root — this holds for the CLI (invoked directly in a project directory) and should also hold for an MCP server subprocess (an MCP client typically launches the server with the project as cwd, the same assumption every one of the 13 existing hook/plugin integrations already makes), but this should be explicitly confirmed against how Claude Code/Cursor actually launch a configured MCP server, not assumed by analogy.
3. **Should `derive_and_verify` be one combined tool or two separate tools** (`verify_only` vs. `derive` which verifies-then-records)? The CLI already exposes these as two separate commands (`rgt verify` vs `rgt derive`) with meaningfully different side effects (one writes to the graph, one doesn't) — the MCP tool surface should probably preserve that distinction rather than collapsing it, but this is a naming/ergonomics decision worth confirming during `/speckit-specify` rather than deciding unilaterally here.

---

## 8. Success criteria / acceptance tests

- A working `rgt mcp` subcommand, stdio transport, exposing at minimum `query_provenance` (zero-refactor tool, ships first per the Risks-table de-risking plan).
- All 8 + 7 `process::exit` call sites in `execute_derive`/`run_verify_cli` converted to typed `Err` returns, with existing CLI-facing exit-code behavior preserved and explicitly asserted in tests (not just "still passes the existing suite," which doesn't specifically target exit codes today).
- A real MCP client (Claude Code and/or Cursor, per their documented MCP support) successfully configured against a locally-built `rgt mcp`, demonstrated calling at least `query_provenance` and `record_values` end-to-end.
- Binary size delta from adding `rmcp` measured and reported, not assumed.

---

## 9. References

- Model Context Protocol, official site (fetched directly for this spec): [modelcontextprotocol.io](https://modelcontextprotocol.io/)
- Cursor's own MCP documentation (linked directly from the official MCP site, confirming Cursor as an MCP client): [cursor.com/docs/context/mcp](https://cursor.com/docs/context/mcp)
- `rmcp` crate documentation (fetched directly for this spec — crate name, version 3.1.4, feature flags, `#[tool]` macro pattern, transport options): [docs.rs/rmcp/latest/rmcp](https://docs.rs/rmcp/latest/rmcp/)
- MCP ecosystem adoption figures and Linux Foundation governance claim (secondary sources, not independently re-verified for this spec — carried over from `docs/weakpoints-and-growth-opportunities.md`): [WorkOS](https://workos.com/blog/everything-your-team-needs-to-know-about-mcp-in-2026), [ChatForest](https://chatforest.com/guides/mcp-ecosystem-2026-state-of-the-standard/)
- Prior art for "CLI ships an MCP server mode alongside its native CLI" (Codex CLI): [Codex Knowledge Base](https://codex.danielvaughan.com/2026/03/30/codex-cli-as-mcp-server/)
- This project's own prior findings that motivate this spec: `docs/findings-hooks-and-value-extraction.md` §2.7, §4, §6, §8; `docs/verification-of-fixes-develop.md` §2 (confirms §2.7 is still open); `docs/spec-033-pdf-capture-via-additionalcontext.md` §6 (Open Questions this spec sidesteps for Cursor specifically); `docs/weakpoints-and-growth-opportunities.md` §4-§5 (original recommendation and feasibility pass this spec formalizes).
- Current source read directly for this spec's refactor-scope claims: `src/main.rs`, `src/cli/mod.rs`, `src/cli/{record,derive,graph,status}.rs`, `src/query/mod.rs`, `src/verify/mod.rs`.
