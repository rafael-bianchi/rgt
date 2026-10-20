# RGT Attestation Fixtures

This directory holds a **real** `rafael-bianchi/rgt` GitHub Artifact
Attestation bundle, captured from the first release built after the
`attest-build-provenance` step landed in `.github/workflows/release.yml`.

The committed fixture covers the published v0.8.0 Apple Silicon archive from
Actions run `39192508416`, captured on 2026-10-20. Its digest is recorded in
`artifact.sha256`. It reproduces the empty-`keyid` envelope serialization bug
in `attestation-verify 0.1.0`. The test is mandatory and never silently skips.
`gh attestation verify --repo rafael-bianchi/rgt` independently accepted the
original archive before this fixture was committed.

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

The test verifies the fixture against the production `rgt_policy()`. A failure
does not authorize weakening that policy: investigate the signed identity and
cryptographic error before making changes.

This is convergence task T024 (FR-003 / T018).
