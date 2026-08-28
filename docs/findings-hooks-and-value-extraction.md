# RGT Findings: Hook Integration, Extraction, Storage, Update & Verification Risks

**Status:** Draft — for maintainer triage
**Scope:** §1–§6: `src/hooks/*`, `src/cli/record.rs`, `src/types/value.rs`, `src/verify/*`. §7: full codebase sweep — `src/updater/*`, `src/store/*`, `src/graph/*`, `src/cli/*`, `src/detection/*`, node-ID generation. §8: comparison against RTK (github.com/rtk-ai/rtk), the project RGT credits as its inspiration.
**Method:** Static code reading + hand-traced regex/parsing behavior + cross-check against existing test suite, **plus live-captured real hook payloads** (§4: PDF `Read` via Claude Code; §8: Bash `cat` via Claude Code, both restart-verified) and **several empirically-confirmed crashes/bypasses in §7** (built `rgt` in release mode and ran it against a live test project — noted per-finding).
**Citation pass:** every finding with a "Fix" in §2, §3, §4, §7, and most of §8 now carries at least one alternative fix and a documentation citation actually fetched and verified (docs.rs, sqlite.org, doc.rust-lang.org, official tool/spec docs) — never a guessed URL; where research found no solid alternative (e.g. §3.2's locale-number-parsing crate search), that's stated explicitly rather than papered over. **Two areas were not covered by this pass and remain as originally flagged, without added citations:** §2.7 (Copilot CLI dialect assumption — still needs a live-captured payload the way §4/§8 got one, not a documentation citation) and §6's per-agent table (Cursor `.cursor/rules`, Copilot's `.github/copilot-instructions.md`, Gemini's `GEMINI.md` — still marked "needs verification" as before; verifying those would follow the same live-capture method as §4/§8, not a citation search).

---

## 1. Executive summary

RGT's provenance tracking has failure surfaces across every major subsystem. §2–§4 (hooks and extraction) were covered first; §7 is a follow-up sweep across the rest of the codebase (self-update, storage/graph, CLI/verification, change-detection), which surfaced two issues more severe than anything found earlier — both confirmed by actually reproducing them, not just reasoning about the code:

1. **`rgt update` can install a completely unverified binary** if a release is missing its `checksums.txt` asset — silently, with no error (§7.1, CRITICAL).
2. **A single `--expression` argument can crash the CLI outright** (stack overflow / SIGABRT), reproduced against both `rgt verify` and `rgt derive` (§7.3, CRITICAL).

Earlier findings, still current:

3. **Hook installation (`rgt init`)** can silently destroy or corrupt a user's existing agent configuration under `--force`, and several agent-specific writers make TOML/JSON assumptions that can produce invalid config files (§2).
4. **Value extraction (`extract_values_from_content`)** is a single pair of naive regexes (`\d+(?:\.\d+)?` for numbers, ISO-8601 only for dates). It has no locale awareness, no sign handling, and a confirmed cross-contamination bug where every recognized date also emits spurious number nodes (§3).
5. **Native PDF (and likely other binary/multimodal) reads never reach the extractor at all via the hook path — confirmed with a real captured payload, not just inferred.** RGT itself contains zero PDF/Excel parsing code, and the host agent's native `Read` tool does *not* surface extracted PDF text anywhere in the hook event JSON — only a structured envelope containing raw base64-encoded binary (§4). §6 has an implementation-ready fix proposal for this specifically.

And from the §7 sweep: the graph's cycle-prevention guard is disconnected from the actual database write path and can silently drop a real dependency edge on reload (§7.2); no SQLite connection sets a `busy_timeout`, so concurrent access can make `rgt status` silently under-report staleness (§7.2); node IDs are truncated to 48 bits with an `ON CONFLICT` clause that never updates value fields on collision, risking silent, permanent data corruption (§7.2/§7.3); and two identical numeric values on the same source line silently collide into one tracked node, losing the other (§7.3, confirmed empirically). Full detail in §7.

6. **Confirmed via live payload capture: every Bash `cat`/`head`/`tail`/`grep` read is invisible to RGT's hook for Claude Code and Cursor** — the two flagship integrations — because `default_dialect` never inspects `tool_input.command`/`tool_response.stdout`, only the native `Read`-tool-shaped fields. RGT already has the parsing logic to fix this (`command_dialect`/`extract_path_from_command`), just not wired to these two agents. Worse: when RTK (RGT's own stated inspiration) is installed alongside it — the exact pairing RGT's README sets up as complementary — RTK's PreToolUse hook transparently rewrites `cat file` into `rtk read file` *before* RGT ever sees it, and RGT's existing command-parser only recognizes the bare verb (`cat`/`read`/...), not `rtk`-wrapped invocations — so even wiring the dialect fix in isn't sufficient on its own. See §8 for the captured payload and fix.

---

## 2. Hook integration findings

### 2.1 Silent settings destruction on malformed JSON (High)

**Where:** `src/hooks/installer.rs:277-282` (`write_claude_code`), `src/hooks/installer.rs:633-637` (`write_json_key`, used by Copilot's VS Code settings)

```rust
let mut existing: serde_json::Value = if settings_file.exists() {
    let content = fs::read_to_string(&settings_file).unwrap_or_default();
    serde_json::from_str(&content).unwrap_or(serde_json::json!({}))
} else {
    serde_json::json!({})
};
```

If the existing `settings.json` fails to parse for any reason (JSONC comments, trailing comma, concurrent write, disk read error swallowed by `unwrap_or_default()`), RGT treats it as an empty object and then writes back **only the RGT keys** — silently discarding every other setting in the file (permissions, keybindings, unrelated hooks, model config, etc.).

**Trigger condition:** `--force`, or first-run when the file doesn't exist yet but a sibling process partially wrote it.

**Suggested fix:** On parse failure, abort with a clear error (`eprintln!` + non-zero exit) instead of silently defaulting to `{}`. Consider writing a `.bak` copy of the original file before any destructive rewrite.

**Alternative fix:** parse with a JSONC-tolerant parser so comments/trailing commas don't count as "malformed" in the first place — the `jsonc-parser` crate ([docs.rs/jsonc-parser](https://docs.rs/jsonc-parser)) is "a JSON parser and manipulator that supports comments and other JSON extensions," with `ParseOptions.allow_comments`/`allow_trailing_commas` defaulting to `true`. This doesn't remove the need for a hard-error fallback when parsing *still* fails — it just shrinks how often that fallback triggers.

### 2.2 Whole-key overwrite instead of merge (High)

**Where:** `src/hooks/installer.rs:309` (`existing["hooks"] = rgt_hooks;`), `src/hooks/installer.rs:339-356` (`write_cursor`)

Claude Code's writer replaces the entire `"hooks"` key rather than merging RGT's `PreToolUse`/`PostToolUse` entries into whatever hooks array already exists. Cursor's writer overwrites the whole `hooks.json` file. If the user has any other hooks configured (their own, or another tool's), `rgt init -g --force` deletes them.

**Confirmed impact on this machine:** this repo's global Claude Code config already has an RTK hook wired into `PreToolUse`/`PostToolUse` in `~/.claude/settings.json` (per this user's `~/.claude/RTK.md`). Running `rgt init -g --force` today would remove it.

**Suggested fix:** Parse `existing["hooks"]["PostToolUse"]`/`["PreToolUse"]` as arrays, append/replace only entries whose `command` starts with `rgt hook`, and leave all other matcher groups untouched. Grounding: Claude Code's own hooks reference ([code.claude.com/docs/en/hooks](https://code.claude.com/docs/en/hooks)) documents `hooks.PreToolUse`/`PostToolUse` as an array of `{matcher, hooks: [...]}` groups — an additive list by design — which is what makes today's whole-key replacement provably wrong against the documented schema, not just risky in practice.

**Alternative fix:** apply a structured patch instead of hand-rolled array surgery. RFC 6902 (JSON Patch) and RFC 7396 (JSON Merge Patch) are both implemented by the `json-patch` crate ([docs.rs/json-patch](https://docs.rs/json-patch)): "A JSON Patch (RFC 6902) and JSON Merge Patch (RFC 7396) implementation for Rust." Merge Patch's recursive-merge semantics still need custom handling for the array-append behavior specifically, but it replaces ad hoc JSON surgery with a declarative, testable diff.

### 2.3 Hermes TOML surgery can emit invalid config (Medium)

**Where:** `src/hooks/installer.rs:754-771` (`insert_into_enabled`)

Naive substring search for the literal `"enabled = ["`, then string-splice `"rgt", ` immediately after it. If the existing `plugins.enabled` array is formatted differently (multi-line array, no space around `=`, or already contains a trailing comment), the marker search misses, and the fallback path appends a **second** `[plugins]` table:

```rust
None => {
    let mut out = content.to_string();
    out.push_str("\n[plugins]\nenabled = [\"rgt\"]\n");
    out
}
```

A duplicate table key is invalid TOML — this would break Hermes' own config parsing for the user, not just RGT's integration.

**Suggested fix:** Use a real TOML parser (e.g. `toml_edit`, which preserves formatting) instead of string search/splice. Verified: [docs.rs/toml_edit](https://docs.rs/toml_edit) — "This crate allows you to parse and modify toml documents, while preserving comments, spaces *and relative order* of items." The TOML spec itself ([toml.io/en/v1.0.0#array](https://toml.io/en/v1.0.0#array)) confirms multi-line arrays are valid syntax ("Arrays can span multiple lines... Any number of newlines and comments may precede values, commas, and the closing bracket") — exactly the case the current `"enabled = ["` substring search can't handle.

**Alternative fix:** the plain `toml` crate ([docs.rs/toml](https://docs.rs/toml)) with typed serde (de)serialization — "a serde-compatible TOML-parsing library." Simpler to reason about than `toml_edit`'s document-editing API, but confirmed to **not** preserve formatting/comments (it operates on a semantic `Value`/`Table` tree and would rewrite the whole file) — a real tradeoff against a user's existing comments/formatting, not a strict downgrade from `toml_edit`.

### 2.4 `--force` truncates user content below the RGT marker (Medium)

**Where:** `src/hooks/installer.rs:773-787` (`truncate_from_marker`)

```rust
match content.lines().position(|l| l.contains(marker)) {
    Some(idx) => content.lines().take(idx).collect::<Vec<_>>().join("\n").trim_end().to_string(),
    ...
}
```

Used by `write_toml_block`/`write_text_block` under `--force`: everything from the first line containing `"## RGT Integration"` onward is discarded, then RGT's block is re-appended. Since these are human-editable rules files (`.clinerules`, `AGENTS.md`, `.windsurfrules`, etc.) that users are expected to add their own notes to, and RGT's own instructions explicitly tell agents to read/append to these files, any user content placed **after** RGT's block is silently deleted on re-init with `--force`.

**Suggested fix:** Locate both the marker's start line and a matching end marker (e.g. `<!-- /RGT -->`), and only replace the bytes between them.

**Prior art:** this is exactly what `conda init` does in practice — verified directly from source ([github.com/conda/conda/blob/main/conda/core/initialize.py](https://github.com/conda/conda/blob/main/conda/core/initialize.py)): `CONDA_INITIALIZE_RE_BLOCK` matches `# >>> conda initialize >>> ... # <<< conda initialize <<<` and replaces only the text between those two literal markers in a user's shell rc file, leaving everything else untouched (the PowerShell variant uses `#region`/`#endregion`). A real, shipped example of the same pattern, not a novel proposal.

### 2.5 Silent no-op when `rgt` isn't resolvable on PATH (Medium — UX/observability)

**Where:** all of `src/hooks/glue/mod.rs` (every glue artifact shells out to the bare name `rgt`), `install.sh` (macOS/Linux only, installs to `~/.local/bin`)

Every integration — Claude Code's `"command": "rgt hook post"`, OpenCode/Pi's `spawnSync("rgt", ...)`, Hermes' `shutil.which("rgt")` — assumes `rgt` is resolvable in whatever environment the hook subprocess inherits. Hooks are designed to fail open (`src/hooks/mod.rs:20-21` doc comment: "every error path returns `Ok(())` with no stdout"), which is correct for not blocking the agent, but it means a broken PATH produces **zero visible signal**: the user believes provenance capture is active (hooks are installed, `rgt init` reported success), but nothing is ever recorded.

Concrete scenarios where this bites:
- Windows users (no install script story at all in `install.sh`).
- Any hook runtime that spawns with a minimal/non-login shell environment (common for editor-embedded tool subprocesses).

**Suggested fix:** `rgt status` (or a new `rgt doctor`) should detect "0 nodes but N hook config files present" and suggest checking `which rgt` from the same shell context the agent uses — a real, documented pattern in other CLI tools: Flutter's `flutter doctor` ([docs.flutter.dev/install/troubleshoot](https://docs.flutter.dev/install/troubleshoot), used throughout as the standard way to diagnose an installed-but-misconfigured environment) and Homebrew's `brew doctor` ([docs.brew.sh/Troubleshooting](https://docs.brew.sh/Troubleshooting): "checks your system for potential problems and exits with non-zero status if any are found") are both official-doc-confirmed precedents for this exact subcommand shape.

**Complementary fix:** resolve to an absolute path at `rgt init` time and write that into the hook command instead of relying on PATH resolution at hook-invocation time, using Rust's `std::env::current_exe()` — confirmed by its official docs ([doc.rust-lang.org/std/env/fn.current_exe.html](https://doc.rust-lang.org/std/env/fn.current_exe.html)): "Returns the full filesystem path of the current running executable." These two fixes address different failure points (prevention vs. detection) and are worth doing together, not as alternatives to each other.

### 2.6 Empty matcher fires the hook on every tool call (Low — perf/behavior surprise, not a failure)

**Where:** `src/hooks/installer.rs:287`, `:296` (`"matcher": ""`)

Claude Code's installed hook matches every tool (Bash, Edit, Write, WebFetch, …), not just file reads. `rgt hook pre/post` fails open so this doesn't break anything, but it means a subprocess spawns on every single tool call, most of which will never yield a capturable path.

**Suggested fix:** scope the matcher to `"Read|Write|Edit"` — confirmed as valid, documented syntax. Claude Code's hooks reference ([code.claude.com/docs/en/hooks](https://code.claude.com/docs/en/hooks)) states that a matcher containing only letters/digits/`_`/`-`/spaces/`,`/`|` is treated as exact tool-name string(s) with `|` or `,` as OR-separators (e.g. `"Edit|Write"` or `"Edit, Write"`), while anything else falls back to being treated as an unanchored regex, and empty/`"*"`/omitted matches everything — confirming today's `""` is documented to mean "fires on every tool call," not just an oversight to guess at.

### 2.7 Copilot CLI dialect is unverified against a real payload (Low — flag as assumption, not a proven bug)

**Where:** `src/hooks/parser.rs:126-153` (`copilot_dialect`)

The dual-dialect parser guesses at two possible Copilot CLI event shapes (`toolArgs` as a JSON-encoded string, or a `command_dialect` fallback). No test fixture in `tests/unit/hook_parsers.rs` is based on a captured real Copilot CLI hook payload. If the actual shape differs, this integration silently captures nothing, with no way to tell short of manual testing against the real tool.

---

## 3. Number/date extraction findings

### 3.1 No native Excel/PDF parsing in the `rgt record` CLI path (High)

**Where:** `src/cli/record.rs:27` — `fs::read_to_string(file)`

`fs::read_to_string` requires valid UTF-8. `.xlsx` (ZIP container) and `.pdf` (binary streams) are not UTF-8, so `rgt record report.xlsx` / `rgt record report.pdf` fail with a generic `"failed to read file: stream did not contain valid UTF-8"` and record 0 values, with no format-aware error message telling the user why. `Cargo.toml` has no `calamine` (xlsx), no PDF crate, and no `csv` crate — CSV "works" only because it happens to already be plain text; there is no real cell/column-aware parsing.

**Note:** the hook capture path (`rgt hook post`) doesn't hit this exact UTF-8 error for the manual-CLI reason above, but it fails for a related, now-confirmed reason of its own for native PDF reads — see §4.

**Suggested fix:** at minimum, detect non-UTF-8 content and return an explicit "unsupported file format for `rgt record`; supported natively: plain text/CSV. For PDF/Excel, use the agent's own read/extraction tool and let the hook capture it, or convert to text/markdown first" message rather than a raw I/O error.

**More complete alternative fix:** add real binary-format support rather than just a better error message. `calamine` ([docs.rs/calamine](https://docs.rs/calamine): "a pure Rust library to read Excel and OpenDocument Spreadsheet files," covering xls/xlsx/xlsm/xlsb/ODS) for Excel, and `pdf-extract` ([docs.rs/pdf-extract](https://docs.rs/pdf-extract), providing `extract_text()`/`extract_text_by_pages()`) for PDFs. Caveat worth flagging either way: `pdf-extract` only recovers a PDF's existing text layer — scanned/image-only PDFs yield an empty string, not an error, so this alone doesn't solve OCR-requiring documents (no OCR crate is proposed here).

### 3.2 No locale-aware number parsing (High)

**Where:** `src/hooks/parser.rs:49` — `Regex::new(r"\b(\d+(?:\.\d+)?)\b")`

- `.` is the only recognized decimal separator; commas are never treated as decimal separators.
- No thousands-separator awareness at all (neither `,` nor `.` nor space nor `'`).
- No sign handling — a leading `-` is not part of the match.
- No accounting-style negative parentheses (`"(500)"`).
- No percent/currency-symbol semantics (not corrupting, but meaning is dropped).

**Confirmed failure modes (hand-traced against the regex):**

| Input | Locale | Intended value | What RGT extracts |
|---|---|---|---|
| `1,234.56` | en-US | `1234.56` | `1` **and** `234.56` (two bogus nodes) |
| `1.234,56` | pt-BR / de-DE / es | `1234.56` | `1`, `234`, `56` (three bogus nodes) |
| `-42` | any | `-42` | `42` (sign silently dropped — a wrong value recorded as trustworthy) |
| `(500)` | accounting | `-500` | `500` |

**Suggested fix:** add a locale/format flag to `rgt record`/`rgt init` (e.g. `--number-format us|eu|auto`), and extend the regex/parsing to consume an optional leading sign and locale-appropriate separator groups before falling back to plain digit runs. At minimum, detect and reject/flag numbers immediately adjacent to a `,` or `.` that don't fit the assumed grouping, rather than silently splitting them.

**Checked for an existing-crate alternative — none fits, worth reporting honestly rather than skipping.** ICU4X's `icu_decimal` is formatting-only ("Formatting basic decimal numbers" per docs.rs — no parsing API); `fixed_decimal`'s `Decimal`/`UnsignedDecimal` types parse only canonical/invariant-form numbers (ASCII digits, `.` decimal point), not locale-grouped strings like `"1.234,56"`. ECMA-402 (`Intl.NumberFormat`) is explicitly formatting-only by design, confirmed in the spec text itself ([tc39.es/ecma402/#numberformat-objects](https://tc39.es/ecma402/#numberformat-objects)): "While the API includes a variety of formatters, it does not provide any parsing facilities. This is intentional, has been discussed extensively..." So there's no drop-in library for this half of the problem — the hand-rolled `--number-format` flag above is, as far as this research could confirm, the realistic path, not a stopgap.

### 3.3 Non-ISO dates are invisible, and their fragments pollute the number stream (High)

**Where:** `src/hooks/parser.rs:47-48`

Only `\d{4}-\d{2}-\d{2}(?:T...)?` is recognized as a date. Formats like `27/08/2026`, `08/27/2026`, `2026/08/27`, `27-Aug-2026`, `August 27, 2026`, or Excel serial dates are not recognized as dates at all — and because the number regex runs independently and unconditionally over the same line, their day/month/year components get individually captured as unrelated `Number` nodes.

**Confirmed cross-contamination bug (traced by hand, and reproducible against the project's own test fixture):**

For the line `"start: 2026-01-15"`, `extract_values_from_content` produces:
- 1 correct `Date` node, **and**
- 3 spurious `Number` nodes: `2026`, `1` (from `"01"`), `15` — because the number regex's `\b...\b` boundaries are satisfied by each hyphen-delimited digit run independently of the already-matched date span.

This is not caught by the existing test suite: `tests/unit/test_record.rs::test_extract_dates_from_content` only asserts `date_count == 3` (filtering on `ValueKind::Date`), never the *total* length of `extracted`, so the 9 extra bogus number nodes (3 per date × 3 dates in that fixture) pass silently today.

**Suggested fix:** when the date regex matches a span, either (a) skip re-scanning that byte range with the number regex, or (b) run the date regex first and exclude/mask matched spans before running the number regex. Add a test asserting `extracted.len() == 3` (not `>= 3`) for the date fixture to prevent regression. Verified: the `regex` crate documents alternation (`x|y`) and named capture groups (`(?P<name>exp)`, accessed via `&caps["name"]`) ([docs.rs/regex](https://docs.rs/regex)), which together let a single combined pattern (`(?P<date>...)|(?P<number>...)`) claim each span exactly once in one pass — eliminating the two-pass overlap by construction rather than patching around it.

**Separate alternative, for the "non-ISO dates are invisible" half specifically:** rather than hand-writing more per-format regexes, the `dateparser` crate ([docs.rs/dateparser](https://docs.rs/dateparser)) is a real, maintained library that parses "date strings in commonly used formats" — confirmed support for Unix timestamps, RFC 3339/2822, Postgres timestamps, ISO 8601, US-style (`MM/DD/YYYY`), European-style (`DD Mon YYYY`), and named-month formats — returning a `chrono::DateTime<Utc>` directly, which is what RGT's `ValueData::Date` already wraps.

---

## 4. Architecture note: how PDF/Excel content *can* reach the extractor — and a CONFIRMED gap for native multimodal PDF reads

RGT's hook path (`rgt hook pre|post`) is format-agnostic by construction — it never looks at file extensions. `handle_passive_hook_event` (`src/hooks/mod.rs:22-91`) only ever consults two fields normalized out of the raw hook event JSON (`src/hooks/parser.rs:6-15`, `:111-122`):

```rust
pub struct HookToolInput { pub path: Option<String>, pub file_path: Option<String> }
pub struct HookToolResponse { pub content: Option<String> }
```

Two independent paths feed text into `extract_values_from_content`:

1. **`tool_response.content` is present** (a flat string) → used directly, regardless of what the original file on disk is. The original file's bytes are touched only for `blake3` hashing/mtime (`src/hooks/mod.rs:48-56`), which works fine on any binary.
2. **`tool_response.content` is absent** → RGT falls back to `fs::read_to_string(path_obj)` (`src/hooks/mod.rs:40-46`) reading whatever `path` the event pointed at, from disk.

The idea — the host agent's own document-reading capability produces text that lands in one of those two places, and RGT rides on it with zero PDF/Excel-specific code — is architecturally sound and matches the glue module's own doc comment (`src/hooks/glue/mod.rs:1-8`: "thin delegates only," "zero business logic"). But for the specific case of a native multimodal `Read` on a PDF, **it doesn't happen**, confirmed below.

### CONFIRMED: native PDF reads never reach the extractor via the hook path (High)

**Verification performed and completed** (an earlier draft of this section reported an inconclusive live-capture attempt; that attempt failed only because the debug hook was installed mid-session without a restart — see the postmortem at the end of this section for what was wrong with it and why). After a genuine session restart, a project-scoped debug hook (`.claude/settings.local.json`, additive, later removed) captured the **real** `PostToolUse` payload for a `Read` on a generated test PDF (`test_invoice.pdf`, containing `Amount: 1234.56`, `Due Date: 2026-01-15`, `Quantity: 42`). The captured `tool_response` was:

```json
{
  "type": "pdf",
  "file": {
    "filePath": ".../test_invoice.pdf",
    "base64": "<raw PDF bytes, base64>",
    "originalSize": 14479
  }
}
```

**There is no `content` string field at all.** This resolves the caveat definitively, and more sharply than originally hypothesized:

- It's not a type mismatch causing a hard deserialization error (a missing optional field just deserializes as `None` in serde — no panic, no error).
- It's that **no extracted text exists anywhere in the hook payload** — only a structured envelope carrying the raw, still-binary PDF bytes.
- The plain-text transcript I (the model) saw when reading the PDF in-session ("Invoice Report / Amount: 1234.56 / ...") is rendered for **model consumption only** and never reaches the `tool_response` JSON that hooks receive.

Tracing the consequence through `handle_passive_hook_event` (`src/hooks/mod.rs:40-46`): `capture.content` is `None` → falls back to `fs::read_to_string(path_obj)` on the **original PDF path** → fails (invalid UTF-8, genuinely binary) → the whole event returns `Ok(())` via the fail-open error path → **zero values recorded, zero signal to the user.**

**Practical implication for the user's original hypothesis:** a native `Read` on a PDF, by itself, never gets captured through the hook — contrary to the assumption that the agent's own PDF-reading capability would surface usable text to RGT for free. The hypothesis *does* still hold, but only for a different, more deliberate workflow: the agent must explicitly produce a **separate plain-text/markdown file** on disk (e.g., "transcribe this PDF's numbers to `invoice_extracted.md`") via a `Write`-style tool call, and either that `Write` event (whose `tool_input.path` now points at a real text file RGT's fallback disk-read *can* parse) or a later `Read` of that same file, is what actually reaches `extract_values_from_content`. The original PDF itself is never touched for content in that flow — only the derived text file is.

### Confirmation against official Claude Code / Claude API documentation

Before proposing a fix, I had this checked against the actual docs rather than relying on inference from one captured payload. Findings, each backed by a specific doc:

1. **The `tool_response` shape is not a documented contract.** The Claude Code hooks reference ([code.claude.com/docs/en/hooks](https://code.claude.com/docs/en/hooks)) confirms `PostToolUse` exists and fires after a tool succeeds, but does not specify `tool_response`'s structure for any tool. The Agent SDK hooks docs ([code.claude.com/docs/en/agent-sdk/hooks.md](https://code.claude.com/docs/en/agent-sdk/hooks.md)) type it as `unknown` (TS) / `Any` (Python) — deliberately unspecified. **The `{type: "pdf", file: {filePath, base64, originalSize}}` shape captured above is an internal implementation detail, not a stable API RGT should hard-code against.**
2. **No hook, for any tool, ever receives extracted text for a binary/multimodal file — only the raw result.** Nothing in the hooks reference or SDK docs exposes a "text was extracted, here it is" field.
3. **The decisive finding: PDF interpretation happens server-side, inside the model — never on the local CLI.** Per Anthropic's PDF support docs ([platform.claude.com/docs/en/build-with-claude/pdf-support](https://platform.claude.com/docs/en/build-with-claude/pdf-support)): "PDF support relies on Claude's vision capabilities," with each page rendered as an image and interpreted by the model over the API. Claude Code does not run local PDF/text extraction before or after the tool call — it base64-encodes the file and sends it to the model, which produces an answer fresh on each read. **There is no point in the local pipeline, ever, where a plain-text transcript of the PDF exists as a file or in-memory object a hook could reach.** The text visible in the model's own turn is generated per-request and is not persisted anywhere hook-visible.
4. **No hook or extension point downstream of the model's interpretation exists either** — `PostToolUse` fires on the tool's raw result, strictly before the model has said anything about it.

### Recommended fix (revised after doc verification — ranked by robustness, not just cost)

Point 3 above rules out the assumption I was implicitly testing: that "extracted PDF text" exists *somewhere* locally and RGT just needs the right field. It doesn't exist locally at all, for any agent built the same way (server-side vision processing is Claude's general PDF-support model, not a Claude-Code-specific quirk) — it's synthesized fresh inside the model on every read and never written down unless something *after* that read writes it down.

1. **Primary fix, and the only genuinely robust one: instructions, not hook code.** Since no local text extraction exists anywhere in the pipeline, the only place PDF/Excel/image content ever becomes text is inside the model's own response. RGT already tells agents (via `AGENTS.md`/rules files) to run `rgt record <file>` after reading a data file — extend that instruction: for PDF/Excel/image sources, the agent must explicitly write the extracted numbers/dates to a plain-text/markdown file (or pipe them via `rgt record --stdin`, which `execute_record`'s existing `stdin: bool` parameter already supports) as its own next action. This requires no engineering, is immune to any given agent's undocumented hook-payload quirks, and generalizes across all 13 supported agents uniformly, since it depends only on the universally-supported "write plain text to a file" capability rather than on reverse-engineering each agent's internal tool-response schema.
2. **Optional, fragile fallback: decode `tool_response`'s embedded bytes and run local extraction anyway.** `rgt` could recognize `tool_response.type == "pdf"` and similar structured types, base64-decode `file.base64`, and run an embedded PDF-text-extraction crate (e.g. `pdf-extract` — verified: [docs.rs/pdf-extract](https://docs.rs/pdf-extract) provides `extract_text()`/`extract_text_by_pages()`, same crate proposed in §3.1) purely against the *original document's own text layer* (independent of anything Claude does) before falling back to failing open. This is real, additive coverage for agents/users who never get to option 1's explicit instruction — but it must be built and tested per-agent, since each of the other 12 agents likely has its own different, equally undocumented envelope shape for how it packages a PDF/image tool result, discovered the same way this one was (capture a real payload, don't guess). Treat it as a best-effort enhancement with an explicit "may break on a Claude Code/other-agent update" comment in the code, not a guaranteed feature. It also doesn't help for scanned/image-only PDFs with no text layer — `pdf-extract` only recovers an existing text layer, so no OCR is proposed here.
3. Either way, **remove the current silent failure**: today, a PDF read produces zero signal anywhere (`rgt status` shows nothing, no warning, nothing). At minimum, `rgt hook post` should be able to detect "this event had a structured, non-string `tool_response` I couldn't extract from" and log something (to stderr or a debug log), rather than the current indistinguishable-from-success silence.

### Postmortem: why the first live-capture attempt failed (kept for the record)

The first attempt to capture this payload — done *without* a session restart — produced no capture file for either the PDF or a control `.txt` read. An earlier draft of this section wrongly floated "harness doesn't route Read hooks" as an explanation, and briefly leaned on an unrelated `ls -la` output oddity as supporting evidence; that oddity doesn't actually support anything and was disregarded. The correct explanation, confirmed once a genuine restart fixed it: **hook configuration is read once at session start and is not hot-reloaded from disk mid-session** — the same reason RGT's own README Quick Start says "Restart your AI tool" as an explicit numbered step after `rgt init`. Editing `.claude/settings.local.json` inside an already-running session cannot make a hook fire in that session, with or without RGT, with or without PDFs; it only takes effect in a session started after the edit. This is a real, generally-applicable gotcha (see the new finding below), not evidence against RGT's hook mechanism itself.

**Cleanup performed:** the temporary `.claude/settings.local.json` was removed after the test; the repo has no leftover hook config from this verification. Test artifacts (`test_invoice.pdf/.txt`, `cupsfilter.log`) were written only to the session scratchpad, not the repo.

### 2.8 No detection of "hooks installed but not yet active this session" (Low — observability)

Because hook config is read at session start, any workflow of the form `rgt init` → immediately use the agent in the *same* running session → expect capture to work, will silently fail with zero signal, for the same reason described above. This is already flagged as a restart requirement in the README's Quick Start, but nothing in the tool itself reinforces it: `rgt init`'s own success output doesn't remind the user to restart, and `rgt status` has no way to distinguish "hook never fired because it's not configured" from "hook never fired because the session predates configuration." Suggested fix: have `rgt init` print an explicit "restart your AI tool now for this to take effect" line (a couple of the agent-specific messages already say "(full hook)" but none mention restart).

---

## 5. Suggested priority order for the maintainer

1. **§2.1 / §2.2** — stop the installer from ever silently destroying existing hook/settings content. This is the highest-blast-radius issue (affects every agent, corrupts user config, not just RGT's own).
2. **§4 (confirmed: native PDF reads never reach the extractor, and never can via hooks alone — verified against official Claude Code/Claude API docs)** — ship the instructions fix (§4 "Recommended fix" item 1) immediately: it's the only genuinely robust option, requires no engineering, and is a documentation correction with zero code risk. Treat item 2 (best-effort local decode of the undocumented envelope) as a separate, lower-confidence follow-up, not a blocker.
3. **§3.3** — the date/number cross-contamination bug is a correctness bug in the core value graph, cheap to fix, and currently masked by an incomplete test assertion.
4. **§3.2** — locale-aware number parsing; likely the highest-value fix for real-world (non-US-formatted) source documents.
5. **§2.3 / §2.4** — TOML/text-block surgery robustness; lower frequency but can corrupt user files outright.
6. **§2.5 / §2.6 / §2.7 / §2.8 / §3.1** — observability, perf, and scope-of-support issues; not correctness bugs, but worth addressing for trust in the tool.

Suggested new regression test, now that the payload shape is known: add a `tests/unit/hook_parsers.rs` fixture using the captured shape (`{"tool_response": {"type": "pdf", "file": {"filePath": ..., "base64": ..., "originalSize": ...}}}`) asserting `normalize_agent_event` returns a capture with `content: None` — locking in today's (gap) behavior so a future fix is a deliberate, tested change rather than an accidental one.

---

## 6. Proposed fix, ready to implement: explicit-report instruction for non-text sources

Decided direction (per maintainer discussion): fix this via agent instructions, not hook code — confirmed as the only robust option in §4, since no local text extraction of PDFs/Excel/images exists anywhere in any of these agents' pipelines (server-side model processing only, per the doc check in §4).

### 6.1 Scope: binary/non-text sources only, not every read

The instruction must be scoped narrowly, to **PDF, Excel (.xlsx), images, and other non-plain-text formats** — explicitly **not** every file read. Plain text/CSV/JSON/Markdown is already captured automatically by the passive hook with zero agent action required (that's the entire point of the hook-capture design — see README §"Why Provenance Tracking Matters"). Broadening the explicit-report instruction to cover formats that already work automatically would:
- Add redundant work to every single file read for no benefit.
- Reintroduce the exact "agent might forget to record" failure mode RGT's passive-capture design exists to eliminate, for the cases where it currently doesn't apply.

The instruction should read (for placement in every rules/instructions artifact, new or existing):

> ### Recording Values from Non-Text Sources (PDF, Excel, images, scanned documents)
> RGT's passive hook can only capture plain-text file content — it cannot see inside PDF, Excel (`.xlsx`), image, or other binary files, even when you read them directly. This is a platform limitation (PDF/image interpretation happens inside the model itself, never as a local text file), not something RGT's hook can intercept.
>
> After reading a PDF, Excel, image, or other non-text source, you must report its values explicitly:
> - Extract the numeric values and dates from what you just read.
> - Pipe them to RGT in plain text form: `echo "Amount: 1234.56\nDue Date: 2026-01-15" | rgt record --stdin`
> - (Alternative: write your extraction to an intermediate `.txt`/`.md` file — that file's own read/write will be captured automatically, since it's plain text.)
>
> Do **not** do this for plain-text, CSV, JSON, or Markdown files — those are already captured automatically; no action needed.

### 6.2 Per-agent implementation plan

| Agent | Current state | Proposed change | Confidence |
|---|---|---|---|
| Codex CLI | `AGENTS.md`, via `codex_section()` (`installer.rs:365-368`) | Append new subsection to the constant string | High — file/mechanism already proven |
| Windsurf | `.windsurfrules`, via `write_windsurf` (`installer.rs:399-414`) | Append new subsection to `rules_content` | High |
| Cline/Roo Code | `.clinerules`, via `glue::CLINE_RULES` (`glue/mod.rs:93-111`) | Append new subsection to the constant | High |
| Antigravity | `.agents/rules/antigravity-rgt-rules.md`, via `glue::ANTIGRAVITY_RULES` (`glue/mod.rs:114-132`) | Append new subsection | High |
| Kilo | `.kilocode/rules/rgt-rules.md`, via `glue::KILO_RULES` (`glue/mod.rs:135-153`) | Append new subsection | High |
| Copilot CLI | `AGENTS.md`-equivalent at Copilot CLI config dir, via `glue::COPILOT_CLI_RULES` (`glue/mod.rs:168-186`) | Append new subsection | High |
| Mistral Vibe | `~/.vibe/prompts/rgt.md`, via `glue::VIBE_PROMPT` (`glue/mod.rs:156-162`) | Append new subsection | High |
| **Claude Code** | **Hook only — no instructions file written at all** (`write_claude_code`, `installer.rs:258-321`) | **New:** write a project (or global) `CLAUDE.md` section, idempotent via the existing `write_text_block`/`RGT_MARKER` helpers, same pattern as the rules-tier agents above | Medium — mechanism (`CLAUDE.md` as a memory file) is well-established for Claude Code, but needs a decision on project vs. global placement and interaction with a user's existing `CLAUDE.md` |
| **Cursor** | **Hook only — no instructions file** (`write_cursor`, `installer.rs:323-363`) | **New:** likely a `.cursor/rules/*.mdc` file (Cursor's project-rules mechanism) | **Needs verification** — confirm the exact rules-file format/location against Cursor's own docs before implementing, the way the Claude Code hook payload was verified in §4 |
| **Copilot Chat (VS Code)** | **Hook only** (`write_copilot`, `installer.rs:422-463`) | **New:** possibly `.github/copilot-instructions.md` (a documented repo-scoped Copilot instructions file) | **Needs verification** against Copilot Chat's own docs |
| **Gemini CLI** | **Hook only** (`write_gemini`, `installer.rs:466-477`) | **New:** possibly `GEMINI.md`, if Gemini CLI supports an equivalent memory/instructions file | **Needs verification** |
| **OpenCode / Pi / Hermes** | **Plugin only** (TS/Python glue shelling out to `rgt hook post`) — plugins can't "instruct" the model directly | **Unclear** whether these tools expose any equivalent instructions/rules-file mechanism at all; if not, this fix path may not be available for these three agents, and the gap would remain unaddressed for them specifically | **Needs verification** — lowest confidence of the group |

### 6.3 Before implementing

- Run the same kind of live-capture verification used for the Claude Code PDF payload (§4) for at least Cursor and Copilot Chat, since those are the two other hook-tier agents most likely to have real users hitting this gap soon. Don't assume the same envelope shape applies — each agent's tool-response format is independently undocumented (confirmed in §4's doc check) and must be verified per agent, not inferred from Claude Code's shape.
- Use the existing idempotent-write helpers (`write_text_block`, `RGT_MARKER`) for any new file this touches, consistent with the rest of the installer, and be mindful of the `--force` truncation bug already flagged in §2.4 when appending to files a user may have added their own content below.
- Add a short line to `rgt init`'s output for any hook-tier agent (Claude Code, Cursor, Copilot, Gemini) once it also writes an instructions file, so users can see both artifacts were configured (mirrors how rules-tier agents already report "(rules-file) -> path" alongside "(full hook) -> path" today).

---

## 7. Codebase-wide sweep: additional findings (self-update, storage/graph, CLI/verify, change-detection)

Four parallel audits covered the remaining major subsystems not touched by §2–§4 above: the self-update mechanism, the SQLite storage/graph-invalidation layer, the CLI commands and derivation-verification logic, and the tier1/tier2 file-change-detection layer. Method: static code reading + tracing, with the CLI/verify audit additionally confirming several findings empirically by building `rgt` in release mode and running it against a live test project (noted per-finding below). Two findings in this sweep are more severe than anything found in §2–§4: a checksum-verification bypass in `rgt update`, and a stack-overflow crash reachable from a single CLI argument.

### 7.1 Self-update mechanism (`src/updater/*`, `src/cli/update.rs`)

**CRITICAL — Checksum verification is silently skipped when a release has no `checksums.txt` asset, installing an unverified binary.**
`src/cli/update.rs:82-94`: `find_checksum_asset` (`src/updater/github.rs:92-94`) returns `None` if no asset is named exactly `checksums.txt`; the `if let Some(...) = ... { verify } ` has no `else` branch, so a missing checksum asset falls straight through to extraction and installation with zero integrity check and no warning printed. Any release published without that one specific asset — accidental CI omission, or a compromised publish token/CI job — causes `rgt update` to install a completely unverified binary over the running executable. The existing test (`tests/contract/test_update_contract.rs:102-114`, `test_no_checksum_returns_error`) only checks that `find_checksum_asset` returns `None` in isolation; it never calls `execute_update` or asserts an actual error surfaces, so the real code path is untested despite the test's name. **Fix: treat a missing checksum asset as a hard error, requiring an explicit opt-out (e.g. `--skip-checksum`) to relax it — never a silent default skip.** This mirrors the canonical "secure by default, insecure only via an explicit named flag" precedent set by curl's `-k`/`--insecure` ([curl.se/docs/manpage.html](https://curl.se/docs/manpage.html): "This option explicitly allows curl to perform insecure SSL connections... All SSL connections are attempted to be made secure... by default"). No second fix is offered here — silently skipping has no defensible alternative; the only open design question is what to name the opt-out flag.

**HIGH — Checksum is fetched from the same untrusted, unsigned source as the binary itself — no independent signature verification exists anywhere in the tree.** `github.rs:92-94, 136-172`, `checksum.rs:54-57`. `checksums.txt` is just another asset in the same release JSON as the binary — same repo, same trust boundary, no GPG/Sigstore/minisign anywhere in the codebase. Anyone who can publish a malicious release can attach a matching checksum for it, and verification passes. SHA-256 here only proves "these are the bytes GitHub is currently serving," never "these bytes came from a trusted build." **Fix options, in order of operational cost — all verified real and current:**
1. **GitHub Artifact Attestations** — the lowest-effort option if releases are already built via GitHub Actions. Confirmed via [docs.github.com](https://docs.github.com/en/actions/security-for-github-actions/using-artifact-attestations/using-artifact-attestations-to-establish-provenance-for-builds): "Artifact attestations enable you to increase the supply chain security of your builds by establishing where and how your software was built," verified via `gh attestation verify`. Requires no custom key management — CI attests at build time via GitHub's own OIDC.
2. **Sigstore/cosign** — confirmed via [docs.sigstore.dev](https://docs.sigstore.dev/cosign/verifying/verify/): `cosign verify` validates artifact signatures, with keyless (Fulcio/OIDC) or key-based verification plus transparency-log checks.
3. **minisign** — confirmed via its own repo ([github.com/jedisct1/minisign](https://github.com/jedisct1/minisign)): "a dead simple tool to sign files and verify signatures," Ed25519-based, with prebuilt binaries — the lightest-weight option if a full Sigstore/cosign toolchain is unwanted.

**HIGH — Version comparison is plain string inequality, not semver ordering — `rgt update` can silently install an older, potentially vulnerable build.** `src/cli/update.rs:20-39`: `latest_version != current_version` — no `semver` crate anywhere in `Cargo.toml`/`Cargo.lock`, no numeric comparison. `/releases/latest` returns the most recently *published* release, not the highest version number; if an old tag is republished (deliberately or via a compromised token), the tool reports an "update" and installs the older build. Also fires non-adversarially for any locally-built dev binary with a version string that doesn't match published "latest." **Fix: parse both with the `semver` crate and only proceed when `latest > current`.** Confirmed via [docs.rs/semver](https://docs.rs/semver): "A parser and evaluator for Cargo's flavor of Semantic Versioning," with `Version` "as defined by semver.org" and real comparison operators, not string comparison. **This alone doesn't fully close the gap** — GitHub's own REST API docs for `/releases/latest` ([docs.github.com/en/rest/releases/releases](https://docs.github.com/en/rest/releases/releases)) state outright: "The latest release is the most recent non-prerelease, non-draft release, sorted by the `created_at` attribute" — time-based, not version-magnitude-based, directly confirming a republished older tag really would be returned as "latest."

**Alternative/complementary fix:** don't trust `/releases/latest` at all — call the "list releases" endpoint (same docs page), parse every tag with `semver`, and select the max version locally. Same crate, but the trust anchor moves from GitHub's time-based heuristic to a locally-computed maximum.

**MEDIUM — `AtomicReplace::replace` isn't atomic as a whole, and has no rollback on failure.** `src/updater/atomic.rs:33-40`. A plain `rename(tmp, target)` is already atomic on POSIX and would replace an existing target in one syscall; the extra backup-rename step (`target → backup`, then `tmp → target`) creates a window where no `rgt` binary exists at all if the process is killed between the two renames — leaving only `rgt.old`/`rgt.tmp` behind, no automatic recovery. If the second rename fails for a non-crash reason (read-only dir, full disk, AV lock), the function returns `Err` with the target already deleted and never restores the backup. **Fix: skip the unnecessary backup-rename on POSIX (direct atomic replace); if keeping a backup, add an explicit rollback path when the second rename fails.** Precision note on citation: Rust's own `std::fs::rename` docs ([doc.rust-lang.org/std/fs/fn.rename.html](https://doc.rust-lang.org/std/fs/fn.rename.html)) confirm it "renam[es] a file or directory to a new name, replacing the original file if `to` already exists," mapping to `rename()` on Unix — but the page itself never uses the word "atomic." That guarantee is a well-established POSIX property of `rename(2)`, not a claim Rust's own docs make explicitly; worth being precise about that distinction rather than over-citing.

**Alternative fix: don't hand-roll this at all.** The `self_replace` crate ([docs.rs/self_replace](https://docs.rs/self_replace)) is purpose-built for exactly this: "allows binaries to replace themselves with newer versions or to uninstall themselves." On Unix it performs "an atomic move with rename"; on Windows, since a running `.exe` can't be unlinked, it "create[s] a copy of our own executable first... open[s] that copied executable with `FILE_FLAG_DELETE_ON_CLOSE`... spawn[s] it and wait[s] for our own shut down" — i.e. it already solves the Windows problem this report flagged above as unverified/needing a live test, which is a strong reason to prefer it over extending `atomic.rs` by hand.

**LOW — TOCTOU between checksum verification and archive extraction** (re-reads the same path twice, `checksum.rs:45-51` then `update.rs`'s `extract_archive`); low practical severity since it requires local code-exec capability equivalent to what's already needed to exploit it. **LOW/hygiene — backup-cleanup errors always swallowed via `let _ =` (`atomic.rs:34,40`)**, giving no diagnostic when rollback/cleanup fails.

**Verified sound, no finding:** GitHub API/download URLs are hardcoded HTTPS to the correct repo (not derived from config/env); `ureq` is configured with `rustls`, no cert-bypass flags anywhere in the tree; redirects strip the `Authorization` header before following, so a `GITHUB_TOKEN` used for private-repo downloads can't leak to a redirect target.

**Needs live verification (not confirmed statically):** actual Windows behavior renaming/deleting a currently-executing `.exe` (likely `ERROR_SHARING_VIOLATION` on the final cleanup, silently swallowed, leaving a permanent `.old` file — reasoned, not observed); zip-slip/path-traversal protection in the exact `extract_archive` call pattern (the `tar`/`zip` crate versions in use are modern enough to include mitigations, but no crafted-archive test was run against this specific code path).

### 7.2 Storage & graph-invalidation layer (`src/store/*`, `src/graph/*`)

**HIGH — The cycle-prevention guard exists only in the transient in-memory graph, not on the persistent write path, and silently drops edges on reload if a cycle ever gets into the DB.** `GraphEngine::add_derivation_edge` (`src/graph/engine.rs:36-56`) is the only place cycle detection runs, and it's called only while *rebuilding* the graph from the DB (`InvalidationCascade::build_from_db`, `src/graph/invalidation.rs:27`) — never on the actual write path, which is `queries::insert_derivation_edge` (`src/store/queries.rs:150-164`), a raw `INSERT ... ON CONFLICT DO NOTHING` with no cycle/self-loop check and no schema-level constraint (`src/store/schema.rs:43-50`). If a cycle ever lands in `derivation_edges` (e.g. via the ID-collision path in the next finding), `build_from_db`'s reload silently drops whichever edge closes the cycle (`let _ = engine.add_derivation_edge(...)`, no log) — with no guarantee about *which* edge, since the underlying `SELECT`s have no `ORDER BY`. A real, persisted dependency can vanish from the in-memory graph with zero diagnostic trail, meaning a genuinely stale derived node could be reported as fresh. **Fix: enforce the cycle/self-loop check at the DB-insert layer, not only in the transient engine; never discard `add_derivation_edge` errors during reload.**

**Two DB-level supplements, confirmed against SQLite's own docs:** (1) a `CHECK` constraint (`CHECK (parent_node_id <> child_node_id)`) can catch the trivial self-loop case directly in the schema — [sqlite.org/lang_createtable.html](https://sqlite.org/lang_createtable.html) confirms CHECK expressions are evaluated per row on insert/update, with the only stated restriction being that "the expression... may not contain a subquery" (referencing multiple columns of the same row is fine). (2) Full-cycle detection (not just self-loops) needs a query, not a constraint: SQLite's recursive CTE docs ([sqlite.org/lang_with.html](https://sqlite.org/lang_with.html)) show `WITH RECURSIVE` explicitly used with `UNION` (not `UNION ALL`) "to prevent the recursion from entering an infinite loop if the graph contains cycles" — exactly the check that could run as a pre-insert guard query before any new edge is committed.

**HIGH — No `busy_timeout` set on any SQLite connection, and staleness-check errors are discarded — concurrent access can silently produce a false "all fresh" status.** `src/store/db.rs:27` opens the connection with no `busy_timeout`/`busy_handler` call anywhere in the codebase; SQLite's C default is `busy_timeout = 0` (immediate `SQLITE_BUSY`, no retry). `src/cli/status.rs:14` discards the result of `evaluate_and_invalidate_all` outright (`let _ = ...`), and the hook path (`src/hooks/mod.rs`) swallows every DB error by design (fail-open). Two `rgt` processes racing on the same `.rgt/store.db` (e.g. a PreToolUse/PostToolUse pair, or a background agent write racing an interactive `rgt status`) can hit `SQLITE_BUSY` immediately; in `status.rs` this means the freshness re-check can fail entirely and the command reports node counts from *before* the (failed) check — potentially printing "all fresh" when the check never actually ran. **Fix: set a real `busy_timeout` on every connection; stop discarding errors from correctness-relevant calls (fail-open is fine for not blocking the agent, it's not fine for silently mis-reporting state).** Confirmed via [docs.rs/rusqlite](https://docs.rs/rusqlite) — `Connection::busy_timeout(&self, timeout: Duration)`: "Set a busy handler that sleeps for a specified amount of time when a table is locked." **Correction/nuance worth noting:** RGT's schema already enables WAL mode (`PRAGMA journal_mode = WAL`, `src/store/schema.rs:7`) — this isn't a complete absence of concurrency handling. But WAL and `busy_timeout` are complementary, not substitutes: SQLite's own WAL docs ([sqlite.org/wal.html](https://sqlite.org/wal.html)) confirm "writers and readers can run at the same time" under WAL, but also that "there can only be one writer at a time" — so concurrent *writers* (e.g. two hook invocations racing) can still hit `SQLITE_BUSY` even with WAL already on; `busy_timeout` is the piece that's still missing.

**MEDIUM-HIGH — 48-bit truncated node IDs, combined with an `ON CONFLICT` clause that never updates value fields, can silently and permanently corrupt a node's data on a hash collision.** `src/types/node.rs:57-77` truncates a blake3 hash to 12 hex chars (48 bits) for both root and derived node IDs — birthday-bound ~50% collision odds around 2^24 (~16.7M) nodes, plausible for a long-lived, heavily-used project store. `src/store/queries.rs:67-90`'s `ON CONFLICT(id) DO UPDATE` only refreshes `is_stale`/`stale_reason`/`updated_at` — never `number_val`/`date_val`/`duration_secs`/`source_doc_id`/`line_number`. On a collision between two genuinely distinct inputs, the second insert succeeds silently but its real data is discarded; the row keeps the *first* writer's value/`source_doc_id` forever, meaning future invalidation can target the wrong source file entirely. **Fix: widen the ID (128+ bits), and/or make the conflict clause update all value-bearing columns so any legitimate re-record reflects the latest write.** Confirmed via [docs.rs/blake3](https://docs.rs/blake3): the full digest is 32 bytes (256 bits) — truncating to 12 hex chars (48 bits) is a deliberate reduction, not a limitation of the hash itself (blake3's own docs don't discuss truncation/collision tradeoffs directly; the birthday-bound math cited earlier is standard cryptographic reasoning, not sourced from blake3's docs — worth being precise about that). For the conflict-clause half of the fix, confirmed via [sqlite.org/lang_upsert.html](https://sqlite.org/lang_upsert.html): `ON CONFLICT(id) DO UPDATE SET col1=excluded.col1, col2=excluded.col2, ...` is valid, documented syntax — `excluded.<column>` refers to the value that *would* have been inserted, exactly what's needed to make every value-bearing column update on conflict instead of only the three staleness columns it updates today.

**MEDIUM — Hook-driven batch inserts have no transaction (unlike `rgt record`), widening both the partial-failure and lock-contention windows.** `src/cli/record.rs:53-76` correctly transacts its insert loop; `src/hooks/mod.rs:72-88` does the equivalent loop with a bare per-row `insert_tracked_node` call, no transaction, result discarded. A killed process or a lock error partway through a many-value file leaves a partially-recorded, inconsistent state with no signal, and multiplies per-hook-invocation lock acquire/release cycles (compounding the `busy_timeout` finding above, since PreToolUse/PostToolUse pairs fire back-to-back on the same file). **Fix: wrap the extraction loop in one transaction, matching `record.rs`'s pattern.** Confirmed via [docs.rs/rusqlite](https://docs.rs/rusqlite) `Transaction`: if dropped without an explicit `commit()`, "transactions will roll back by default" — safe by construction if an early return happens mid-loop.

**Alternative/complementary fix:** a single multi-row `INSERT INTO t VALUES (...), (...), (...)` statement, confirmed valid SQLite syntax ([sqlite.org/lang_insert.html](https://sqlite.org/lang_insert.html)) — cuts per-statement overhead further than wrapping many single-row inserts in one transaction, though prepared-statement reuse with bound parameters (which the existing loop already does) captures most of that benefit already.

**LOW-MEDIUM — No schema migration path at all.** `src/store/schema.rs` only issues `CREATE TABLE/INDEX IF NOT EXISTS` — no `PRAGMA user_version`, no migration table, no `ALTER TABLE` anywhere in the repo. A future schema change against an existing `.rgt/store.db` breaks every command with "no such column" (new binary, old DB) or a `NOT NULL` violation (old binary, newer DB) — and since RGT ships its own self-update mechanism, a version-skewed update has no graceful degradation path today. **Fix: track schema version via `PRAGMA user_version`**, confirmed via SQLite's own docs ([sqlite.org/pragma.html#pragma_user_version](https://sqlite.org/pragma.html#pragma_user_version)): "The user-version is an integer that is available to applications to use however they want. SQLite makes no use of the user-version itself" — a free, application-owned field suited exactly for this. **Alternative: don't hand-roll it** — the `rusqlite_migration` crate ([docs.rs/rusqlite_migration](https://docs.rs/rusqlite_migration)) is purpose-built for versioned SQL migrations over a `rusqlite::Connection`, applying an ordered slice of migration structs via `Migrations::to_latest()`, and internally uses the same `PRAGMA user_version` primitive — same mechanism, already packaged and tested. **LOW — no garbage collection**: no `DELETE`/`VACUUM` anywhere; every recorded value is a permanent row, which also increases the practical odds of the ID-collision finding above as a project ages. **LOW — `total_invalidated` in `invalidate_file` (`invalidation.rs:52-64`) can double-count a node reachable from two stale ancestors of the same file** — cosmetic (staleness state itself is correctly idempotent), but the printed "blast radius" count can overstate distinct nodes affected.

**Verified sound, no finding:** every query is parameterized (`rusqlite::params!`); grepped for string-formatted SQL (`execute(&format!`, etc.) across `src/` — zero matches, no injection surface. Diamond-shaped dependency cascades and in-memory cycle prevention both work correctly in isolation (`petgraph::visit::Bfs` dedupes visits; existing `tests/unit/test_invalidation.rs` cycle test passes) — the defect is specifically that this guard is disconnected from the persistent write path (first finding above), not that the algorithm itself is wrong.

**Needs live verification:** actual `SQLITE_BUSY` behavior under genuine concurrent processes against the same store; whether WAL mode holds on all filesystems RGT runs on; whether the unordered `SELECT` in `build_from_db` actually produces non-deterministic edge-drop order across repeated runs given a real seeded cycle; real-world collision frequency for 48-bit IDs at production scale.

### 7.3 CLI commands & derivation-verification logic (`src/cli/*`, `src/verify/*`, node ID generation)

**CRITICAL — A deeply-nested `--expression` string crashes the process with a stack overflow (SIGABRT), not a clean error.** `src/verify/expression.rs:38` passes the raw expression straight to `evalexpr::eval_with_context` with no length/depth limit. Evalexpr's evaluator recurses once per nesting level. **Confirmed empirically**: `rgt verify --parents <id> --operation EXPRESSION --expression "$(python3 -c "print('('*200000+'1'+')'*200000)")" --result 1` aborts with "stack overflow" and exit code 134; the same crash reproduces through `rgt derive`. This is process-ending, not a catchable `Err` — a single malformed or adversarial `--expression` argument kills the CLI outright. **Fix: reject expressions above a length/paren-depth threshold before evaluating.**

**Complementary (not substitute) fix:** run evaluation on a dedicated thread with an explicit stack size via `std::thread::Builder::stack_size` — confirmed real and documented ([doc.rust-lang.org/std/thread/struct.Builder.html](https://doc.rust-lang.org/std/thread/struct.Builder.html)): "Sets the size of the stack (in bytes) for the new thread." This only raises the ceiling and isolates the crash to a joinable thread — it doesn't eliminate unbounded recursion, so pair it with the depth-limit validation rather than relying on it alone.

**HIGH — Verification is silently bypassed by a case-mismatched `--operation` value, and `rgt verify` reports success (exit 0) for a wildly wrong result.** `src/verify/mod.rs:59` and the equivalent match in `src/cli/derive.rs:44-67` compare `operation` case-sensitively against exactly `"EXPRESSION"`/`"DATE_DIFF"`; anything else (`expression`, `Expression`, `date_diff`) falls into an "unknown operation, skip" branch. **Confirmed empirically**: `rgt verify --operation expression --expression "a + 999999" --result 1` (a deliberately wrong result) prints a skip warning and exits 0. Given the entire purpose of this command is to gate agent-claimed values, a plausible LLM casing slip silently turns the safety check into a no-op that reports success to any caller checking only the exit code. **Fix: validate `operation` against an allow-list and error (non-zero exit) on anything else, rather than passing through.**

**More idiomatic alternative:** make `--operation` a `clap` `ValueEnum` instead of a raw `String` matched later — confirmed via [docs.rs/clap](https://docs.rs/clap/latest/clap/trait.ValueEnum.html): fields implementing `ValueEnum` with `#[arg(value_enum)]` get automatic, clean, clap-generated errors for unrecognized values, with optional case-insensitive matching available via `from_str`. This moves validation to argument-parsing time (rejecting `expression`/`Expression` before the command even runs its logic) rather than deep inside `derive.rs`/`verify/mod.rs`'s match arms.

**HIGH — Two numerically-identical values on the same source line silently collide onto one node, permanently losing one of them.** `extract_values_from_content` (`src/hooks/parser.rs:76-85`) tags every number on a line with the same `line_number`; `TrackedNode::generate_root_id` (`src/types/node.rs:57-66`) hashes only `file_path + line_number + value` — no column/occurrence discriminator. **Confirmed empirically**: `revenue,120000,discount,120000` on one line records as "2 values" per the CLI's own success message, but `rgt graph` shows only one resulting node — the second occurrence silently merges into the first via the same `ON CONFLICT(id) DO UPDATE` path noted in §7.2, and is never independently trackable again. A control case with two *different* values on the same line correctly produces two nodes, isolating the cause precisely to same-value collision. (Re-recording the same file twice, by contrast, was confirmed to dedupe correctly — that specific worry is unfounded.) **Fix: include a per-occurrence discriminator in the ID hash** — either the column offset of the match within the line, or simpler to implement, a running occurrence counter incremented per value seen during a single `extract_values_from_content` pass. This is a project-specific ID-design choice with no applicable external documentation to cite; either option guarantees uniqueness, the counter is just less code.

**MEDIUM — `verify_expression` uses `.as_float()` instead of `.as_number()`, falsely rejecting correct integer-typed results.** `src/verify/expression.rs:41-44`: evalexpr's `Value::as_float()` does not coerce `Value::Int → Float` (unlike `as_number()`, which does); an expression that never references a bound parent variable (pure literals, e.g. `"2*50"`) stays `Value::Int` and fails with "did not evaluate to a number" even when mathematically correct. **Confirmed empirically**: `--expression "2*50" --result 100` fails verification despite being correct. Narrow (only affects zero-parent-reference expressions) but a genuine correctness bug. **Fix: use `.as_number()`.** Confirmed directly against [docs.rs/evalexpr](https://docs.rs/evalexpr/latest/evalexpr/enum.Value.html) `Value` enum docs: `as_float` "returns `Err` if `self` is not a `Value::Float`" (no coercion), while `as_number` "returns `Err` if `self` is not a `Value::Float` or `Value::Int`. Note that this method silently converts `IntType` to `FloatType`, if `self` is a `Value::Int`" — confirming the fix exactly. No second fix is offered: `.as_number()` is the documented, correct API; a hand-rolled `match` on `Value::Int`/`Value::Float` would just reimplement what it already does internally.

**MEDIUM — evalexpr's full builtin function set is exposed to `--expression`, beyond the intended `+ - * / % ^` contract — including `random()`, which breaks verification determinism.** Verified by reading `evalexpr`'s source: RGT never calls `context.set_builtin_functions_disabled(true)`, so `random()`, `str::regex_matches/replace`, `math::*`, `min`/`max`/`if`/`bitand`/`typeof`, etc. are all callable. `random()` in particular can make an expression evaluate differently across invocations, undermining the entire "recompute and check against parent values" verification model; the regex functions accept arbitrary caller-supplied patterns nobody intended to expose. **Fix: call `context.set_builtin_functions_disabled(true)` so only bound variables and arithmetic operators are usable.** Confirmed on evalexpr's `Context` trait (docs.rs/evalexpr): "Disables builtin functions if `disabled` is `true`, and enables them otherwise."

**Alternative fix:** implement a custom, minimal `Context` rather than toggling builtins off a default `HashMapContext` — evalexpr ships `EmptyContext` (builtins permanently disabled, cannot be re-enabled) as a documented example of exactly this kind of restricted context. Less explicitly documented as a "how-to" than the toggle method, but the trait's minimal required surface supports it.

**LOW — Node IDs truncated to 48 bits** (same underlying issue as §7.2's ID-collision finding and its blake3/SQLite UPSERT citations, from the CLI-side perspective). **LOW — `DATE_DIFF`'s parent-order convention (`date2 - date1`) is documented but not enforced**; a consistently-swapped order passes verification with the opposite sign convention silently, a false positive not caught by any runtime check. **Possible fix:** require explicit named flags (`--date1`/`--date2`) instead of positional parent order, removing the ambiguity entirely — a standard `clap` named-argument pattern ([docs.rs/clap](https://docs.rs/clap)), not a novel idea, though it changes the CLI's existing `--parents` interface, so this is flagged as needing product judgment rather than a drop-in fix. **LOW — unchecked `u8` arithmetic for parent variable naming beyond 26 parents** (`src/verify/expression.rs:27`) panics in debug builds past index 159, silently wraps in the shipped release binary (non-exploitable as shipped, but unchecked arithmetic on an unbounded, caller-controlled count). **Possible fix:** bound-check the parent count explicitly (clean error past 26 parents), or enable `overflow-checks = true` for release builds too — documented in the Cargo Book ([doc.rust-lang.org/cargo/reference/profiles.html#overflow-checks](https://doc.rust-lang.org/cargo/reference/profiles.html#overflow-checks)) — trading a small performance cost for a deterministic panic in all build profiles instead of silent wraparound in release.

**Verified sound, no finding:** nonexistent node/parent IDs return clean errors, no panics; `rgt query`'s lineage BFS uses a visited-set guard, immune to cycles; `rgt graph`'s DOT/Mermaid export is non-recursive and immune to cycle/injection concerns since edge operation-type strings are always one of two fixed literals; evalexpr's arithmetic uses checked operations (clean `Err` on overflow/div-by-zero, not a panic); re-recording an identical file twice dedupes cleanly; the `approx_eq` float tolerance (`1e-9` absolute / `1e-12` relative) looks reasonable across realistic magnitudes, though a dedicated property-based test would close this out more rigorously than manual reasoning did.

### 7.4 Change-detection layer (`src/detection/tier1.rs`, `tier2_blake3.rs`)

**HIGH — Tier 2 (content hashing) is architecturally unreachable for catching a Tier 1 false negative — it only ever demotes a false *positive* back to unchanged, never confirms a false "unchanged."** `src/detection/mod.rs:35-37`: `if !tier1_check_changed(...) { return DetectionResult::Unchanged; }` — the hash is computed only *after* Tier 1 already says "changed." If mtime+size happen to match stored values while content genuinely differs (coarse-mtime filesystems: NFS/SMB mounts, FAT32/exFAT, some Docker bind mounts; or any same-second, same-length rewrite — e.g. `git checkout`/`stash pop` restoring a prior byte-identical-length version), the file is trusted as unchanged indefinitely and dependent graph values never go stale despite a real content change. This is the same risk tier1.rs's own doc comment flags for non-Unix platforms, but the exposure isn't platform-limited — it's inherent to the branch structure. **Fix: don't let Tier 1 alone confirm "unchanged" unconditionally — periodically re-hash regardless of Tier 1's verdict, or detect low mtime resolution and fall back to hashing.**

**Architecturally different alternative:** replace polling mtime/size with OS-level file-change notifications via the `notify` crate ([docs.rs/notify](https://docs.rs/notify)) — confirmed real, platform-native backends: inotify (Linux), FSEvents (macOS, default), ReadDirectoryChangesW (Windows), kqueue (BSD), with `recommended_watcher()` auto-selecting the best one per platform and a polling fallback everywhere else. This sidesteps the mtime-resolution problem entirely for files actively watched, rather than working around it.

**HIGH — TOCTOU: the content used for value extraction and the metadata/hash later stored as its "fingerprint" are read via separate, unsynchronized filesystem operations, both in the CLI path and internally within the detection check itself.** `src/cli/record.rs:20-33` / `src/hooks/mod.rs:40-56`: content is read once for extraction, then the file is independently re-opened for metadata and again for hashing — nothing confirms the file didn't change in between, so the persisted `(mtime, size, hash)` triple can describe different bytes than what was actually extracted and recorded. Separately, `src/detection/mod.rs:30-42` has the same gap internally: `get_metadata_snapshot` and `compute_blake3_hash` are two independent reads with no re-check, so the `Changed` result returned can pair a metadata snapshot from time T1 with a hash from time T2 that never coexisted on disk. **Fix: hash the same in-memory buffer used for extraction rather than re-reading the file separately for hashing.** Confirmed: `blake3::Hasher::update(&mut self, input: &[u8])` ([docs.rs/blake3](https://docs.rs/blake3)) accepts an in-memory byte slice directly — "Add input bytes to the hash state. You can call this any number of times" — so bytes already read for extraction can be hashed with no second file open.

**Alternative fix:** open the file once and use that single handle for both the read and the metadata stat, rather than reopening by path for each. `std::fs::File::metadata(&self)` ([doc.rust-lang.org](https://doc.rust-lang.org/std/fs/struct.File.html#method.metadata)) is an instance method on an already-open handle — "Queries metadata about the underlying file" — distinct from `fs::metadata(path)`, which reopens by path. This narrows, though doesn't eliminate, the TOCTOU window, since no second `open()` occurs for the stat.

**MEDIUM — `FileNotFound` conflates any I/O error (permissions, lock, path-replaced-by-directory) with genuine deletion, mislabeling the stale reason surfaced to the agent.** `src/detection/mod.rs:30-33, 39-42` collapse all `Err(_)` into `DetectionResult::FileNotFound`, consumed at `src/graph/invalidation.rs:113-115` as `stale_reason = "FILE_DELETED"`. A transient permission error or a Windows file lock gets the same "deleted" label as an actual deletion. **Fix: distinguish `io::ErrorKind::NotFound` from other I/O errors via `io::Error::kind()`; treat other errors as "check failed, state unchanged," not "deleted."** Confirmed: `std::io::ErrorKind` ([doc.rust-lang.org](https://doc.rust-lang.org/std/io/enum.ErrorKind.html)) documents `NotFound` ("An entity was not found, often a file") as distinct from `PermissionDenied` and others (the enum is `#[non_exhaustive]`, so new kinds can appear); `Error::kind()` ([doc.rust-lang.org](https://doc.rust-lang.org/std/io/struct.Error.html#method.kind)) is the standard, idiomatic way to inspect it. No genuinely different second fix exists here — this is the one correct approach, not a design choice among alternatives.

**LOW-MEDIUM — Symlink target swaps are invisible whenever the new target coincidentally matches the stored mtime+size** — same root cause as the first finding above, but deliberate/easy to construct via a symlink repoint rather than requiring accidental timing. **Fix: record `st_dev`/`st_ino` of the resolved target as part of the fingerprint** — confirmed via `std::os::unix::fs::MetadataExt` ([doc.rust-lang.org](https://doc.rust-lang.org/std/os/unix/fs/trait.MetadataExt.html)): `ino()` ("Returns the inode number") and `dev()` ("Returns the ID of the device containing the file"). **Alternative/complementary fix:** separately capture the symlink's own (unresolved) metadata via `std::fs::symlink_metadata` ([doc.rust-lang.org](https://doc.rust-lang.org/std/fs/fn.symlink_metadata.html): "Queries the metadata about a file without following symlinks") as a distinct signal for "the symlink itself was repointed," rather than only ever recording the target's identity.

**LOW — path-identity assumptions outside `detection/` proper undermine its guarantees**: `source_documents` is keyed on a byte-exact `file_path` string (`src/store/queries.rs:35-36`); on case-insensitive filesystems (default macOS/Windows), a file recorded under one casing and reported under another via a different tool/hook creates two independent, unsynchronized tracking rows for the same file. **Fix: canonicalize paths before using them as a storage key** — but note a real limitation confirmed directly from Rust's own docs: `std::fs::canonicalize` ([doc.rust-lang.org](https://doc.rust-lang.org/std/fs/fn.canonicalize.html)) resolves symlinks and produces an absolute path, but its documentation says nothing about case-folding — it does **not** solve the case-insensitive-filesystem collision on its own. **Better alternative:** the `same-file` crate ([docs.rs/same-file](https://docs.rs/same-file)) is purpose-built for this — "provides a safe and simple cross platform way to determine whether two file paths refer to the same file or directory" via `is_same_file()`/`Handle`, comparing "inode numbers on Unix and a combination of identifier, volume serial, and file size on Windows," i.e. actual on-disk identity rather than path strings, which correctly treats two differently-cased paths pointing at the same file as identical. **LOW — no test coverage of the tier1/tier2 algorithm itself**: despite its name, `tests/unit/detection.rs` actually tests agent-installation detection (`hooks::installer::detect_installed_agents`), an unrelated function; none of `tier1_check_changed`, `get_metadata_snapshot`, `compute_blake3_hash`, or `evaluate_file_change` appear anywhere in the test suite, so none of this section's findings have any regression guard.

**Needs live verification:** actual mtime resolution on the filesystems RGT is deployed against (whether nanosecond fields are genuinely populated vs. silently rounded); whether same-tick, same-size rewrites are practically achievable on real target platforms; Windows symlink/junction-following behavior versus the Unix behavior traced here.

### 7.5 Updated priority ranking (supersedes §5 for these four areas)

1. **§7.1 checksum-bypass (CRITICAL)** and **§7.3 stack-overflow crash (CRITICAL)** — both are trivially triggerable (one CLI arg; one missing release asset) and both were confirmed empirically, not just reasoned about. Fix before anything else in this sweep.
2. **§7.2 cycle-guard bypass + missing `busy_timeout`** and **§7.1 signature verification / semver downgrade** — all four undermine the tool's core correctness/security promise (trustworthy provenance graph; safe self-update) under realistic, non-exotic conditions.
3. **§7.3 case-sensitive operation bypass** and **§7.3 same-line value collision** — both confirmed empirically, both silently defeat the specific guarantee (verification; distinct value tracking) the affected command exists to provide.
4. **§7.4 Tier 1/Tier 2 architecture gap and TOCTOU** — real, but requires specific filesystem/timing conditions rather than firing on ordinary usage.
5. Remaining MEDIUM/LOW items across all four areas — schema migrations, ID-width, transaction coverage, error-kind conflation, evalexpr builtin exposure — worth a cleanup pass but none are independently critical.

---

## 8. CONFIRMED: Bash-command reads (`cat`/`head`/`grep`) are invisible to RGT's hook for Claude Code/Cursor — and RTK, RGT's own stated inspiration, makes this worse when both are installed together

Prompted by comparing RGT against RTK (github.com/rtk-ai/rtk, the project RGT's README explicitly credits as its inspiration and mirrors for 13-agent coverage). RTK's own docs state plainly: "the hook only runs on Bash tool calls. Claude Code built-in tools like `Read`, `Grep`, and `Glob` do not pass through the Bash hook." That's the mirror image of a gap worth checking in RGT — and it's real, confirmed with a live captured payload rather than inferred from the match statement alone.

### 8.1 The captured payload

A debug hook (matcher `"Bash"`, project-scoped `.claude/settings.local.json`, removed after the test) captured the real `PostToolUse` event for a plain `cat test_invoice.txt` command run in this session:

```json
{
  "tool_name": "Bash",
  "tool_input": {
    "command": "rtk read /path/to/test_invoice.txt",
    "description": "..."
  },
  "tool_response": {
    "stdout": "Invoice Report\nAmount: 1234.56\nDue Date: 2026-01-15\nQuantity: 42",
    "stderr": "",
    "interrupted": false,
    "isImage": false,
    "noOutputExpected": false
  }
}
```

Two things confirmed at once:

1. **`tool_input`/`tool_response` for a Bash call have a completely different shape than RGT's `default_dialect` expects.** `HookToolInput { path, file_path }` (`src/hooks/parser.rs:6-10`) has nothing to bind — the real field is `command`. `HookToolResponse { content }` (`:12-15`) has nothing to bind either — the real fields are `stdout`/`stderr`. Both come back `None`; `handle_passive_hook_event` (`src/hooks/mod.rs:31-33`) exits immediately on `capture.path.as_deref()` being `None`. **Every `cat`/`head`/`tail`/`grep` read a Claude Code or Cursor agent runs via Bash is a complete, silent blind spot in RGT today** — confirmed, not inferred.
2. **RTK's own PreToolUse hook rewrote the command live, in this exact session, before it executed** — I typed `cat test_invoice.txt`; the payload shows `tool_input.command: "rtk read /path/..."`. This is RTK's documented "Auto-Rewrite" mechanism from §7's doc check, caught in the act.

### 8.2 RGT already has the fix half-built — it's just not wired to the two agents most likely to need it

`command_dialect`/`extract_path_from_command` (`src/hooks/parser.rs:159-205`) already exists and already parses exactly this command shape (`tool_input.command`, extracting the path argument from `cat`/`read`/`less`/`head`/`tail`/`more`/`python`/`python3` invocations) — but per the dispatch table in `normalize_agent_event` (`:99-107`), it's wired only to `gemini`/`vibe`/`opencode`/`pi`/`hermes`. `claude-code` and `cursor` — arguably the two most widely used agents in RGT's own supported list, and the two using `default_dialect` — get none of this. This isn't a "maybe some agents miss it" gap; it's the two flagship integrations.

### 8.3 The compounding problem: even wiring this up wouldn't be enough as-is, because of RTK specifically

`extract_path_from_command` (`:181-183`) checks only the **literal first token** of the command against a fixed allow-list: `["cat", "read", "less", "head", "tail", "more", "python", "python3"]`. With RTK installed and auto-rewriting Bash commands — which, per §8.1, is demonstrably happening in this very session, and which RGT's own README explicitly sets up as the expected complementary pairing ("RGT was inspired by RTK... mirrors its 13-agent coverage") — the real first token becomes `rtk`, not `cat`. `rtk` is not in the allow-list, so `extract_path_from_command` would return `None` even if `command_dialect` were wired into `claude-code`/`cursor`. **Fixing the dialect wiring alone is not sufficient; the parser also needs to recognize `rtk <verb> <path>` (and potentially other wrapper-proxy patterns) and extract the path from behind the wrapper**, not just from a bare read command.

### 8.4 Suggested fix

1. Extend `normalize_agent_event`'s dispatch so `claude-code`/`cursor` also fall back to command-based extraction when `default_dialect` yields no path/content — i.e., try `default_dialect` first (covers the native `Read` tool), then `command_dialect` as a fallback for `Bash` tool calls, rather than the current either/or per-agent dispatch. Grounding: the same Claude Code hooks reference cited in §2.6/§4 confirms `tool_name` is present on every event, so branching on it (Bash vs. Read) before choosing a dialect is a documented, available signal, not a guess.
2. Extend `extract_path_from_command`'s recognized-binary matching to see through known wrapper proxies: if the first token is `rtk`, re-check the *second* token against the existing `READ_STYLE` list before extracting the path argument. **Correction from the initial draft:** RTK does not have a separate `rtk cat` subcommand — per RTK's own README (fetched in this session), its file-reading subcommand is named `read` (`rtk read <file>`), and it *rewrites* both `cat` and `read` shell invocations to that one subcommand; `grep`/`find`/`ls` map to their own like-named `rtk` subcommands. So the check specifically needs `rtk read` (not `rtk cat`) recognized as a read-style command behind the wrapper. This is project-specific glue logic; no external documentation applies beyond RTK's own README, already cited in §7/§8.
3. For `tool_response`, add a Bash-shaped variant to the normalized capture (`stdout` as content, when present and non-empty) alongside the existing `content` field, so a captured `cat`/`rtk read` output can be used directly rather than always falling back to a disk re-read. (No external citation needed — this mirrors the already-documented `stdout` field confirmed in the §8.1 captured payload.)
4. This keeps RGT's own stated design promise intact — nothing here rewrites, filters, or blocks the agent's command; it only widens what RGT's *passive* observation recognizes as a read.

### 8.5 RESOLVED (discarded): RTK's "signatures over full bodies" does not threaten RGT's numeric extraction for data files

The open question from the initial draft — whether RTK's summarization could make numbers invisible to RGT even after fixing §8.2–§8.4 — was tested directly against the live `rtk` binary running in this session (not inferred from docs). Four tests, `rtk read` at each documented filter level (`none` = default, `minimal`, `aggressive`) against files real agents would actually read:

| Test file | Ground-truth lines | `rtk read` default | `rtk read -l minimal` | `rtk read -l aggressive` |
|---|---|---|---|---|
| 3,000-row CSV, random revenue/cost/date columns | 3001 | **3001 (full)** | 3000 (full rows intact) | 3000 (full rows intact) |
| 500-line repetitive log, distinct `$amount` per line | 500 | **500 (full)** | — | 499 (all distinct values intact) |
| 200 truly identical lines + 1 distinct final line | 201 | **201 (full)** | — | 200 (all values, including the distinct one, intact — no dedup collapsing) |
| 60,000-row / ~1MB CSV, via the **live auto-rewrite hook** (plain `cat`, not an explicit `rtk read` call) | 60001 | **60001 (full)** | — | not tested (see below) |
| 13-line Rust source file with numeric literals in function bodies | 13 | full | not tested | **bodies replaced with `// ... implementation`, some numeric literals inside collapsed branches lost** |

Conclusion: **discarded as a practical risk for RGT's actual use case.**
- The live auto-rewrite hook — the mechanism that actually fires when an agent runs plain `cat`/`head` — never appends a `-l` flag; it always uses the default level, confirmed in both the original §8.1 capture and the 60,000-row test here. Full content, at any size tested (up to ~1MB / 60k rows), passes through unmodified.
- Signature/body stripping is real, but confirmed to apply **only to recognized source-code files** (the Rust test), never to CSV/log/plain-text data files — those pass through with every row and every value intact even when `-l aggressive` is forced explicitly, which is itself not what the auto-rewrite hook does by default.
- No dynamic escalation to `minimal`/`aggressive` based on file size was observed up to the largest size tested here (~1MB).

Residual, narrower risk not covered by this test: an agent that explicitly chooses `rtk read -l aggressive`/`-l minimal` itself (not the default auto-rewrite path) on a file RTK recognizes as source code containing embedded numeric constants (e.g. a hardcoded tax rate or discount multiplier in a `.rs`/`.py`/`.ts` file) could lose that specific value. This is a narrow, opt-in edge case, not a default-path concern, and arguably out of scope for RGT either way — RGT's own regex-based extraction targets data files, not numeric literals buried in source code logic.
