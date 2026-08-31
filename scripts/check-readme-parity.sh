#!/bin/sh
# check-readme-parity.sh - verifies the translated READMEs track the English
# feature set (spec 038). Offline and deterministic (Constitution IX).

set -u

fail() {
    echo "check-readme-parity: FAIL: $1" >&2
    exit 1
}

REPO_ROOT="$(CDPATH="" cd "$(dirname "$0")/.." && pwd)"
cd "$REPO_ROOT"

# Per-language required content markers (localized): Doctor section,
# Update/attestation flow, gc, --number-format, non-text-source callout.
# Matching is lenient (grep -qi, case-insensitive) and term-based so CJK and
# accented languages all pass when the content is present.
require_terms() {
    file="$1"; shift
    for term in "$@"; do
        if ! grep -qi "$term" "$file"; then
            fail "$file is missing required content matching '$term'"
        fi
    done
}

require_terms README_es.md \
    "doctor" "attestation\|attestaci" "rgt gc" "number-format" "binario" "no puede ver"

require_terms README_fr.md \
    "doctor" "attestation" "rgt gc" "number-format" "binaire" "ne peut pas voir"

require_terms README_ja.md \
    "doctor" "attestation\|証明" "rgt gc" "number-format" "バイナリ" "見ることができ"

require_terms README_ko.md \
    "doctor" "attestation\|증명" "rgt gc" "number-format" "바이너리" "볼 수 없"

require_terms README_pt.md \
    "doctor" "attest" "rgt gc" "number-format" "binário" "não consegue ver"

require_terms README_zh.md \
    "doctor" "attestation\|证明\|验证" "rgt gc" "number-format" "二进制" "无法查看\|无法读取"

# Language-neutral markers for the Duration (0.6) and Turtle export (0.6)
# features that must survive alongside the 0.5 hardening content.
for f in README_es.md README_fr.md README_ja.md README_ko.md README_pt.md README_zh.md; do
    require_terms "$f" "DURATION_SUM" "DURATION_AVG" "graph --format ttl\|ttl"
done

# Language-switcher bar: every README (incl. English) must link all 7 variants.
all_langs="README.md README_fr.md README_zh.md README_ja.md README_ko.md README_es.md README_pt.md"
for f in $all_langs; do
    [ -f "$f" ] || fail "$f missing"
    for link in "README.md" "README_fr.md" "README_zh.md" "README_ja.md" "README_ko.md" "README_es.md" "README_pt.md"; do
        if ! grep -q "$link" "$f"; then
            fail "$f language bar missing $link"
        fi
    done
done

# Markdown integrity: balanced fenced code blocks per file.
for f in $all_langs; do
    fences=$(grep -c '^```' "$f" || true)
    if [ $((fences % 2)) -ne 0 ]; then
        fail "$f has an odd number of fenced code blocks ($fences)"
    fi
done

echo "check-readme-parity: OK"
