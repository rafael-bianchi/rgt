#!/bin/sh
# check-test-presence.sh - flags changed Rust source files that have no test
# coverage reference anywhere under tests/ (spec 037 US1).
#
# Usage:
#   scripts/check-test-presence.sh [DIFF_FILE]
#
# With DIFF_FILE: reads the newline-separated changed-file list from it.
# Without: computes it from `git diff --name-only origin/main...HEAD`, falling
# back to `HEAD~1` when origin/main is unavailable (so it works locally and in
# shallow CI checkouts).
#
# Exits non-zero when a changed src/*.rs (excluding main.rs/lib.rs) has no
# reference in tests/ — printing the file and a suggested test path. Docs-only
# and workflow-only diffs pass.

set -u

fail() {
    echo "check-test-presence: FAIL: $1" >&2
    exit 1
}

warn() {
    echo "check-test-presence: WARN: $1" >&2
}

REPO_ROOT="$(CDPATH="" cd "$(dirname "$0")/.." && pwd)"
cd "$REPO_ROOT"

if [ -n "${1:-}" ]; then
    diff_file="$1"
    [ -f "$diff_file" ] || fail "diff file '$diff_file' not found"
    changed=$(cat "$diff_file")
else
    BASE_REF="origin/${GITHUB_BASE_REF:-develop}"
    if git rev-parse --verify "$BASE_REF" >/dev/null 2>&1; then
        changed=$(git diff --name-only "${BASE_REF}...HEAD")
    elif git rev-parse --verify origin/main >/dev/null 2>&1; then
        changed=$(git diff --name-only origin/main...HEAD)
    else
        changed=$(git diff --name-only HEAD~1...HEAD)
    fi
fi

missing=0
for f in $changed; do
    case "$f" in
        src/*.rs) ;;
        *) continue ;;
    esac
    case "$f" in
        src/main.rs|src/lib.rs) continue ;;
    esac

    # A source file is covered when any file under tests/ references its
    # basename or its module path, or is named after its basename.
    basename=$(basename "$f" .rs)
    module_ref=$(printf '%s' "$f" | sed -E 's#^src/##; s#/#::#g; s#\.rs$##')
    if [ -n "$(find tests -name "*${basename}*.rs" -print -quit 2>/dev/null)" ]; then
        continue
    fi
    if grep -Rq --include='*.rs' -e "$basename" -e "$module_ref" tests/ 2>/dev/null; then
        continue
    fi

    echo "check-test-presence: no test reference found for '$f'" >&2
    # Suggest a plausible test path derived from the module path.
    test_hint=$(printf '%s' "$f" | sed -E 's#^src/#tests/#; s#/#_#g; s#\.rs$#.rs#')
    echo "check-test-presence: consider adding a test in a file under tests/ covering '$f' (e.g. tests/unit/${f#src/}, or a reference to '${module_ref}')" >&2
    missing=1
done

if [ "$missing" -ne 0 ]; then
    exit 1
fi

echo "check-test-presence: all changed Rust source files have test references"
exit 0
