#!/bin/sh
# check-version-state.sh - verifies RGT's version-bearing files agree (spec 036)
#
# Checks that Cargo.toml, .release-please-manifest.json (. key),
# Formula/rgt.rb, and the latest CHANGELOG.md heading all declare the same
# version, that the manifest carries no stale "rgt" key, and (when the
# RGT_CHECKSUMS_URL env var is set and reachable) that the Homebrew formula's
# per-arch sha256 values match the release checksums file.
#
# Network-independence (Constitution IX): SHA verification is opt-in via
# RGT_CHECKSUMS_URL and degrades to a warning when unreachable, so this script
# never depends on network availability for a basic pass.

set -u

fail() {
    echo "check-version-state: FAIL: $1" >&2
    exit 1
}

warn() {
    echo "check-version-state: WARN: $1" >&2
}

REPO_ROOT="$(CDPATH="" cd "$(dirname "$0")/.." && pwd)"

# --- 1. Extract versions from each source ------------------------------------
cargo_version="$(sed -n 's/^version = "\([^"]*\)".*/\1/p' "$REPO_ROOT/Cargo.toml" | head -n 1)"
[ -n "$cargo_version" ] || fail "could not read version from Cargo.toml"

manifest_json="$(cat "$REPO_ROOT/.release-please-manifest.json")"
manifest_version="$(printf '%s' "$manifest_json" | sed -n 's/.*"\."[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p')"
[ -n "$manifest_version" ] || fail "could not read '.' version from .release-please-manifest.json"

formula_version="$(sed -n 's/^  version "\([^"]*\)".*/\1/p' "$REPO_ROOT/Formula/rgt.rb" | head -n 1)"
[ -n "$formula_version" ] || fail "could not read version from Formula/rgt.rb"

changelog_version="$(sed -n 's/^## \[\([0-9][^]]*\)\].*/\1/p' "$REPO_ROOT/CHANGELOG.md" | head -n 1)"
[ -n "$changelog_version" ] || fail "could not read latest release heading from CHANGELOG.md"

# --- 2. Assert all four agree -------------------------------------------------
for src in "Cargo.toml:$cargo_version" "manifest:$manifest_version" "CHANGELOG:$changelog_version"; do
    name="${src%%:*}"
    ver="${src#*:}"
    if [ "$ver" != "$cargo_version" ]; then
        fail "$name version '$ver' does not match Cargo.toml version '$cargo_version'"
    fi
done
if [ "$formula_version" != "$cargo_version" ]; then
    previous_version="$(sed -n 's/^## \[\([0-9][^]]*\)\].*/\1/p' "$REPO_ROOT/CHANGELOG.md" | sed -n '2p')"
    if [ -z "$previous_version" ] || [ "$formula_version" != "$previous_version" ]; then
        fail "Formula version '$formula_version' matches neither current '$cargo_version' nor previous release '$previous_version'"
    fi
    echo "check-version-state: source release $cargo_version pending; formula retains previous release $formula_version until verified assets exist"
else
    echo "check-version-state: all version fields agree ($cargo_version)"
fi

# --- 3. Assert no stale "rgt" manifest key ------------------------------------
if printf '%s' "$manifest_json" | grep -q '"rgt"'; then
    fail ".release-please-manifest.json still contains a stale 'rgt' key"
fi
echo "check-version-state: manifest has no stale 'rgt' key"

# --- 4. Optional formula SHA verification against checksums -------------------
if [ -n "${RGT_CHECKSUMS_URL:-}" ]; then
    tmp="$(mktemp)"
    if curl -fsSL "$RGT_CHECKSUMS_URL" >"$tmp" 2>/dev/null; then
        formula_sha256="$(sed -n 's/^      sha256 "\([^"]*\)".*/\1/p' "$REPO_ROOT/Formula/rgt.rb")"
        mismatch=0
        for sha in $formula_sha256; do
            if ! grep -q "^$sha  " "$tmp"; then
                echo "check-version-state: FAIL: formula sha256 '$sha' not found in checksums file" >&2
                mismatch=1
            fi
        done
        rm -f "$tmp"
        [ "$mismatch" -eq 0 ] || exit 1
        echo "check-version-state: formula sha256 values match checksums"
    else
        rm -f "$tmp"
        warn "could not fetch RGT_CHECKSUMS_URL '$RGT_CHECKSUMS_URL' — skipping SHA verification (network required)"
    fi
fi

echo "check-version-state: OK"
