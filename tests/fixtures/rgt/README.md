# RGT Attestation Fixtures

This directory holds a **real** `rafael-bianchi/rgt` GitHub Artifact
Attestation bundle, captured from the first release built after the
`attest-build-provenance` step landed in `.github/workflows/release.yml`.

Until such a release exists, the directory intentionally has **no fixture** and
`verifies_real_rgt_bundle_when_fixture_present` in
`tests/unit/test_update_attestation.rs` skips itself.

## Capture procedure (run once after the first attested release)

For a release tag `vX.Y.Z`, pick any released archive asset (e.g.
`rgt-vX.Y.Z-aarch64-apple-darwin.tar.gz`):

```sh
# 1. Digest of the released artifact
curl -fsSL -o /tmp/rgt-artifact \
  "https://github.com/rafael-bianchi/rgt/releases/download/vX.Y.Z/rgt-vX.Y.Z-aarch64-apple-darwin.tar.gz"
shasum -a 256 /tmp/rgt-artifact          # -> <hex>

# 2. Fetch the attestation bundle for that digest
gh api "/repos/rafael-bianchi/rgt/attestations/sha256:<hex>" \
  --jq '.attestations[0].bundle_url'     # -> <bundle_url>

# 3. Download and snappy-decompress it
curl -fsSL -o /tmp/rgt-bundle.snappy "<bundle_url>"
# decompress with `snap` (or `gh attestation download`): the body is
# raw-snappy-compressed Sigstore v0.3 bundle JSON

# 4. Commit the decompressed bundle as rgt-bundle.json and record the digest
cp /tmp/rgt-bundle.json tests/fixtures/rgt/rgt-bundle.json
echo "<hex>" > tests/fixtures/rgt/artifact.sha256
```

Then the gated test in `tests/unit/test_update_attestation.rs` verifies it
against the production `rgt_policy()`. If verification fails, the recorded
identity differs from the policy constants — update `RGT_SIGNER_WORKFLOW`
and/or the `RefPolicy` in `src/updater/attestation.rs` to match the real
bundle's `source_ref`/`signer_workflow_path` (inspect the failed
`VerificationReport`/error), then re-run the test.

This is convergence task T024 (FR-003 / T018).
