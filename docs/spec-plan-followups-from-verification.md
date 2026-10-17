# RGT Spec Plan: Follow-ups from `verification-of-fixes-develop.md`

**Status:** Proposed — not yet executed
**Source:** `docs/verification-of-fixes-develop.md` §7 "Overall scorecard" and the "New issues found" callouts in §4 (self-update) and §2 (hooks). These are net-new issues surfaced while verifying specs `019`–`029`, not present in the original `findings-hooks-and-value-extraction.md`.
**Method:** Same as `docs/spec-plan-from-findings.md` — each spec is an independently developable/testable/deliverable unit. Priority reflects actual current impact: a red test suite is P0, a latent-but-real bug with no known trigger yet is P1, and a security-relevant dependency-maturity concern with no fix action beyond monitoring is P2.

## Spec list (feature directories continue from `029`, so start at `030`)

| # | Feature directory | Findings covered | Priority | Rationale |
|---|-------------------|------------------|----------|-----------|
| 1 | `030-update-contract-test-fixture-fix` | Two currently-failing tests in `tests/contract/test_update_contract.rs` (`genuine_newer_tag_proceeds_past_gating`, `failed_verification_never_reaches_replace`) — stale fixtures hardcode stub version `v0.4.1` as "newer," which stopped being true once `Cargo.toml` was bumped to `0.5.0` | P0 | `develop`'s test suite is red right now. Blocks trusting CI on this branch for anything else. Trivial fix (update the fixture's stub version), but must land before or alongside any other change on this branch. |
| 2 | `031-hook-editor-path-quoting-fix` | `src/hooks/installer.rs` `shell_quote` (quote-wraps the resolved `rgt` path for hook `command` strings) vs. `src/hooks/editor.rs` `is_rgt_command` (tokenizes hook `command` strings via plain `split_whitespace()`, not quote-aware) disagree when the resolved `rgt` binary path contains a space | P1 | Real latent bug in the just-shipped absolute-path hook-install fix (spec `028`). No known trigger has hit it yet (this machine's install path has no spaces), but it's a plausible real-world case (Windows `Program Files`, some macOS `Application Support` layouts) and the failure mode is silent duplicate hook entries on re-init, which is exactly the class of bug specs `019`/`028` were built to eliminate. |
| 3 | `032-attestation-dependency-hardening` | `attestation-verify` v0.1.0 (spec `020`'s new signature-verification dependency) is a brand-new, seemingly zero-adoption third-party crate carrying the entire trust guarantee for `rgt update`'s signature check; its production verification policy (`rgt_policy()`) has never been exercised against a real signed `rafael-bianchi/rgt` release — the relevant self-test explicitly self-skips pending a fixture that doesn't exist yet | P2 | Not a known bug — the implementation is architecturally sound (verified against the actual code in `docs/verification-of-fixes-develop.md` §4) — but it's an unaudited dependency carrying a security-critical function, untested in the one scenario (a real attested release) that matters most. This is a hardening/monitoring task, not a code defect with a clear one-line fix, so it's scoped and prioritized differently from the other two. |
| 4 | `033-pdf-capture-via-additionalcontext` | Full spec: `docs/spec-033-pdf-capture-via-additionalcontext.md`. Closes the §4 native-PDF gap fully automatically — `rgt hook post` decodes the PDF's base64 envelope, runs local `pdf-extract` text extraction, records provenance nodes with zero agent action required, and (for Claude Code) injects the real extracted text back via `PostToolUse`'s `additionalContext` field. Mechanism empirically verified live, twice, in-session (not inferred from docs) — including proving `additionalContext` injection actually reaches the model. | P1 | Supersedes spec `023`'s Claude-Code behavior with something strictly better (no agent cooperation required, vs. relying on the agent remembering to call `rgt record --stdin`); doesn't remove `023`'s fallback, which stays as the safety net for agents/scenarios this hasn't been verified for yet (see the spec's own Open Questions: Cursor support, `.xlsx` envelope shape, `pdf-extract` malformed-input safety). |

## Execution order

**P0 — unblock CI first**
1. `030-update-contract-test-fixture-fix` — do this before anything else touches this branch; a red test suite makes every other change harder to trust.

**P1 — real latent bug, no urgency-forcing trigger yet**
2. `031-hook-editor-path-quoting-fix` — independent of everything else; touches `src/hooks/editor.rs`/`installer.rs` only.

**P1 — highest-leverage capability gap, independent of the P0/other P1 items above**
3. `033-pdf-capture-via-additionalcontext` — see the dedicated spec doc for full detail; independent of `030`/`031`, can run in parallel with either.

**P2 — hardening/monitoring, not a code fix**
4. `032-attestation-dependency-hardening` — see scope note below; the actionable part of this spec may be "add the real-release fixture and un-skip the self-test" plus "document the dependency risk," not a behavior change. Revisit once RGT has shipped its first attested release (needed for the fixture regardless).

## Scope notes

- **`030`** is a pure test-fixture fix: update the hardcoded stub version in the two failing tests to something genuinely older than the current `Cargo.toml` version (or derive it from `Cargo.toml` at test time instead of hardcoding, to prevent this from recurring on the next version bump — worth deciding which during `/speckit-specify`).
- **`031`**: the fix is either (a) make `is_rgt_command`'s tokenizer quote-aware (e.g. use the same quoting/escaping logic `shell_quote` applies, in reverse), or (b) avoid the ambiguity entirely by storing/matching on the unquoted resolved path directly rather than re-parsing the shell-quoted command string. Recommend adding a test with a deliberately spaced install path (e.g. simulate a `/Applications/My App/rgt` style path) before considering this closed, since neither of the two prior verification passes (spec `028`'s own tests, nor the verification audit) exercised this case empirically — it was reasoned from code, not reproduced.
- **`032`** has an unusual shape: there may be no code change available yet (the missing fixture is gated on a real attested release existing, which is a release-process dependency, not something fixable in this repo alone). The spec should capture: (a) pin/monitor the `attestation-verify` crate version and watch for its own security advisories given its immaturity; (b) as soon as a real attested `rafael-bianchi/rgt` release exists, capture its bundle as the missing test fixture and un-skip `verifies_real_rgt_bundle_when_fixture_present`; (c) consider whether a documented interim risk acceptance (e.g. in `CONTRIBUTING.md` or a `SECURITY.md`) is warranted until that fixture lands, given `rgt update`'s signature check currently rests entirely on an unreviewed 0.1.0 dependency.

## What "specify next" means

Same as `docs/spec-plan-from-findings.md`: after completing each feature's `/speckit-specify` + plan/tasks cycle and merging the implementation, look up the next unstarted entry in the table above. The `#` column is the stable identity; feature directory numbers (`030`…) are assigned at spec-creation time from `.specify/init-options.json` (`feature_numbering: sequential`).
