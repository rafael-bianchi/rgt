#!/bin/sh
# capture-rgt-attestation-fixture.sh - captures a REAL rafael-bianchi/rgt
# GitHub Artifact Attestation bundle as the gated test fixture (spec 032).
#
# Usage:
#   scripts/capture-rgt-attestation-fixture.sh TAG [ASSET]
#
#   TAG    release tag without 'v' or with (e.g. 0.5.0 or v0.5.0)
#   ASSET  optional asset name (default rgt-v<TAG>-aarch64-apple-darwin.tar.gz)
#
# Steps (mirrors tests/fixtures/rgt/README.md):
#   1. Download the released archive, compute its SHA-256.
#   2. Query the GitHub attestations API for that digest's bundle_url.
#   3. Download the bundle and snappy-decompress it.
#   4. Write tests/fixtures/rgt/rgt-bundle.json + artifact.sha256.
#
# Fails cleanly (non-zero, no files written) when the release has no
# attestation. Requires network and the `gh` CLI; this is a dev/ops tool, not
# part of the test suite.

set -u

REPO_ROOT="$(CDPATH="" cd "$(dirname "$0")/.." && pwd)"
FIXTURE_DIR="$REPO_ROOT/tests/fixtures/rgt"
REPO="rafael-bianchi/rgt"

TAG="${1:-}"
if [ -z "$TAG" ]; then
    echo "usage: $0 TAG [ASSET]" >&2
    exit 2
fi
VERSION="${TAG#v}"
ASSET="${2:-rgt-v${VERSION}-aarch64-apple-darwin.tar.gz}"

mkdir -p "$FIXTURE_DIR"
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

ARTIFACT="$WORK/artifact"
echo "capture: downloading $ASSET for v$VERSION"
if ! curl -fsSL -o "$ARTIFACT" \
    "https://github.com/$REPO/releases/download/v$VERSION/$ASSET"; then
    echo "capture: FAIL: could not download $ASSET (does the release exist?)" >&2
    exit 1
fi

DIGEST="$(shasum -a 256 "$ARTIFACT" | awk '{print $1}')"
echo "capture: artifact sha256: $DIGEST"

# gh exits non-zero on a 404 (no attestation) — capture the status so we can
# distinguish "no attestation yet" from a bundle_url we can then fetch.
BUNDLE_URL="$(gh api "/repos/$REPO/attestations/sha256:$DIGEST" \
    --jq '.attestations[0].bundle_url' 2>/dev/null)" || BUNDLE_URL=""
if [ -z "$BUNDLE_URL" ] || [ "$BUNDLE_URL" = "null" ]; then
    echo "capture: FAIL: no attestation found for v$VERSION asset '$ASSET' (sha256:$DIGEST)" >&2
    echo "capture: a real attested release does not exist yet — nothing written." >&2
    exit 1
fi
echo "capture: bundle_url: $BUNDLE_URL"

curl -fsSL -o "$WORK/bundle.snappy" "$BUNDLE_URL" || {
    echo "capture: FAIL: could not download the attestation bundle" >&2
    exit 1
}

# Snappy-decompress the raw bundle body into Sigstore v0.3 bundle JSON.
python3 - "$WORK/bundle.snappy" "$FIXTURE_DIR/rgt-bundle.json" <<'PY' || {
    echo "capture: FAIL: could not decompress the attestation bundle" >&2
    exit 1
}
import sys
try:
    import snappy
except ImportError:
    print("capture: python-snappy not installed (pip install python-snappy); "
          "falling back to a note: decompress manually and write the JSON.",
          file=sys.stderr)
    sys.exit(2)
with open(sys.argv[1], "rb") as fh:
    data = fh.read()
with open(sys.argv[2], "wb") as fh:
    fh.write(snappy.decompress(data))
PY
if [ ! -s "$FIXTURE_DIR/rgt-bundle.json" ]; then
    echo "capture: FAIL: decompression produced an empty bundle" >&2
    rm -f "$FIXTURE_DIR/rgt-bundle.json"
    exit 1
fi
printf '%s\n' "$DIGEST" > "$FIXTURE_DIR/artifact.sha256"

echo "capture: wrote $FIXTURE_DIR/rgt-bundle.json + artifact.sha256"
echo "capture: next, run: cargo test --test test_update_attestation verifies_real_rgt_bundle_when_fixture_present"
echo "capture: if verification fails, reconcile the policy constants in src/updater/attestation.rs with the bundle's identity (see tests/fixtures/rgt/README.md)."
