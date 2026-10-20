# Attestation Upgrade Repair Implementation Plan

**Goal:** Repair RGT's verification of real GitHub attestations without disabling cryptographic checks, and prevent stable releases from publishing unverified assets.

**Architecture:** Retain the Rust verifier and pin a locally documented compatibility patch to attestation-verify 0.1.0 if no upstream fix is available. Accept only the two observed equivalent empty-keyid envelope serializations; never modify the logged body or bypass signature, certificate, payload, inclusion, checkpoint, or repository-policy verification.

**Constraints:** Preserve published v0.8.0 source identity. Never replace CI archives with locally built binaries. An installed 0.7.0 verifier cannot be changed by downloading a newer binary; document an independently verified one-time migration if no compatible signed attestation is available.

## Tasks

- [x] Reproduce the failure with a disposable copy of the published 0.7.0 binary and the restored v0.8.0 archive. Observed checksum success followed by envelopeHash mismatch.
- [x] Add the real RGT bundle as a deterministic fixture; demonstrate the existing verifier fails it.
- [x] Patch envelope reconstruction and test absent/empty/non-empty key IDs and tampered payload, signature, log body, certificate, and artifact digest. Retain all other checks; investigate any subsequent failure rather than suppressing it.
- [x] Add a release verification command and workflow gates for archive completeness, checksums, attestations, and the packaged binary's own verifier before stable publication. Check out the release tag for verification code.
- [ ] Document the one-time verified migration, run formatting/lint/full tests and real-asset verification, obtain review, and publish only after the release gates pass.

## Progress

Release recovery: the original Mac CI archive and all six checksum entries have been restored and verified through the GitHub API. GitHub CLI verifies that archive independently. No further published assets will be modified during implementation.

Verifier proof: the original 0.7.0 binary failed on the restored asset. The
patched verifier passed the full real bundle and rejected tampering. A copied
patched executable completed a real `update --yes --version v0.8.0` including
checksum, attestation, extraction, and replacement in a disposable directory.
This does not change the verifier in existing installed 0.7.0 binaries.

Review: no Critical issues; addressed Important findings for surplus unsigned
archives, RC source/tag version mismatch, and publication before uploads. Added
seven deterministic release-gate tests. Full Rust suite and the upstream vendor
suite pass locally. Hosted CI and the patch publication are still pending.
