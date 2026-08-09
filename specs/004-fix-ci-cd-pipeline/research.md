# Technical Research: Fix CI/CD Pipeline Post-Deployment Issues

**Feature Branch**: `004-fix-ci-cd-pipeline` | **Date**: 2026-07-31

## 1. Push Trigger for CI Workflow

### Decision
Add `push: branches: [develop, main]` alongside the existing `pull_request` trigger in `ci.yml`.

### Rationale
- Maintainers push directly to develop/main during release merges and hotfixes. Without push-triggered CI, these commits bypass all validation gates.
- Adding the push trigger does not interfere with PR behavior — both triggers coexist and produce separate workflow runs.
- GitHub Actions supports multiple `on` event types in the same workflow file natively.

### Alternatives Considered
- *Keep PR-only and rely on branch protection rules to require PRs*: Not practical for a small team; maintainers need direct push capability for release-please PRs and hotfixes.
- *Separate push-ci.yml file*: Unnecessary duplication of the same workflow logic.

## 2. Dynamic Base Branch in Security Scans

### Decision
Replace hardcoded `origin/main` with `origin/${{ github.base_ref || 'main' }}` in the three git diff commands in `ci.yml`.

### Rationale
- `github.base_ref` provides the PR's target branch at runtime (e.g., `develop` or `main`). When running on a pull_request event, this is the correct base for diffs.
- The `|| 'main'` fallback handles push events where `github.base_ref` is empty. Comparing against main on direct pushes is a reasonable default since main is the canonical source of truth.
- GitHub Actions evaluates `${{ }}` expressions before the shell script runs, so the base ref is injected as a literal string with no runtime overhead.

### Alternatives Considered
- *Always use origin/develop*: Breaks PRs targeting main.
- *Use `github.event.pull_request.base.sha`*: Provides an exact commit SHA which is more precise but requires the PR event context to exist; the fallback logic is messier for push events.
- *Remove the hardcoded diff and rely on clippy only*: Loses the critical files check, pattern scan, and new dependency detection — unacceptable security regression.

## 3. Release-Please Permission Fix

### Decision
Enable "Allow GitHub Actions to create and approve pull requests" in the repository's Actions settings (Settings → Actions → General → Workflow permissions).

### Rationale
- This is a one-time repository configuration change with no workflow code modifications needed.
- The `cd.yml` already declares the correct permissions (`contents: write, pull-requests: write`). The repository-level setting gates whether the GITHUB_TOKEN can actually exercise the `pull-requests: write` permission.
- RTK uses a GitHub App token (`actions/create-github-app-token@v3`) for this purpose, which is the recommended approach for larger projects. For a project of RGT's scale, enabling the repository setting is simpler and avoids the overhead of creating and maintaining a GitHub App.

### Alternatives Considered
- *GitHub App token*: Requires creating a GitHub App, generating a private key, and storing it as a repository secret. Overkill for current project scale.
- *Personal Access Token (PAT)*: Works but ties the automation to an individual account; breaks when the token owner leaves. Not recommended for team projects.
- *Manual release tagging*: Reverts the CD automation built in feature 003. Undesirable regression.
