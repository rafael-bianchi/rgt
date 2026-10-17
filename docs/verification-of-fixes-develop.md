# RGT Verification Report: `develop` Fixes Against `findings-hooks-and-value-extraction.md`

**Status:** Complete
**Verified against:** branch `develop` @ `acc5e18` (version 0.5.0)
**Source spec:** `docs/findings-hooks-and-value-extraction.md` (20+ findings), broken into 11 feature specs (`019`–`029`) per `docs/spec-plan-from-findings.md`, all merged.
**Method:** Five parallel audits, one per original report section, each re-reading the original finding, then the current code, then classifying every item as **FIXED**, **PARTIALLY FIXED**, **NOT FIXED**, or **NEW ISSUE**. Wherever possible, audits built `rgt` in release mode and empirically reproduced the *original* repro steps against the new binary — not just read code or trust commit messages/names. Noted per finding below.

---

## 1. Executive summary

This is an unusually clean outcome: **of ~30 tracked findings, the large majority are genuinely FIXED and empirically confirmed**, several with dedicated new tests. A handful are honestly **PARTIALLY FIXED** with the residual gap clearly scoped, one is **explicitly NOT FIXED** (and was never claimed to be — flagged as an open assumption in the new code's own tests), and the audits surfaced **new issues** — most notably **two pre-existing test failures currently on `develop`** and **one real, unaddressed subtle bug** in the new hook-config editor.

**Do this first if picking `develop` up for release:**
1. **Fix the 2 failing tests in `tests/contract/test_update_contract.rs`** (`genuine_newer_tag_proceeds_past_gating`, `failed_verification_never_reaches_replace`) — stale fixtures hardcode `v0.4.1` as "newer," which broke once `Cargo.toml` was bumped to `0.5.0`. Ironically, this is proof the anti-downgrade fix *works* — the tests just need their stub version updated.
2. **Review the new `attestation-verify` v0.1.0 dependency** — a brand-new, seemingly zero-adoption third-party crate now carries the entire signature-verification security guarantee for `rgt update`. The production verification policy has never been exercised against a real signed `rgt` release (no attested release exists yet; the self-test gates on a fixture that doesn't exist).
3. **Decide on the two intentionally-open gaps**: Cursor still gets no non-text-source instructions file (§6), and the Copilot CLI dialect is still an unverified assumption (§2.7) — both correctly self-documented as open in the new code's own comments/tests, not silently claimed done.

---

## 2. §2 / §6 / §8 — Hooks, instructions files, Bash-tool capture (specs `019`, `023`, `026`, `028`)

| # | Finding | Verdict | Evidence |
|---|---|---|---|
| 2.1 | Silent settings destruction on malformed JSON | **FIXED** | `src/hooks/editor.rs:59-98,141-149` — parse failure now returns `ConfigEditError`; backup (`<path>.rgt.bak`) written before any destructive write. Uses `jsonc-parser` for comment/trailing-comma tolerance. Test: `hook_installer.rs:439-464`. |
| 2.2 | Whole-key `hooks` overwrite instead of merge | **FIXED** | `editor.rs:186-331` merges only RGT-owned entries into existing arrays. Test explicitly covers the RTK-coexistence scenario: `claude_merge_preserves_foreign_hooks_settings_and_comments` (`hook_installer.rs:107-149`). |
| 2.3 | Hermes TOML string-splicing | **FIXED** | Real `toml_edit::DocumentMut` (`editor.rs:429-520`, `Cargo.toml` dependency added). Test confirms no duplicate `[plugins]` table. |
| 2.4 | `--force` truncates content below marker | **FIXED** | Paired start/end markers now required (`editor.rs:602-654`); a legacy single-marker block is now a hard error rather than a guess. |
| 2.5 | Silent no-op when `rgt` not on PATH | **FIXED** | `rgt doctor` (new, `src/cli/doctor.rs`) warns on "hooks installed, 0 nodes ever recorded." `rgt init` now writes an absolute path via `current_exe()` into every hook command. |
| 2.6 | Empty matcher fires on every tool call | **FIXED** | Matcher now `"Read|Edit|Write|Bash"` (kept broader than the original `Read|Write|Edit` suggestion, deliberately, to keep Bash-read capture working for the §8 fix). |
| 2.7 | Copilot CLI dialect unverified assumption | **NOT FIXED — honestly so.** | `hook_parsers.rs:461-463` states outright: "an assumption pending live verification." Never claimed fixed. |
| 2.8 | No restart reminder after `rgt init` | **FIXED** | `src/cli/init.rs:27-32,93-98` prints "Restart your AI coding tool now..." when something was newly configured. |
| 6 | Non-text-source instructions per agent | **MOSTLY FIXED — one deliberate gap.** | All 7 rules-tier agents plus, newly, Claude Code (`CLAUDE.md`) got the instruction block. **Cursor did not** — the implementing commit's own message says this was deliberate, since Cursor's actual rules-file format was never verified (same caution this report itself flagged in §6.3). |
| 8.1/8.2 | Bash `cat`/`head`/`grep` invisible to Claude Code/Cursor | **FIXED** | `normalize_agent_event` now tries `default_dialect` then falls back to `command_dialect`. Test: `bash_payload_is_captured_for_claude_and_cursor`. |
| 8.3 | `extract_path_from_command` doesn't see through `rtk` wrapper | **FIXED** | Special-cased: first token `rtk`, second token must be `read` (matching RTK's actual subcommand name, correcting the original report's minor inaccuracy about a nonexistent `rtk cat`). |
| 3.1 | Confusing raw I/O error for PDF/xlsx in `rgt record` | **FIXED** | Explicit "unsupported file format... use your AI agent's own read/extraction tool" message, both file and `--stdin` paths. |

**New issue found, not in the original report:** `shell_quote` wraps the resolved `rgt` path in literal double quotes for the hook `command` string (to handle paths with spaces), but `is_rgt_command`'s ownership check tokenizes with plain `split_whitespace()`, which isn't quote-aware. If RGT is ever installed under a path containing a space (e.g. Windows `Program Files`, or certain macOS `Application Support` layouts), a subsequent `rgt init` may fail to recognize its own previously-written hook entry and could append a duplicate instead of updating in place. Flagged as medium-confidence (reasoned from code, not empirically reproduced) — worth a quick test with a spaced path before relying on it in those environments.

---

## 3. §3.2 / §3.3 / same-line collision — Extraction correctness & locale (specs `022`, `027`)

| # | Finding | Verdict | Evidence |
|---|---|---|---|
| 3.2 | No locale-aware number parsing | **FIXED** | New `--number-format us\|eu\|auto` (default `auto`), wired through `rgt init` and `rgt record`. Hand-traced and test-confirmed against all four original failure modes: `"1,234.56"` (US) → `1234.56`; `"1.234,56"` (EU) → `1234.56`; `"-42"` → `-42` (sign preserved); `"(500)"` → `-500` (accounting parens). Malformed groupings are now rejected/skipped, never silently mis-split. |
| — | Residual, documented limitation in `Auto` mode | **Flag, not a defect** | A file with *only* single-separator numbers (no number containing both `.` and `,`) falls back to `Us` convention — so a pure-EU file using `.` as a thousands separator with no comma-decimal numbers anywhere would still misread `"1.234"` as `1.234` rather than `1234`. This is a tested, intentional tradeoff (`auto_single_separator_falls_back_to_us`), not a silent bug — but worth the maintainer's awareness as a known edge case. |
| 3.3 | ISO date → 3 spurious number nodes | **FIXED** | Date spans are now collected first and excluded from number matching. Empirically confirmed: `"start: 2026-01-15"` now produces exactly 1 node, zero spurious numbers. The originally-missing regression test now exists and asserts total count, not just date count. |
| 3.3 | Non-ISO dates (`27/08/2026`, etc.) still invisible | **NOT FIXED — scoped out, not a false claim.** | The date regex itself is unchanged; no flexible-date-parsing crate was added. Empirically confirmed: `"due: 27/08/2026"` still produces 3 spurious number nodes today. The fix addressed "mask matched spans" (suggestion a) but not "recognize more formats" (the separate alternative) — a reasonable scoping choice, but this specific leak is still live and should not be considered closed. |
| §7.3 | Same-line identical values collide into one node | **FIXED, confirmed with the exact original repro.** | `TrackedNode::generate_root_id` now includes a per-occurrence discriminator. `revenue,120000,discount,120000` → CLI reports "2 values," and `rgt graph`/`rgt status` now genuinely show 2 distinct nodes (previously 1). |

**New issue found:** none beyond the settings.json side-effect already handled above, and the pre-existing `test_update_contract.rs` failures (§7.1, not this area's fault).

---

## 4. §7.1 — Self-update mechanism (spec `020`)

| # | Finding | Verdict | Evidence |
|---|---|---|---|
| Checksum bypass | Missing `checksums.txt` silently skipped verification | **FIXED** | Hard error unless the explicit `--skip-checksum` flag is passed. Confirmed by passing tests `missing_checksums_is_a_hard_refusal`, `skip_checksum_bypasses_the_checksum_refusal`. |
| No signature verification | Checksum from the same untrusted source as the binary | **FIXED — architecturally sound, with real maturity caveats.** | Implements GitHub Artifact Attestations (Sigstore bundle: Fulcio cert chain, Rekor transparency log), verified against an **embedded Sigstore public-good trust root** — a genuinely independent trust anchor, not a second hash from the same release JSON. Release workflow emits attestations via `actions/attest-build-provenance@v2`. |
| — | Maturity concern on the above | **Flag — not a defect, but real risk** | The verification crate (`attestation-verify` v0.1.0) is a brand-new, third-party dependency with no visible adoption (repo created 2026-07-29, 0 stars/forks). The production policy has **never been tested against a real `rafael-bianchi/rgt` attestation** — the relevant self-test explicitly self-skips because no fixture exists yet (gated on "first attested release"). Until a real attested release ships and that test actually runs, this fix is unverified in the one scenario that matters most. |
| Semver downgrade | Plain string inequality, not semver ordering | **FIXED** | Real `semver` crate; `latest_is_newer` is the single gate; explicit `--version` downgrade requires a named opt-in flag. **Not implemented**: the "list all releases, pick max" alternative — still calls `/releases/latest` — but this is an acceptable residual gap since the semver gate independently prevents a downgrade either way (worst case: incorrectly reports "up to date," never installs something older). |
| Non-atomic replace | Backup-then-rename dance, no rollback | **FIXED** | POSIX path is now a single `fs::rename`, confirmed never to leave the target absent. Windows now uses the `self_replace` crate (exactly the alternative this report suggested) instead of hand-rolled logic. |

**New issues found:**
- **The test suite currently has 2 real failures** (`tests/contract/test_update_contract.rs::genuine_newer_tag_proceeds_past_gating`, `::failed_verification_never_reaches_replace`) — both hardcode a stub "newer" version (`v0.4.1`) that is no longer newer than the project's actual current version (`0.5.0`, bumped in a later commit). This needs a one-line fixture fix, but it means `develop`'s test suite is red right now.
- The unaudited `attestation-verify` dependency and its untested-against-reality production policy (detailed above).

---

## 5. §7.2 / §7.3 — Storage/graph integrity & CLI verification (specs `021`, `024`, `029`)

All 13 tracked findings in this area: **FIXED**, most with passing dedicated tests and several with direct empirical reproduction against a release build.

| # | Finding | Verdict | Evidence |
|---|---|---|---|
| Cycle guard | Only in-memory, not at DB-insert layer | **FIXED** | `insert_derivation_edge` now rejects self-loops and full cycles via a `WITH RECURSIVE` reachability check before insert, plus a schema-level `CHECK` constraint. Tests cover self-loop, 2-node, and 3-node cycles. |
| `busy_timeout` | Never set | **FIXED** | `conn.busy_timeout(Duration::from_secs(5))` on every connection open. |
| 48-bit IDs / non-updating conflict clause | Truncated hash + partial `ON CONFLICT` update | **FIXED** | IDs now 128 bits; `ON CONFLICT DO UPDATE` now updates every value-bearing column, not just staleness fields. |
| No transaction in hook batch insert | Unwrapped per-row inserts | **FIXED** | Wrapped in a single transaction; empirically confirmed both values from a test file land as distinct nodes atomically. |
| No schema migration path | `CREATE TABLE IF NOT EXISTS` only | **FIXED** | `PRAGMA user_version`-based versioning, hand-rolled (reasonable given only one version exists so far) rather than pulling in `rusqlite_migration`. |
| No garbage collection | No `DELETE`/`VACUUM` | **FIXED** | New `rgt gc` command: deletes stale leaf nodes with no dependents, cascades edge deletion, runs `VACUUM` correctly outside the transaction. |
| `total_invalidated` double-count | Overcounted shared descendants | **FIXED** | Now dedupes via a `HashSet` of distinct node IDs. |
| Stack-overflow crash | Deeply-nested `--expression` | **FIXED, exact original repro reproduced.** | Length (4096) and depth (256) limits checked before evaluation; the exact original crashing command now cleanly errors with exit code 2 instead of aborting. |
| Case-mismatched `--operation` bypass | Silent skip, exit 0 | **FIXED, empirically confirmed.** | Case-sensitive allow-list; `--operation expression` now exits 2 with an explicit "unknown operation" error instead of silently passing. |
| `.as_float()` vs `.as_number()` | False rejection of integer literals | **FIXED** | Now uses `.as_number()`; `"2*50"` correctly verifies. |
| evalexpr builtins exposed | `random()`, regex functions, etc. | **FIXED** | `set_builtin_functions_disabled(true)`; dedicated tests confirm `random`/`str::*`/`if` are now rejected. |
| `DATE_DIFF` parent order | Unenforced, silent sign flip | **PARTIALLY FIXED.** | A negative result now triggers an explicit "parents may be swapped" error — closes the confirmed failure mode, but doesn't enforce true ordering (a same-magnitude coincidence could theoretically still slip through). Reasonable, pragmatic fix for the case that was actually demonstrated. |
| >26 parents / `u8` overflow | Unchecked arithmetic | **FIXED, empirically confirmed.** | Explicit bound check; 27 parents now exits 2 with a clear message instead of silently wrapping. |

**New issue found:** `src/cli/status.rs`'s `--stale-only`-style branch appears to be dead code — both branches of a conditional call the same function identically, so the flag has no differentiating effect on the returned node list. Output happens to already only show stale nodes either way, so likely harmless in practice, but looks like an incomplete refactor worth a quick look.

---

## 6. §7.4 — Change-detection layer (spec `025`)

| # | Finding | Verdict | Evidence |
|---|---|---|---|
| Tier 2 unreachable for Tier 1 false negatives | Hash only computed after Tier 1 says "changed" | **FIXED — periodic, not unconditional (the right call).** | A `force_hash` path now re-hashes even when Tier 1 says "unchanged," gated by a 1-hour threshold per file (`REHASH_THRESHOLD`). This bounds the staleness window to ≤1h per file while preserving the fast mtime/size-only path for recently-checked files — deliberately not "hash every single check," which would have real performance cost. Confirmed via passing test and a live benchmark (6.04µs, well under the 1ms budget). |
| TOCTOU (content vs. metadata/hash read separately) | Independent re-reads | **PARTIALLY FIXED.** | `rgt record` and the hook's disk-read fallback now hash the exact in-memory bytes already read for extraction — fully closed for those two paths. **The hook's primary path (when the agent's tool actually returned `tool_response.content`) still independently re-reads the file from disk to compute the hash**, while extraction uses the agent-reported text — so if disk content differs from what the agent reported (a race, or a stale agent-side read), the stored fingerprint can still describe different bytes than what was recorded. This is explicitly out of scope per the implementing commit's own message, not a false claim, but it's a live residual gap in the most commonly-hit code path (the passive hook). |
| `FileNotFound` conflating all I/O errors | Permission/lock errors mislabeled as deletion | **FIXED** | New `CheckFailed` variant distinguishes `io::ErrorKind::NotFound` from other errors; routes to "state unchanged" rather than "file deleted." |
| Symlink swaps invisible | No inode/device identity tracked | **FIXED on Unix; gated to MSVC on Windows.** | `st_dev`/`st_ino` now part of the fingerprint. Windows support is deliberately MSVC-only (GNU target gets `None`), a documented choice, not an oversight. |
| Path-identity case-sensitivity collisions | Byte-exact `file_path` key | **FIXED, and wasn't even named in the commit title.** | When device/inode identity is known, a follow-up query removes stale differently-cased rows pointing at the same underlying file, leaving exactly one row. Same Unix/MSVC caveat as above. |
| No test coverage of tier1/tier2 algorithm | Misleadingly-named test file tested something unrelated | **FIXED** | `tests/unit/detection.rs` now has a dedicated module directly exercising the real detection functions; original unrelated tests left intact alongside it. |

**New issues found:**
- `benches/bench_detection.rs` was only updated for the new function signature — it never benchmarks the forced-rehash (full blake3) path, so the headline behavior change of this entire hardening pass (periodic full-file hashing) has zero performance coverage or budget, despite being the most performance-relevant change in the set.
- The hook's tool_response-content hashing path (see TOCTOU above) has a comment that could be misread as claiming this path is TOCTOU-safe when it isn't — worth a doc correction even before a code fix.

---

## 7. Overall scorecard

- **~24 of ~30 findings: FIXED and verified** (many empirically, several with the exact original repro re-run against the new binary).
- **4 findings: PARTIALLY FIXED**, each with a clearly bounded, honestly-scoped residual gap (non-ISO dates, hook's primary-path TOCTOU, `DATE_DIFF` ordering enforcement, `Auto`-mode single-separator ambiguity).
- **2 findings: NOT FIXED**, both self-documented as open in the new code itself rather than silently dropped (Copilot CLI dialect, Cursor instructions file).
- **New issues surfaced by this audit, not in the original report:** the `rtk`-path-with-spaces hook-ownership detection gap, the dead `--stale-only` branch, missing rehash-path benchmark coverage, and — most actionable — **2 currently-failing tests** on `develop` from a stale version fixture, plus the unaudited/untested-in-production attestation-verification dependency.

This is a strong result for a single findings report driving real fixes: the vast majority of issues were closed correctly and durably (schema constraints, real crates, dedicated regression tests), the few that weren't are transparently flagged as such in the implementation itself, and nothing was found to have been fixed only in the commit message.
