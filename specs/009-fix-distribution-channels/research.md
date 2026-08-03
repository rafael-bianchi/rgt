# Research: Fix Distribution Channels

**Feature**: `009-fix-distribution-channels` | **Date**: 2026-08-01

## Decision 1: Homebrew Formula — In-Repo vs. Separate Tap

**Decision**: Keep `Formula/rgt.rb` in the main repo. Serve it via `brew install rafael-bianchi/rgt/rgt`. Replace the `repository-dispatch` to `rafael-bianchi/homebrew-tap` with an in-repo SHA256 update step.

**Rationale**:
- The formula already uses main repo URLs (`#{homepage}` → `rafael-bianchi/rgt`). Only the update mechanism needed changing.
- No separate tap repo means no `HOMEBREW_TAP_TOKEN` secret, no dispatch workflow, no second repo to maintain.
- RTK (rtk-ai/rtk) uses the official homebrew-core, which is the ideal end state. In-repo formula is the simplest stepping stone.
- The CD `homebrew` job can compute SHA256s from the just-built release artifacts and commit the updated formula back to the repo using the workflow's built-in `GITHUB_TOKEN`.

**Alternatives considered**:
1. **Separate tap repo (`rafael-bianchi/homebrew-tap`)**: Adds complexity (second repo, dispatch token, event listener workflow). Rejected — unnecessary for a project this size.
2. **Submit to homebrew-core immediately**: Requires meeting their popularity/stability thresholds. Rejected for now — can be done later when the project is more established.

## Decision 2: Cargo Install — Git vs. crates.io

**Decision**: Document `cargo install --git https://github.com/rafael-bianchi/rgt` as the Cargo install method. Remove all references to bare `cargo install rgt` or crates.io publishing from active documentation.

**Rationale**:
- The `rgt` crate name on crates.io is owned by an unrelated project (red/green syntax tree library). Contacting the owner is unlikely to succeed.
- RTK (rtk-ai/rtk) uses the same approach (`cargo install --git`) due to the same name collision problem.
- No crates.io account, API token, or `CARGO_REGISTRY_TOKEN` secret needed.
- `cargo install --git` works identically to `cargo install <crate>` for the end user — same CLI, same result.

**Alternatives considered**:
1. **Publish under a different name (e.g., `rgt-tracker`, `rust-graph-tracker`)**: Still requires crates.io setup and a non-ideal crate name. Users would need to know the alternate name. Rejected — git install is simpler and the repo name is the canonical identifier.
2. **Contact crates.io owner to transfer name**: Unlikely to succeed; the existing crate has active versions. Rejected as unreliable.

## Decision 3: Homebrew Job Update Mechanism

**Decision**: In the `homebrew` job of `release.yml`, replace the `peter-evans/repository-dispatch` step with:
1. Checkout the main repo
2. Download the just-released artifacts
3. Compute SHA256 digests with `sha256sum`
4. Use `sed` to replace the `REPLACE_WITH_SHA256_*` placeholders in `Formula/rgt.rb`
5. Commit and push the updated formula

**Rationale**:
- No external dependency (no token, no second repo).
- Self-contained: the CD workflow already has access to the artifacts and the repo.
- The formula has predictable placeholder patterns that `sed` can safely target.

**Alternatives considered**:
1. **Custom script to generate the formula from a template**: More robust but over-engineered for 4 SHA256 values. Rejected — `sed` replacement is sufficient.
2. **Manual SHA256 updates after each release**: Error-prone and defeats CD automation. Rejected.

## Decision 4: Dead Code in `src/updater/github.rs`

**Decision**: Remove unused fields/structs/functions rather than wiring them into the CLI `update` subcommand. The updater module needs a separate feature spec if a self-update CLI is desired.

**Rationale**:
- The `rgt update` CLI already exists as a subcommand. The updater module has dead code (`check_update`, `UpdateCheckResult`, unused fields in `GitHubRelease` and `GitHubReleaseAsset`).
- Wiring the updater into the CLI is out of scope for this distribution fix — it would require its own spec, testing, and integration work.
- Removing dead code is the minimal change that satisfies FR-008 and eliminates clippy warnings.

**Alternatives considered**:
1. **Wire the updater into `rgt update`**: Requires significant new code (HTTP client integration, update flow, error handling). Rejected — out of scope.
2. **`#[allow(dead_code)]` annotations**: Suppresses warnings but doesn't address the underlying maintenance debt. Rejected.

## Decision 5: CD Concurrency on `main`

**Decision**: Keep `cancel-in-progress: false` on `main` (prevents partial releases). Cancel the currently stuck runs manually and re-trigger CD by pushing to `main` (or via `workflow_dispatch`).

**Rationale**:
- Cancelling an in-progress main release could leave a half-uploaded release. The current behavior is correct.
- The stuck runs (30719536844 pending, 30717436776 queued) need manual intervention via the GitHub Actions UI.
- After clearing the queue, the next push to `main` (e.g., release-please PR merge) will trigger a clean CD run.

**Alternatives considered**:
1. **Change `cancel-in-progress` to `true` on main**: Would cancel partial releases, leaving broken releases. Rejected.
2. **Separate `main` CD into its own workflow without concurrency**: Adds complexity. Rejected — the current design is correct; the issue is a one-time queue blockage.
