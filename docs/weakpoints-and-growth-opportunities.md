# RGT: Remaining Weak Points & Growth Opportunities

**Status:** Research report — for maintainer review, not yet turned into specs
**Scope:** Everything NOT covered by prior code-correctness reviews (`docs/findings-hooks-and-value-extraction.md`, `docs/verification-of-fixes-develop.md`) — distribution/CI, documentation accuracy, competitive positioning, and concrete feature candidates for adoption.
**Method:** Four parallel audits — two grounded in this repo's actual code/docs/CI (with commands actually run, not just files read), two external research passes (competitive landscape, engineering feasibility) using live web research and real crate/SDK verification.

---

## 1. Executive summary

The application logic has already been through two hardening passes and is in good shape. What's left is almost entirely in **distribution, documentation freshness, and go-to-market** — none of it is a correctness bug, but several items are real adoption blockers a new user would hit in their first five minutes.

**Top 5, ranked by impact:**
1. **`rgt` is already taken on crates.io** by an unrelated crate — `cargo install rgt` (the instinctive first move for any Rust user) either fails or installs the wrong tool.
2. **Intel Macs are silently unsupported end-to-end** — `install.sh` computes a target triple that was never built in CI, so it just 404s with a generic error.
3. **Non-English users have no path to discover `rgt doctor`, `rgt gc`, or `--number-format`** — all 6 translated READMEs are frozen at a pre-hardening-pass snapshot, and the *installed* agent-instructions text (`AGENTS.md`/`.clinerules`/etc.) for 6 of the 13 agent integrations is similarly stale, independent of language.
4. **A real CLI/doc mismatch**: `rgt query`'s `--json` flag does nothing — it's always JSON, contradicting both the README and the function's own doc comment.
5. **The single highest-leverage feature to build is an MCP server mode** — it directly targets the fragility already documented in the hooks findings report (undocumented per-agent payloads, guessed schemas), and MCP is now the dominant 2026 integration standard with a real, official Rust SDK (`rmcp`) available.

---

## 2. Distribution & CI weak points

| # | Finding | Impact | Fix |
|---|---|---|---|
| 1 | **crates.io name collision** — `rgt` there is an unrelated "red/green syntax tree library" (github.com/KeqingMoe/rgt). This project isn't published to crates.io at all; README correctly routes around it with `cargo install --git`. | High — blocks the standard Rust discovery path entirely. | Publish under a distinct name (`rgt-cli`, `rust-graph-tracker`) and document the collision explicitly, mirroring how RTK's own README already handles its "Rust Type Kit" collision. |
| 2 | **Intel Mac unsupported end-to-end, silently.** `release.yml`'s build matrix only produces `aarch64-apple-darwin`. `install.sh`'s arch detection still computes `x86_64-apple-darwin` for an Intel Mac and 404s with a generic "Download failed." `Formula/rgt.rb` has no Intel branch either. | High for any Intel Mac user (still common in enterprise fleets). | Either build x86_64 macOS in CI, or make `install.sh` explicitly detect and reject Intel Mac with a clear message, the way it already does for genuinely unsupported platforms. |
| 3 | **Version/release-please state has desynced from reality.** `Cargo.toml`/tags are at `0.5.0` (a manual "chore: bump version" commit), but `.release-please-manifest.json` still says `0.4.0`, `CHANGELOG.md` has only the `0.4.0` entry, and `Formula/rgt.rb` is stuck at `0.4.0` too. Every feature from specs 019–029 (`doctor`, `gc`, `--number-format`, attestations, etc.) has no changelog entry anywhere. | High — undersells the last month of real work, and risks release-please misbehaving (duplicate/wrong release PR) on its next run. | Reconcile the manifest and backfill the changelog now; stop hand-bumping the version outside release-please's own flow going forward. |
| 4 | **A CI job that looks like a quality gate is a no-op.** `ci.yml`'s "Test Presence Check" job runs on every PR, posts a summary, and its actual body is `echo "Skipped - add custom check logic as needed."` — it always passes. | Medium — false confidence is worse than an absent gate. | Implement it for real or delete it. |
| 5 | No code-coverage tooling/badge anywhere (`cargo llvm-cov`/`tarpaulin`, Codecov, etc.). | Medium — matters more than average given the domain is numeric correctness. | Add `cargo llvm-cov` to CI + a README badge. |
| 6 | GitHub Actions are pinned by tag, not commit SHA, in jobs holding `contents: write`/`id-token: write`/`secrets: inherit`. `pr-target-check.yml`'s `pull_request_target` usage itself is safe (no PR-head checkout). | Low-medium — standard supply-chain hardening, not an active exploit. | Pin third-party actions to commit SHA. |
| 7 | `install.sh` is curl-pipe-to-sh with no signature/checksum check **on the installer script itself** (only the downloaded binary archive is verified). It does handle checksum mismatch, unsupported OS/arch, and permission errors gracefully with clear messages/exit codes. | Low-medium — a known, criticized pattern broadly, though the binary itself is still verified. | Consider a documented "verify the script's own hash" step, or accept this as a known tradeoff (many tools do) and move on. |
| 8 | No Docker image, `.msi`/winget/scoop package. Windows users get only a raw `.zip`; Windows CI/build/test does exist and pass, so this is a packaging gap, not a support gap. | Low. | Consider winget (lowest effort, standard Windows discovery path) if Windows adoption becomes a priority. |
| 9 | `.deb`/`.rpm` packaging (`cargo deb`/`cargo generate-rpm`) is real and wired into `release.yml`, not inert metadata — no action needed, listed here only to correct an assumption worth double-checking before assuming it's a gap. | — | None needed. |

---

## 3. Documentation accuracy

| # | Finding | Impact | Fix |
|---|---|---|---|
| 1 | **All 6 translated READMEs are frozen at a pre-hardening-pass snapshot** — identically 221 lines (vs. English's 278), zero mentions of "doctor," "gc," "number-format," or "attestation" across all six. This isn't uneven drift; it's a clean split — English was updated, the rest weren't, at all. | High — non-English users have no localized path to discover the exact tool (`rgt doctor`) built to diagnose the most common failure mode. | Re-sync all 6 translations against current English README. |
| 2 | **`AGENTS.md` (and the actual glue text RGT installs for 6 of 13 agents) has diverged from reality — verified by actually running `rgt init` and diffing the output.** `CLINE_RULES`, `ANTIGRAVITY_RULES`, `KILO_RULES`, `VIBE_PROMPT`, `COPILOT_CLI_RULES`, and `codex_section()` all still only describe `record`/`derive`/`status`/`query`/`graph` — none mention `doctor`, `gc`, or `--number-format`. The non-text-source instruction block (`rgt record --stdin`) *is* correctly synced everywhere. | High — this is what real users actually get installed into their projects; they never learn these features exist regardless of which README they read. | Add the missing feature mentions to each glue constant in `src/hooks/glue/mod.rs` and `installer.rs`'s `codex_section()`. |
| 3 | **`CHANGELOG.md` was never updated for the `0.5.0` release** — same root cause as CI finding #3 above. | High (duplicate of the CI finding, listed here for completeness). | Same fix. |
| 4 | **Real CLI/doc bug, not just a doc issue: `rgt query <id> --json` always prints JSON.** Both README and the command's own `--help`/doc comment frame `--json` as opt-in with an implied plain-text default. Confirmed by running it: `src/cli/query.rs` calls `serde_json::to_string_pretty` in both branches — the only difference is an outer `{"lineage": ...}` wrapper. | Medium — a genuine behavior/documentation mismatch a user will notice immediately. | Either implement a real plain-text lineage renderer for the non-`--json` path, or fix the docs/help text to state the actual behavior (always JSON, `--json` only changes the wrapper). |
| 5 | `CONTRIBUTING.md`'s project-structure diagram and test-category examples are stale — missing `doctor.rs`/`gc.rs` in the file listing, and missing 6 real test files (`hook_editors.rs`, `store_integrity.rs`, `test_update_attestation.rs`, `test_update_semver.rs`, `test_doctor_contract.rs`, `test_store_maintenance_contract.rs`) from the examples. | Low-medium — undersells test coverage to prospective contributors. | Update the file listing and examples. |
| 6 | Everything else checked out cleanly: the English README's Quick Start commands were run verbatim in a sandboxed scratch project and behaved exactly as documented; every subcommand's `--help` output matches the README's Commands table flag-for-flag. | — | None needed. |

---

## 4. Competitive landscape & positioning

**No direct shipped competitor exists.** The closest analogs are research work, not tools: **Proof-Carrying Numbers** (arXiv:2509.06902) is a protocol for tying numeric spans to verified claims, conceptually close to RGT's "verify before recording" model but never shipped as a CLI. Microsoft Research's **VeriTrail** (Jan 2026) does per-step provenance tracing for multi-step AI workflows — same instinct as RGT's staleness cascade, positioned as a research system. **Guardrails AI**, **TruLens**, and **LlamaIndex**'s eval tooling solve a *different* version of this problem — probabilistic groundedness/hallucination scoring against retrieval context, not deterministic re-computation of a specific derived number — and they target RAG/chatbot pipelines, not coding-agent CLI workflows. RGT's combination (persistent local provenance graph + mathematical re-verification + staleness cascading + fully offline) appears to be a genuinely underserved niche, not a crowded one. The actual risk is that "AI hallucinates numbers" gets solved piecemeal by tools users already have open (Guardrails, LlamaIndex) before they ever hear about RGT — which is a distribution problem, not a differentiation problem.

**MCP (Model Context Protocol) is now the dominant integration standard.** 10,000+ public servers by Dec 2025, adoption by OpenAI/Google DeepMind/Microsoft/Cursor/VS Code, donated to the Linux Foundation's Agentic AI Foundation for vendor-neutral governance in Dec 2025. The "CLI ships an MCP server mode alongside its native interface" pattern is already established — Codex CLI itself ships this way. Given RGT's own findings doc documents several of its 13 hook integrations as relying on guessed/undocumented payload formats, an MCP server mode is a credible way to replace 13 bespoke, fragile integrations with one standard, documented surface.

**Adoption playbook, grounded in a real comparable.** RTK — this environment's own inspiration, explicitly credited in RGT's README — solo-launched Jan 2025 and reached 70k+ GitHub stars via: a sharp, quantifiable one-line pitch ("cuts your AI bill 60-90%"), two staged HN launches, a single static Rust binary with zero deps (low install friction), and organic pickup by third-party blog posts repeating the core stat. Curated "awesome-X" lists (e.g. `awesome-claude-code`, 36.8k stars) require ~1,000+ stars for inclusion — traction has to come first from HN/Reddit/a sharp demo, not from list placement.

**Real, citable messaging material already exists.** KPMG's Oct 2025 agentic-AI report had only 5 of 45 citations accurate; PwC published "thought leadership" reports later found to be effectively AI-generated with fabricated claims; Deloitte partially refunded an Australian government report over fabricated references; documented AI-hallucination court filings grew from 25 (early 2025) to 1,300+ by April 2026, with sanctions up to $15,000/attorney. These are concrete, sourced incidents RGT can position directly against ("Deloitte refunded a report over fabricated numbers — here's a tool that makes that structurally impossible for your own agent-generated reports").

**Recommendations, ranked by impact vs. effort:**
1. Ship an MCP server mode (see §5 below) — highest leverage, fixes a documented weakness *and* rides the standard every 2026 coding tool speaks.
2. Launch on HN/Reddit using the Deloitte/PwC/KPMG incidents as the hook, with a short demo GIF showing a caught arithmetic error — high impact, low effort.
3. Get listed in `awesome-mcp-servers`/`awesome-claude-code` once there's real traction (chicken-and-egg — do this after #2).
4. Publish a short comparison note vs. Guardrails AI/TruLens ("deterministic re-verification of a specific number" vs. "probabilistic groundedness scoring") to pre-empt the "isn't this just Guardrails?" objection.
5. Consider focusing marketing/demo polish on one flagship integration (Claude Code, given ecosystem size) rather than claiming parity across all 13 — a stronger, more credible demo than "works everywhere, sort of."

---

## 5. Concrete feature candidates, ranked by feasibility × impact

Grounded in the actual current codebase, not generic suggestions.

| # | Feature | Effort | Adoption impact | Verdict |
|---|---|---|---|---|
| 1 | **MCP server mode** — expose `record`/`derive`/`verify`/`query`/`status` as MCP tools via `rmcp` (a real, official Rust MCP SDK confirmed on crates.io) | Medium — core logic (`execute_record`, `verify::verify`, `query::query_provenance`, etc.) is already reasonably decoupled from clap/`println!`, needs a pull-apart-and-wrap pass across ~5-6 files, not a rewrite | High — directly targets the undocumented-payload fragility already documented in the hooks findings report; one implementation instead of 13 | **Build now** |
| 2 | **GitHub Action** wrapping `rgt status --json` as a CI gate on tracked data files | Small — mostly a composite-action script downloading the existing release binary; `execute_status` already has a `--json` branch (though it currently always returns `Ok(())` regardless of staleness — would need a `--fail-on-stale`-style flag, ~20 lines) | Medium — real friction: only helps teams that commit `.rgt/store.db` to the repo, which isn't an established workflow anywhere in the project's own docs | **Build later**, once that workflow question is resolved |
| 3 | One more built-in verify operation (beyond `EXPRESSION`/`DATE_DIFF`) | Small, if it's one more `match` arm mirroring `date_diff.rs`'s shape; a true user-pluggable system would need an embedded scripting engine or `dlopen`-based plugins — real new architecture, not an extension | Low-medium — `EXPRESSION` already covers most "custom formula" demand | **Build later**, only the small built-in-op case, on concrete demand — **don't build** a general plugin system |
| 4 | Local web dashboard for the provenance graph (`rgt graph --serve`) | Medium-large — no HTTP server crate or JS graph library exists in the dependency tree today; adding either works against the "single lightweight offline binary" pitch (`[profile.release]` already optimizes hard for binary size) | Low-medium — GitHub already renders Mermaid natively in markdown, covering most "I want to see it" demand already | **Don't build now** |
| 5 | Multi-project/workspace awareness | Small (a `--project` override flag) to Medium (real cross-DB aggregation, which reopens the ID-collision risk already flagged in the storage findings) | Low — the tool's actual use case is single-project; a monorepo already gets one shared graph for free by running `rgt init` at the root | **Don't build** |

---

## 6. Suggested next step

Everything in §2 items 1-4 and §3 items 1-4 is small, mechanical, and independently fixable — a natural next spec-plan batch (in the same `019`-`032` style already established) if you want these turned into specs the way the verification follow-ups were. The MCP server mode (§5 #1) is the one substantial engineering investment worth scoping as its own dedicated spec given its outsized adoption leverage.
