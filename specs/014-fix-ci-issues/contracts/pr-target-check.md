# Contract: PR Target Check Workflow Permissions

**Feature**: 014-fix-ci-issues
**Contract type**: CI workflow contract

## Synopsis

`.github/workflows/pr-target-check.yml` enforces the contribution convention (target `develop`, not `main`) by labeling and commenting on mis-targeted PRs. The workflow must have the permissions required to do so, scoped to least privilege.

## Permissions Contract

The workflow MUST declare exactly these scopes:

```yaml
permissions:
  contents: read
  issues: write
  pull-requests: write
```

**Rationale**:
- `issues: write` — required by `github.rest.issues.addLabels` (labels are issue-scoped API)
- `pull-requests: write` — required by `github.rest.issues.createComment` on PR threads
- `contents: read` — minimal base scope

**Prohibited**: Broad scopes (`contents: write`, `actions: write`, `*`).

## Behavior Contract

| Condition (PR) | Expected Behavior |
|---|---|
| `base == main` AND `head != develop` | Job runs; label `wrong-base` added; guidance comment posted; job succeeds |
| `base == develop` (any head) | Job skipped by `if` guard; no label, no comment, success status |
| other base | Job skipped; success status |

## Idempotency Contract

- `pull_request_target` fires on `opened` and `edited`.
- Re-adding the `wrong-base` label when already present MUST NOT error (REST API no-op; optionally guarded by an existence check).
- Duplicate comments are acceptable (no dedup required).

## Output Contract

**Comment body** MUST state that contributions should target `develop` and include a link to `CONTRIBUTING.md`.

## Acceptance Mapping

| Acceptance Scenario | Contract Clause |
|---|---|
| US2/AC1 (label added, no error) | Permissions + behavior |
| US2/AC2 (guidance comment with CONTRIBUTING.md link) | Output contract |
| US2/AC3 (base=develop skips cleanly) | Behavior contract |
| US2/AC4 (least privilege) | Permissions contract |
| Edge: fork PR | Permissions contract (elevated token works) |
| Edge: repeated edits | Idempotency contract |
