# Spec 033: Automatic PDF (and future binary-format) Provenance Capture via `PostToolUse` `additionalContext` Injection

**Status:** Proposed — not yet executed, written for the `/speckit-specify` + plan/tasks cycle
**Priority:** P1 — closes a confirmed total gap (§4 of `docs/findings-hooks-and-value-extraction.md`) with a mechanism strictly better than the one already shipped in spec `023-non-text-source-capture`
**Depends on:** none (independent; can ship alongside or after spec `023`, and supersedes its Claude Code-specific behavior without removing the existing fallback)
**Author's confidence level:** High on the core mechanism (empirically verified twice, live, in-session — not inferred from docs). Medium-low on generalizing beyond Claude Code/PDF without further verification (explicitly scoped below).

---

## 1. Problem statement

`docs/findings-hooks-and-value-extraction.md` §4 established, with a live-captured real hook payload, that RGT's passive `PostToolUse` hook (`rgt hook post`) never receives extracted text for a native `Read` on a PDF. The captured `tool_response` was:

```json
{
  "type": "pdf",
  "file": {
    "filePath": "/path/to/test_invoice.pdf",
    "base64": "<raw PDF bytes, base64>",
    "originalSize": 14479
  }
}
```

No `content` string field exists. `HookToolResponse.content` (`src/hooks/parser.rs:12-15`) deserializes to `None`, `handle_passive_hook_event` (`src/hooks/mod.rs:40-46`) falls back to `fs::read_to_string` on the original PDF path, fails on invalid UTF-8, and the event returns `Ok(())` via the fail-open path — **zero values recorded, zero signal to the user or the agent.**

This was confirmed against official documentation: PDF interpretation happens **server-side, inside the model**, never locally on the CLI/hook side. Per Anthropic's PDF support docs ([platform.claude.com/docs/en/build-with-claude/pdf-support](https://platform.claude.com/docs/en/build-with-claude/pdf-support)): "PDF support relies on Claude's vision capabilities," with each page rendered as an image and interpreted by the model over the API. **No plain-text transcript of the PDF ever exists anywhere in the local pipeline** — the text a user sees in the model's own turn is synthesized fresh per request and never written to a file or object a hook can reach.

**The fix already shipped (spec `023`)** addresses this by instructing the agent, via `AGENTS.md`/rules files, to explicitly write extracted values to a text file or pipe them via `rgt record --stdin` after reading a PDF. This works, but it reintroduces exactly the failure mode RGT's entire passive-hook design exists to eliminate: **it depends on the agent remembering to do something extra.** If the agent reads a PDF and just answers the user's question directly, without a follow-up `rgt record --stdin` call, nothing is captured — silently, with no signal, exactly as before.

This spec proposes a mechanism that closes the gap **without requiring any agent cooperation at all.**

---

## 2. The mechanism: `PostToolUse` → `additionalContext`

### 2.1 What was verified, and how (not inferred — reproduced live, twice)

Claude Code's hooks reference documents a `hookSpecificOutput.additionalContext` field on `PostToolUse` hook output. Verbatim, from the raw markdown source ([code.claude.com/docs/en/hooks.md](https://code.claude.com/docs/en/hooks.md)), fetched and quoted directly in this session:

> `additionalContext` — "Text Claude sees as a system message, so it can react. Not shown in the transcript to the user" — applies to `PreToolUse`, `UserPromptSubmit`, `UserPromptExpansion`, `PostToolUse`, `PostToolUseFailure`, `Stop`, `SubagentStop`, `SessionStart`, `PostModelSwitch`.

And specifically for `PostToolUse`'s own section:

> `PostToolUse` supports these `hookSpecificOutput` fields: `additionalContext` (string) — "Text Claude sees as a system message for evaluating the tool output." ... Hook handlers run in parallel and can't block the tool call, which already succeeded, but **can halt the conversation or modify what Claude sees next.**

This was **not taken on faith**. It was reproduced live, twice, in this session, using the exact same "capture a real payload, don't guess" methodology already established for the other hook findings in this project:

1. A temporary, project-scoped debug hook (`.claude/settings.local.json`, matcher `"Read"`, later removed) was configured to always print `{"hookSpecificOutput": {"hookEventName": "PostToolUse", "additionalContext": "RGT_TEST_MARKER_7f3a9: ... The secret magic word is PINEAPPLE-42."}}` to stdout on every `PostToolUse:Read` event.
2. After the session restart required for hook config to load (confirmed necessary and sufficient in the same manner documented in §4's own postmortem), a plain `Read` of an unrelated file was performed.
3. **The exact injected marker text appeared as a system-reminder** immediately after the tool result, addressed to the model, reading: `"PostToolUse:Read hook additional context: RGT_TEST_MARKER_7f3a9: this is injected via PostToolUse additionalContext, not real file content. The secret magic word is PINEAPPLE-42."`

This is unambiguous, first-party proof: a `PostToolUse` hook's `additionalContext` output is injected into the model's context automatically, on every matching tool call, requiring zero agent action, zero prompt/instruction changes, and zero cooperation from whatever code the agent is currently executing.

### 2.2 Why this is a categorically better fix than spec `023`'s instruction-based approach

| | Spec `023` (shipped) | Spec `033` (this proposal) |
|---|---|---|
| Requires agent to remember an extra step | **Yes** — agent must explicitly call `rgt record --stdin` or write a file | **No** — fires automatically on every matching tool call |
| Silent-failure mode if agent forgets | Same as before the fix (zero capture, zero signal) | N/A — nothing to forget |
| Gives the agent better information | No — agent still relies on its own vision pass for the PDF | **Yes** — agent also receives the actual extracted text, which can be more accurate than a vision-based read for dense tabular/numeric content |
| Consistent with RGT's own stated design principle ("hooks record data only, never rewrite/filter/block tool calls") | Yes | **Yes** — `additionalContext` is purely additive information the model can choose to react to; it does not modify, filter, or block the underlying `Read` tool call or its primary result |

This does not replace spec `023` — the instruction-based fallback should remain in place for agents/scenarios where this mechanism is unverified or unavailable (see §5, Open Questions). This spec is additive: for Claude Code specifically, it upgrades an "agent must remember" fix to a "just works" fix.

---

## 3. Detailed design

### 3.1 New capture flow in `rgt hook post`

Current flow (`src/hooks/mod.rs:22-91`, `handle_passive_hook_event`): parse stdin → normalize via dialect → if `content` present, extract values; else fall back to disk read; on any failure, return `Ok(())` silently. **No code path ever writes to stdout today.**

New flow, additive to the existing one:

1. In `normalize_agent_event`/`default_dialect` (`src/hooks/parser.rs:111-122`), when `tool_response.content` is absent, additionally check for the structured binary envelope shape already confirmed in §4: `tool_response.type == "pdf"` with a `file.base64` field. (Do **not** trust the docs' example input field name `tool_output` seen elsewhere in the hooks reference — the empirically captured real payload for this project used `tool_response`, confirming a real doc/reality mismatch that should be resolved by trusting the captured payload, not the doc example.)
2. If matched, base64-decode `file.base64` (see §3.2 for the exact crate/API), then run local PDF text extraction (see §3.3).
3. If extraction yields non-empty text: run it through the existing `extract_values_from_content` (`src/hooks/parser.rs`) exactly as any other captured content, recording nodes into the graph via the existing `insert_tracked_node` path (`src/hooks/mod.rs:72-88`) — **this is the part that fully closes the original §4 gap, automatically, regardless of whether the code below it (stdout emission) succeeds or is even supported by the calling agent.**
4. Additionally (not instead), print a `hookSpecificOutput.additionalContext` JSON object to stdout containing the extracted text (subject to the truncation policy in §3.4), so Claude Code also receives accurate text instead of relying solely on its own vision pass.
5. If extraction yields empty text (e.g., a scanned/image-only PDF with no text layer — `pdf-extract` returns an empty string rather than an error for this case, per the same finding already documented in §3.1/§4 of the findings report), still emit an `additionalContext` noting this explicitly (e.g. `"RGT: no text layer found in this PDF; provenance capture requires the agent to extract values manually via rgt record --stdin"`) rather than the current silent, indistinguishable-from-success failure. This directly implements the "remove the current silent failure" recommendation already in §4's fix list, item 3.
6. All of the above must fail open on any error (malformed base64, extraction panic/error, etc.) — see §4 Risks for the specific hardening this requires.

### 3.2 Base64 decoding

Add the `base64` crate (v0.22+; confirmed current API via [docs.rs/base64](https://docs.rs/base64/latest/base64/), fetched in this session — the crate is at 0.23.1 as of this writing, and the standalone `base64::decode()` function is deprecated). Correct current usage:

```rust
use base64::prelude::*;

let decoded: Vec<u8> = BASE64_STANDARD.decode(&file.base64)
    .map_err(|_| /* fail open, do not propagate a hard error */)?;
```

### 3.3 Local PDF text extraction

Add the `pdf-extract` crate ([docs.rs/pdf-extract](https://docs.rs/pdf-extract)), already vetted in `docs/weakpoints-and-growth-opportunities.md` and `docs/findings-hooks-and-value-extraction.md` §3.1/§4 as the correct crate for this. Confirmed API surface (fetched in this session): `extract_text(path: impl AsRef<Path>)` and `extract_text_by_pages(path, ...)`, both documented as taking a **file path**, not an in-memory byte buffer — meaning the decoded base64 bytes will need to be written to a temporary file (`tempfile` crate, already a dependency per `Cargo.toml`) before calling `extract_text`, rather than processed purely in memory. Confirm during implementation whether an in-memory variant exists (e.g. operating on a `Cursor<&[u8]>` or `Read` type) that would avoid the temp-file round-trip; if not, a `tempfile::NamedTempFile` is the correct, already-idiomatic-in-this-codebase pattern (used elsewhere for the updater's download-then-verify flow).

**Known caveat, already documented:** `pdf-extract` only recovers a PDF's existing text layer. Scanned/image-only PDFs yield an empty string, not an error — handled explicitly per §3.1 step 5 above, not a silent gap.

### 3.4 Token-cost mitigation (new consideration, not previously addressed)

`additionalContext` becomes part of the model's context window on every matching tool call. For a large, text-dense PDF, injecting the full extracted text unbounded could be a meaningful, unexpected token cost — a real tradeoff not present in the original vision-based path (where Claude's own multimodal processing has its own internal budget, opaque to the user, but not additionally billed as visible context injected by a third-party hook).

**Proposed mitigation:** cap injected text at a configurable character budget (suggest default ~8,000 characters, roughly ~2,000 tokens, tunable via a new `--max-context-chars` flag or a persisted `rgt init` setting, following the same settings-persistence pattern already established for `--number-format` in spec `027`). When truncated, append an explicit marker (e.g. `"[... truncated, N characters omitted; full text was still extracted and recorded to the provenance graph ...]"`) so the agent knows the graph has the complete data even if the injected context doesn't.

### 3.5 Files/modules touched (rough scope)

- `src/hooks/parser.rs` — extend `HookToolResponse`/`default_dialect` to recognize the `{type, file: {filePath, base64, originalSize}}` envelope shape.
- `src/hooks/mod.rs` — extend `handle_passive_hook_event` with the new decode → extract → record → emit flow; this is the first code path in this file that ever writes to stdout, so review carefully against the existing "fail open with no stdout" contract for every other case.
- New module, e.g. `src/extract/pdf.rs` — houses the base64-decode + `pdf-extract` call + error handling, kept separate from `parser.rs`'s regex-based text extraction for testability.
- `Cargo.toml`/`Cargo.lock` — add `base64`, `pdf-extract`.
- `tests/unit/hook_parsers.rs` / a new test file — add a fixture using the exact captured envelope shape from §4 (this fixture was already recommended, unimplemented, in the original findings report's §5 regression-test suggestion — implementing it here serves double duty).

---

## 4. Risks and mitigations

| Risk | Mitigation |
|---|---|
| **`pdf-extract`'s behavior on malformed/corrupt PDF input is not documented either way** (confirmed by direct doc lookup in this session — the docs describe the happy path only, with no stated `Result` vs. panic behavior for bad input). Given this project's own prior finding that an unrelated crate (`evalexpr`) could be driven into a process-crashing stack overflow by adversarial input (`docs/verification-of-fixes-develop.md` §7.3), the same caution applies here: a hook that can be crashed by a malformed PDF violates the "fail open, never crash" contract every other part of this hook already honors. | Before shipping, either (a) confirm via source inspection or a fuzz test that `extract_text` returns cleanly on garbage/truncated input, or (b) defensively wrap the call in `std::panic::catch_unwind`, or (c) run extraction with a timeout in case of pathological input causing a hang rather than a crash. Do not assume safety; verify it, consistent with this project's established standard. |
| **The exact envelope shape (`{type: "pdf", file: {...}}`) is an undocumented internal implementation detail**, already flagged in §4 of the findings report — it could change on any Claude Code update with no warning, since the Agent SDK docs type `tool_response` as `unknown`/`Any` deliberately. | Detect the shape defensively (pattern-match, don't assume field presence); if the shape doesn't match, fall through cleanly to the existing fail-open disk-read-fallback path rather than erroring. Add the fixture test in §3.5 so a future Claude Code update that breaks this is caught by CI, not discovered silently in production. |
| **Token-cost/context pollution** from injecting large extracted text on every PDF read (see §3.4). | Truncation cap, as designed above. |
| **`additionalContext`'s exact behavior is confirmed for Claude Code only.** No evidence yet whether Cursor (which shares `default_dialect` in RGT's parser) supports an equivalent mechanism. | Gate this behavior behind the `agent` parameter already threaded through `rgt hook post --agent <name>` (`src/hooks/parser.rs`); enable the `additionalContext` emission only for `claude-code` until Cursor is separately verified (see Open Questions below). The *recording* half of the fix (step 3 in §3.1) is agent-agnostic and safe to enable universally regardless of whether stdout injection is supported, since it degrades gracefully to "just doesn't emit extra context" rather than failing. |

---

## 5. Non-goals / explicitly out of scope for this spec

- **Excel (`.xlsx`) via the same mechanism.** It is *not yet verified* whether Claude Code's `Read` tool on an `.xlsx` file produces an analogous structured envelope, or whether it's handled at all (Excel isn't a natively-multimodal format for Claude the way PDF/images are). This needs its own live-capture verification, following the exact same methodology as §4/§8 of the findings report, before any `calamine`-based extraction code is written against an assumed shape. Flagged as a natural follow-up spec (`034`) once that verification is done.
- **Image files (`.png`/`.jpg`/screenshots).** Even if an analogous `{type: "image", file: {base64, ...}}` envelope exists (plausible, unverified), local extraction would require OCR, not a text-layer extractor like `pdf-extract` — a meaningfully larger dependency (Tesseract bindings or similar) that cuts against the project's "single lightweight binary" positioning (see `docs/weakpoints-and-growth-opportunities.md` §5 on the same tradeoff for the web-dashboard feature). Also, images are the one case where Claude's own vision pass is arguably already doing a comparable job to OCR, unlike PDFs where a real text layer is a strictly higher-fidelity source than a vision pass over a rendered page. Not recommended for near-term work.
- **Cursor and other agents.** See Open Questions below.

---

## 6. Open questions requiring further live verification before generalizing

1. **Does Cursor's hook system support an equivalent `additionalContext`-style output field on `PostToolUse`?** RGT's `default_dialect` currently treats `claude-code` and `cursor` identically for input parsing; this spec should not assume they're identical for *output* capabilities without the same live-capture verification already applied elsewhere in this project.
2. **Does the `{type: "pdf", ...}` envelope shape hold across Claude Code versions/releases**, or was it specific to the version active during this session's testing? The fixture-based regression test in §3.5 is the primary defense here, but this is worth flagging as an ongoing maintenance concern, not a one-time verification.
3. **What does `pdf-extract` actually do on a corrupt/truncated PDF** — this needs a direct answer (source read or empirical fuzz test) before shipping, per the Risks table above.

---

## 7. Success criteria / acceptance tests

- A live end-to-end test (in the same spirit as this session's manual verification, ideally automated where possible): install RGT's hooks for Claude Code, read a real PDF containing known numeric/date values, and confirm — without any explicit `rgt record`/`rgt derive` call by the agent — that (a) `rgt status` shows new tracked nodes matching the PDF's actual values, and (b) the agent's own subsequent response reflects the injected `additionalContext` text (e.g., can correctly answer a question about the PDF's content that would require the real text, not just a vision-based approximation).
- A new unit/contract test fixture using the exact captured `{type: "pdf", file: {...}}` shape, asserting the new code path extracts and records correctly, and that a shape mismatch degrades to the existing fail-open path without panicking.
- A fuzz or malformed-input test against the `pdf-extract` call path, confirming no panic/hang on garbage input (see Risks).

---

## 8. References

- Claude Code hooks reference (raw markdown, fetched and quoted verbatim in this session): [code.claude.com/docs/en/hooks.md](https://code.claude.com/docs/en/hooks.md)
- Claude Code hooks reference (rendered page, first-pass fetch): [code.claude.com/docs/en/hooks](https://code.claude.com/docs/en/hooks)
- Claude Agent SDK hooks docs (confirms `tool_response` is typed `unknown`/`Any`, i.e. deliberately unspecified): [code.claude.com/docs/en/agent-sdk/hooks.md](https://code.claude.com/docs/en/agent-sdk/hooks.md)
- Anthropic PDF support docs (confirms server-side, vision-based, non-local PDF processing): [platform.claude.com/docs/en/build-with-claude/pdf-support](https://platform.claude.com/docs/en/build-with-claude/pdf-support)
- `base64` crate documentation (current API, version 0.23.1 as fetched): [docs.rs/base64/latest/base64](https://docs.rs/base64/latest/base64/)
- `pdf-extract` crate documentation: [docs.rs/pdf-extract/latest/pdf_extract](https://docs.rs/pdf-extract/latest/pdf_extract/)
- This project's own prior findings, which this spec directly builds on and supersedes in part: `docs/findings-hooks-and-value-extraction.md` §4 (original gap + PDF payload capture), §6 (the instruction-based fix this spec improves on), §8 (methodology precedent: live payload capture over documentation inference); `docs/verification-of-fixes-develop.md` §3 (confirms spec `023`'s current shipped behavior, which this spec is additive to, not a replacement for); `docs/weakpoints-and-growth-opportunities.md` §5 (prior vetting of `pdf-extract`/`calamine` as the right crates for this class of problem).
