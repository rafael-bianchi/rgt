# Quickstart & Validation Guide: Fix CI/CD Pipeline Post-Deployment Issues

**Feature Branch**: `004-fix-ci-cd-pipeline` | **Date**: 2026-07-31

## Overview

This guide provides runnable validation scenarios to verify the three CI/CD fixes.

---

## Scenario 1: Verify CI Triggers on Push

### Objective
Confirm CI runs on direct pushes to develop and main, not just pull requests.

### Execution
1. Make a trivial change (e.g., add a comment in README.md)
2. Commit and push directly to the develop branch
3. Check GitHub Actions tab

### Expected Outcome
- A CI workflow run appears within 30 seconds of the push
- All jobs (fmt, clippy, test, security) execute, regardless of PR presence

---

## Scenario 2: Verify Security Scan Uses Correct Base Branch

### Objective
Confirm the critical files check diffs against the PR's target branch, not hardcoded main.

### Execution
1. Create a branch from develop
2. Make a change only to README.md
3. Open a PR targeting develop
4. Observe the security job output

### Expected Outcome
- Critical files check reports "No critical files modified" (comparing against develop, not main)
- If the same PR were targeting main, files already changed on develop would show in the diff

---

## Scenario 3: Verify Release Automation Creates PR

### Objective
Confirm the CD pipeline successfully creates a release PR after the repository permission fix.

### Execution
1. Ensure "Allow GitHub Actions to create and approve pull requests" is checked in repository settings
2. Merge a `feat:` or `fix:` conventional commit to main
3. Check the CD workflow run

### Expected Outcome
- CD workflow completes without the "not permitted to create or approve pull requests" error
- A release PR is created on the main branch with version bump and changelog entries
