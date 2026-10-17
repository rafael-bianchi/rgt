# Research: CI Pipeline Hygiene

**Feature**: [spec.md](./spec.md) | **Date**: 2026-08-31

Decisions grounded in the actual `.github/workflows/*.yml` files (read in-session) and the growth report's §2 items 4-6.

## 1. Test-presence check (FR-001, SC-001)

**Finding**: `ci.yml`'s `check-test-presence` job (lines 150-165) ends with `echo "Skipped - add custom check logic as needed."` and always passes. It already has the `github.base_ref == 'develop'` gate and a `fetch-depth: 50` checkout.

**Decision**: Implement the real logic in `scripts/check-test-presence.sh`:
1. Compute changed files vs the PR base (`git diff --name-only origin/main...HEAD`, matching the existing security job's diff pattern; fall back to `HEAD~1` when origin/main is unavailable so the script works locally).
2. For each changed `src/*.rs` excluding `main.rs`/`lib.rs`: derive a "test stem" (e.g. `src/cli/derive.rs` → look for `tests/` files whose content references the module, OR a path-derived test file). The check is: does any file under `tests/` reference this source file? Use a pragmatic heuristic — grep the `tests/` tree for the source file's basename and its module path.
3. A changed source file with no test reference → print an actionable failure naming the file and a suggested test path; exit non-zero. Files that are re-exported via `lib.rs` and tested through other modules are the known-false-positive risk — the check only fails on a hard miss (no reference anywhere), keeping false positives minimal.
4. Docs/workflow-only diffs → no changed `src/*.rs`, pass.

**Rationale**: The job's name and summary promise exactly this; a real check surfaces the "missing regression test" class this repo's verification passes repeatedly flagged. Local testability (a script, not buried YAML) is required by Constitution IX.

## 2. Code coverage (FR-002, FR-003, SC-003)

**Finding**: No coverage tooling or badge anywhere (growth report §2.5).

**Decision**: Add a `coverage` job to `ci.yml` (ubuntu only, after `test`):
- `cargo install cargo-llvm-cov` (or use `dtolnay/rust-toolchain`'s llvm-tools companion; install `llvm-tools-preview` component).
- `cargo llvm-cov --all` → writes summary; extract the line-coverage percentage.
- Write the percentage into `$GITHUB_STEP_SUMMARY`.
- Upload the `lcov`/HTML output as an artifact (no external upload service, no secret).

README: add a badge line. Without an external service, a badge needs a hosted URL. **Decision**: add a text note + link to the coverage job/artifact in the README's CI badge area ("Coverage: measured per-PR in CI"), avoiding a broken badge to a non-existent upload; a `codecov` badge is explicitly a maintainer follow-up (needs a token). This satisfies FR-003's "badge OR explicit note" and keeps the default path secret-free.

**Alternatives considered**: Codecov upload — rejected (needs a token/secret not present). `tarpaulin` — rejected: llvm-cov is more accurate and installable; keep as fallback note.

## 3. Action pinning (FR-004, FR-005, SC-002)

**Finding**: `release.yml` holds `contents: write`, `id-token: write`, `attestations: write` and uses tag-pinned third-party actions: `actions/checkout@v4`, `dtolnay/rust-toolchain@stable`, `Swatinem/rust-cache@v2`, `actions/attest-build-provenance@v2`, `actions/upload-artifact@v4`, `actions/download-artifact@v4`, `softprops/action-gh-release@v2`. `cd.yml` holds `contents: write`/`pull-requests: write` and uses `actions/checkout@v4`, `googleapis/release-please-action@v4`.

**Decision**: Resolve each tag → its exact commit SHA via the GitHub API (public repos need no token), then rewrite each `uses:` line as `uses: owner/repo@<40-hex-sha> # v4` (tag kept as a comment). Apply to `release.yml` and `cd.yml` mandatorily; `ci.yml` and `pr-target-check.yml` for consistency.

**Rationale**: Mutable tags can be re-pointed by a compromised owner; SHA pins are immutable. The trailing-comment convention preserves readability and auditability. The growth report's own note says `pr-target-check`'s `pull_request_target` is safe (no PR-head checkout), so no design change there — only pinning.

## 4. Verification (SC-001..SC-004)

- Run `scripts/check-test-presence.sh` against a synthetic diff (new `src/foo.rs`, no tests) → non-zero; against a docs-only diff → zero; against a real prior feature diff (e.g. a `src/*.rs` + `tests/` pair from the recent merges) → zero.
- `grep -E "uses: [a-zA-Z0-9_.-]+/[a-zA-Z0-9_.-]+@[0-9a-f]{40}"` matches every third-party action in `release.yml`/`cd.yml`.
- `cargo llvm-cov --all` runs and produces a percentage locally.
- Full `cargo test` green (no source changes).
