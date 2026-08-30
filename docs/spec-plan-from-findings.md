# RGT Spec Plan: Breaking `findings-hooks-and-value-extraction.md` into Feature Specs

**Status:** Proposed — not yet executed
**Source:** `docs/findings-hooks-and-value-extraction.md` (20+ findings across 8 subsystems)
**Method:** Each spec is an independently developable/testable/deliverable feature. Order follows the doc's own priority guidance: §5 (original ranking) + §7.5 (supersedes §5 for §7 areas). Both empirically-confirmed CRITICALs and the highest-blast-radius installer findings are P0.

## Spec list (feature directories continue from `018`, so start at `019`)

| # | Feature directory | Findings covered | Priority | Rationale |
|---|-------------------|------------------|----------|-----------|
| 1 | `019-installer-config-preservation` | §2.1 silent settings destruction, §2.2 whole-key overwrite, §2.3 Hermes TOML splice, §2.4 `--force` truncation | P0 | Doc §5 #1 — highest blast radius: corrupts user config for every agent. Also the safe foundation for every later spec that writes agent config (023, 026, 028). |
| 2 | `020-update-integrity` | §7.1 checksum bypass (CRITICAL), unsigned checksum source, string version compare, non-atomic replace, TOCTOU | P0 | CRITICAL — silently installs an unverified binary when a release lacks `checksums.txt`. §7.5 #1. |
| 3 | `021-derive-verify-hardening` | §7.3 stack-overflow (CRITICAL), case-mismatched `--operation` bypass, `as_float` vs `as_number`, evalexpr builtins/`random()`, `DATE_DIFF` parent order, u8 parent overflow | P0 | CRITICAL — one `--expression` arg crashes the CLI; silent verify bypass defeats the command's purpose. §7.5 #1/#3. |
| 4 | `022-value-extraction-correctness` | §3.3 date→number cross-contamination + broken test assertion, §7.3 same-line identical-value collision | P1 | Doc §5 #3 — cheap correctness bug masked by an incomplete test assertion; both confirmed empirically. |
| 5 | `023-non-text-source-capture` | §4 native PDF gap (confirmed), §6 per-agent explicit-report instructions, §3.1 `rgt record` non-UTF-8 error | P1 | Doc §5 #2 — primary fix is agent instructions only, zero code risk; ships right after the P0 batch. |
| 6 | `024-store-graph-integrity` | §7.2 cycle-guard not on write path, missing `busy_timeout`, 48-bit IDs + non-updating `ON CONFLICT`, no transaction in hook inserts | P1 | §7.5 #2 — undermines the "trustworthy provenance graph" promise under concurrency/collision. |
| 7 | `025-change-detection-hardening` | §7.4 Tier1/Tier2 architecture gap, TOCTOU, `FileNotFound` conflation, symlink swaps, path-identity case | P1 | §7.5 #4 — silent "unchanged" can misreport staleness; needs specific FS/timing conditions, lower urgency than 024. |
| 8 | `026-capture-bash-tool-reads` | §8 Claude/Cursor Bash `cat`/`head`/`grep` invisible, `rtk` wrapper recognition, stdout capture | P1 | Both flagship integrations silently blind — confirmed with live payload. Moderate code, high value. |
| 9 | `027-locale-aware-extraction` | §3.2 locale/separator/sign handling, `--number-format` flag | P2 | Doc §5 #4 — highest real-world value for non-US documents; design-heavy, decoupled from 022. |
| 10 | `028-init-observability` | §2.5 PATH no-op + `rgt doctor`, §2.6 empty matcher, §2.7 Copilot dialect live-verification, §2.8 restart reminder | P2 | Doc §5 #6 — trust/observability; no correctness bugs. |
| 11 | `029-store-maintenance` | §7.2 LOWs: no schema migration, no GC, `total_invalidated` double-count | P3 | §7.5 #5 — hygiene; can slip a release. |

## Execution order (one spec at a time)

**P0 — safety & security first**
1. `019-installer-config-preservation` — do before any other spec that writes agent config.
2. `020-update-integrity` — CRITICAL supply-chain fix, fully independent.
3. `021-derive-verify-hardening` — CRITICAL crash + silent bypass, fully independent.

> Order within the P0 trio is flexible (019/020/021 are mutually independent). 019 lands before 023/026/028 so new config-writing work builds on safe merge/append semantics.

**P1 — provenance correctness before coverage**
4. `022-value-extraction-correctness`
5. `023-non-text-source-capture`
6. `024-store-graph-integrity`
7. `025-change-detection-hardening`
8. `026-capture-bash-tool-reads`

**P2/P3 — value and hygiene**
9. `027-locale-aware-extraction`
10. `028-init-observability`
11. `029-store-maintenance`

## Deliberate grouping decisions (flagged for review)

- **§7.3 same-line collision** is in `022-value-extraction-correctness`, not `021-derive-verify-hardening`, because its root cause is in `extract_values_from_content`/node-ID hashing, not the verify command.
- **§8 (Bash reads)** and **§2.7 (Copilot dialect)** are kept separate from §6's "needs verification" work:
  - `023` owns verification of Cursor/Copilot Chat instructions-file formats (§6.3).
  - `026` owns Claude/Cursor Bash payloads and the `rtk` wrapper (§8).
  - `028` owns the Copilot CLI dialect live-capture check (§2.7).
- **§7.1 LOWs** (backup-cleanup swallowed errors, checksum TOCTOU) ride along in `020-update-integrity`; **§7.2 LOWs** are collected into `029-store-maintenance`.

## What "specify next" means

After completing each feature's `/speckit-specify` + plan/tasks cycle and merging the implementation, look up the next unstarted entry in the table above. The `#` column is the stable identity; feature directory numbers (`019`…) are assigned at spec-creation time from `.specify/init-options.json` (`feature_numbering: sequential`).
