# Verified migration from RGT 0.7.0 / 0.8.0

These builds use `attestation-verify 0.1.0`, which reconstructs DSSE envelopes
with empty `keyid` omitted. GitHub's JS submitter hashes the envelope with
`keyid: ""` present. The installed updater therefore rejects legitimate
attestations with an `envelopeHash` mismatch before it can install a fix.

This is a verifier compatibility bug, not evidence that disabling verification
is safe. `--skip-attestation` is not the recommended migration. The fix in a
new archive cannot repair the verifier already running on your machine.

## One-time migration using independent verification

First wait for a patch release containing the compatibility fix and all
verification gates to finish. Substitute its actual stable tag below; do not
use an unpublished tag. Install GitHub CLI (`gh`) from its trusted distribution
if it is not already available.

For an Apple Silicon Mac, in a new temporary directory:

```sh
TAG=vX.Y.Z  # replace with the published fixed version
ARCHIVE="rgt-${TAG}-aarch64-apple-darwin.tar.gz"
gh release download "$TAG" --repo rafael-bianchi/rgt \
  --pattern "$ARCHIVE" --pattern checksums.txt --pattern "${ARCHIVE}.sigstore.json"
grep -F "  $ARCHIVE" checksums.txt | shasum -a 256 -c -
gh attestation verify "$ARCHIVE" --bundle "${ARCHIVE}.sigstore.json" \
  --repo rafael-bianchi/rgt \
  --signer-workflow rafael-bianchi/rgt/.github/workflows/release.yml
```

Stop on any failure. Only after **both** checks succeed, extract `rgt`, confirm
`./rgt --version` matches the chosen tag, and replace the executable at your
installation's actual location (use `command -v rgt` to locate it). For
package-manager installations, use that package manager rather than manually
replacing managed files. On other platforms use the corresponding archive;
the verification procedure is the same.

Re-run `rgt init` for your configured agents if the executable path changed.
Subsequent updates use the fixed built-in verifier with attestation checks on.

The supported Cargo channel remains `cargo install --git`. Crates.io publication
is disabled while the verifier fix is vendored: Cargo's registry packaging
strips path dependencies and would otherwise substitute the unpatched verifier.

## Maintainer rules

- Never substitute locally built binaries for signed release assets.
- Keep complete checksum manifests and the original signed bundles together.
- Do not delete or move released tags to fix an upload failure.
- Release Please creates drafts; only the successful asset-verification job
  publishes them. No stable release may be published merely because builds ran.
- While a source release is pending, the formula retains the immediately
  preceding changelog release and its matching hashes. The release workflow
  updates the formula only after new verified assets have been published.
- Verify each archive with both `gh` and the verifier inside the packaged RGT
  binary. A successful upload or signing job alone is insufficient.
