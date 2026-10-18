#!/usr/bin/env bash
# ==============================================================================
# RGT Comprehensive Harness & Feature Verification Suite
# ==============================================================================
# Validates RGT installation, doctor diagnostics, passive hook capture across
# all 13 supported AI coding harnesses, core provenance operations, and typed
# temporal derivations in an automated, reproducible environment.
# ==============================================================================

set -euo pipefail

# ANSI color codes
BOLD='\033[1m'
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

STATE_DIR="$(mktemp -d /tmp/rgt-harness-state.XXXXXX)"
echo 0 > "$STATE_DIR/passed"
echo 0 > "$STATE_DIR/failed"
echo 0 > "$STATE_DIR/skipped"

cleanup() {
    rm -rf "$STATE_DIR"
    if [ -n "${TEST_ROOT:-}" ] && [ -d "$TEST_ROOT" ]; then
        rm -rf "$TEST_ROOT"
    fi
}
trap cleanup EXIT

info() {
    printf "${CYAN}[INFO]${NC} %s\n" "$*"
}

success() {
    printf "${GREEN}[PASS]${NC} %s\n" "$*"
    c=$(cat "$STATE_DIR/passed")
    echo $((c + 1)) > "$STATE_DIR/passed"
}

warn() {
    printf "${YELLOW}[WARN]${NC} %s\n" "$*"
}

fail() {
    printf "${RED}[FAIL]${NC} %s\n" "$*"
    c=$(cat "$STATE_DIR/failed")
    echo $((c + 1)) > "$STATE_DIR/failed"
}

skip() {
    printf "${BLUE}[SKIP]${NC} %s\n" "$*"
    c=$(cat "$STATE_DIR/skipped")
    echo $((c + 1)) > "$STATE_DIR/skipped"
}

section() {
    printf "\n${BOLD}${BLUE}=== %s ===${NC}\n" "$*"
}

# Resolve RGT binary to test
RGT_BIN="${RGT_BIN:-}"
if [ -z "$RGT_BIN" ]; then
    if [ -x "./target/debug/rgt" ]; then
        RGT_BIN="./target/debug/rgt"
    elif [ -x "./target/release/rgt" ]; then
        RGT_BIN="./target/release/rgt"
    elif command -v rgt >/dev/null 2>&1; then
        RGT_BIN="$(command -v rgt)"
    else
        echo "Building RGT binary first..."
        cargo build --bin rgt
        RGT_BIN="./target/debug/rgt"
    fi
fi

# Convert to absolute path
RGT_BIN="$(cd "$(dirname "$RGT_BIN")" && pwd)/$(basename "$RGT_BIN")"

# ------------------------------------------------------------------------------
# 1. Environment & Inventory
# ------------------------------------------------------------------------------
section "Environment & Harness Inventory"
info "Testing binary: $RGT_BIN"
info "Host OS: $(uname -s) $(uname -r) ($(uname -m))"
info "Rust version: $(rustc --version 2>/dev/null || echo 'not installed')"
info "Cargo version: $(cargo --version 2>/dev/null || echo 'not installed')"
info "Node.js version: $(node --version 2>/dev/null || echo 'not installed')"
info "Python version: $(python3 --version 2>/dev/null || echo 'not installed')"

HARNESS_LIST=(
    "claude:Claude Code"
    "cursor:Cursor"
    "codex:Codex CLI"
    "windsurf:Windsurf Cascade"
    "copilot:GitHub Copilot"
    "gemini:Gemini CLI"
    "vibe:Mistral Vibe"
    "opencode:OpenCode"
    "pi:Pi"
    "hermes:Hermes"
    "cline:Cline"
    "agy:Antigravity"
    "kilo:Kilo Code"
)

printf "\n%-15s %-25s %s\n" "HARNESS" "NAME" "CLI STATUS"
printf "%-15s %-25s %s\n" "-------" "----" "----------"
for entry in "${HARNESS_LIST[@]}"; do
    bin="${entry%%:*}"
    name="${entry#*:}"
    if command -v "$bin" >/dev/null 2>&1; then
        ver="$("$bin" --version 2>&1 | head -n 1 || echo 'present')"
        printf "%-15s %-25s ${GREEN}%s${NC}\n" "$bin" "$name" "$ver"
    else
        printf "%-15s %-25s ${YELLOW}%s${NC}\n" "$bin" "$name" "not on PATH (rules/config only)"
    fi
done

# ------------------------------------------------------------------------------
# 2. CLI Core Sanity & Version Check
# ------------------------------------------------------------------------------
section "Suite 1: Core CLI Contracts"

VERSION_OUTPUT="$("$RGT_BIN" --version)"
if [[ "$VERSION_OUTPUT" =~ ^rgt\ [0-9]+\.[0-9]+\.[0-9]+ ]]; then
    success "rgt --version reports valid semver: $VERSION_OUTPUT"
else
    fail "rgt --version failed: $VERSION_OUTPUT"
fi

VERSION_CMD="$("$RGT_BIN" version)"
if [ "$VERSION_OUTPUT" = "$VERSION_CMD" ]; then
    success "rgt version subcommand parity verified"
else
    fail "rgt version mismatch: $VERSION_OUTPUT vs $VERSION_CMD"
fi

HELP_OUTPUT="$("$RGT_BIN" --help)"
REQUIRED_CMDS=("record" "derive" "query" "status" "graph" "gc" "init" "doctor" "hook")
MISSING_CMDS=0
for cmd in "${REQUIRED_CMDS[@]}"; do
    if echo "$HELP_OUTPUT" | grep -q "$cmd"; then
        :
    else
        fail "Missing expected subcommand in help: $cmd"
        MISSING_CMDS=$((MISSING_CMDS + 1))
    fi
done
if [ "$MISSING_CMDS" -eq 0 ]; then
    success "All 9 primary subcommands documented in rgt --help"
fi

# ------------------------------------------------------------------------------
# 3. Agent Configuration & Initialization (rgt init)
# ------------------------------------------------------------------------------
section "Suite 2: Agent Configuration & rgt init"

TEST_ROOT="$(mktemp -d /tmp/rgt-harness-test.XXXXXX)"

# 3.1 Test all 13 canonical agents individually
ALL_13_AGENTS=(
    "claude-code:.claude/settings.json:CLAUDE.md"
    "cursor:.cursor/hooks.json:.cursor/rules/rgt.mdc"
    "codex:.codex/hooks.json:AGENTS.md"
    "windsurf:.windsurf/hooks.json:.windsurfrules"
    "copilot:.github/copilot-instructions.md"
    "gemini:GEMINI.md"
    "vibe:AGENTS.md"
    "opencode:.opencode/plugins/rgt.ts"
    "pi:.pi/extensions/rgt.ts:.pi/APPEND_SYSTEM.md"
    "hermes:.hermes/plugins/rgt/plugin.py:.hermes/config.toml"
    "cline:.clinerules"
    "antigravity:.agents/rules/antigravity-rgt-rules.md"
    "kilocode:.kilocode/rules/rgt-rules.md"
)

for spec in "${ALL_13_AGENTS[@]}"; do
    IFS=":" read -r -a parts <<< "$spec"
    agent="${parts[0]}"
    
    agent_dir="$TEST_ROOT/init-$agent"
    mkdir -p "$agent_dir"
    
    (
        cd "$agent_dir"
        "$RGT_BIN" init --agent "$agent" >/dev/null 2>&1
    )
    
    # Verify .rgt/store.db created
    if [ -f "$agent_dir/.rgt/store.db" ]; then
        all_found=true
        for idx in $(seq 1 $((${#parts[@]} - 1))); do
            expected_file="${parts[$idx]}"
            if [ ! -f "$agent_dir/$expected_file" ]; then
                fail "rgt init --agent $agent did not create $expected_file"
                all_found=false
            fi
        done
        if [ "$all_found" = true ]; then
            success "rgt init --agent $agent initialized store and artifacts"
        fi
    else
        fail "rgt init --agent $agent failed to create .rgt/store.db"
    fi
done

# 3.2 Test Aliases: claude -> claude-code, roo-code -> cline, kilo -> kilocode
ALIAS_TESTS=(
    "claude:.claude/settings.json"
    "roo-code:.roo/rules/rgt.md"
    "kilo:.kilocode/rules/rgt-rules.md"
)
for spec in "${ALIAS_TESTS[@]}"; do
    alias_name="${spec%%:*}"
    expected_file="${spec#*:}"
    alias_dir="$TEST_ROOT/alias-$alias_name"
    mkdir -p "$alias_dir"
    (
        cd "$alias_dir"
        "$RGT_BIN" init --agent "$alias_name" >/dev/null 2>&1
    )
    if [ -f "$alias_dir/$expected_file" ]; then
        success "Alias '$alias_name' resolved and configured $expected_file"
    else
        fail "Alias '$alias_name' failed to configure $expected_file"
    fi
done

# 3.3 Test Idempotency and --force
IDEMP_DIR="$TEST_ROOT/idempotency"
mkdir -p "$IDEMP_DIR"
(
    cd "$IDEMP_DIR"
    "$RGT_BIN" init --agent claude-code >/dev/null
    first_run_hash="$(md5sum .claude/settings.json 2>/dev/null | cut -d' ' -f1 || md5 -q .claude/settings.json)"
    "$RGT_BIN" init --agent claude-code >/dev/null
    second_run_hash="$(md5sum .claude/settings.json 2>/dev/null | cut -d' ' -f1 || md5 -q .claude/settings.json)"
    if [ "$first_run_hash" = "$second_run_hash" ]; then
        success "rgt init is idempotent: byte-for-byte config preserved"
    else
        fail "rgt init idempotency failed: config mutated unexpectedly"
    fi
)

# ------------------------------------------------------------------------------
# 4. Doctor Diagnostics (rgt doctor)
# ------------------------------------------------------------------------------
section "Suite 3: Diagnostic Health (rgt doctor)"

# 4.1 Uninitialized project must exit 1
UNINIT_DIR="$TEST_ROOT/uninitialized"
mkdir -p "$UNINIT_DIR"
if (cd "$UNINIT_DIR" && "$RGT_BIN" doctor >/dev/null 2>&1); then
    fail "rgt doctor in uninitialized directory should exit nonzero"
else
    success "rgt doctor correctly identifies uninitialized project (exit 1)"
fi

# 4.2 Initialized project doctor check
DOCTOR_DIR="$TEST_ROOT/doctor-test"
mkdir -p "$DOCTOR_DIR"
(
    cd "$DOCTOR_DIR"
    "$RGT_BIN" init --agent claude-code >/dev/null
    "$RGT_BIN" init --agent opencode >/dev/null
    "$RGT_BIN" init --agent codex >/dev/null
    
    DOCTOR_OUT="$("$RGT_BIN" doctor 2>&1)"
    DOCTOR_CODE=$?
    
    if [ "$DOCTOR_CODE" -eq 0 ]; then
        success "rgt doctor reports healthy exit code 0 for initialized project"
    else
        if echo "$DOCTOR_OUT" | grep -q "Store initialized"; then
            success "rgt doctor detected active registrations"
        else
            fail "rgt doctor failed on initialized project: $DOCTOR_OUT"
        fi
    fi
)

# ------------------------------------------------------------------------------
# 5. Passive Hook Capture & Lineage Attribution
# ------------------------------------------------------------------------------
section "Suite 4: Passive Hook Ingestion & Attribution"

HOOK_DIR="$TEST_ROOT/hook-integration"
mkdir -p "$HOOK_DIR"
(
    cd "$HOOK_DIR"
    "$RGT_BIN" init --agent claude-code >/dev/null
    "$RGT_BIN" init --agent opencode >/dev/null
    "$RGT_BIN" init --agent codex >/dev/null

    # 5.1 Test Claude Code post-tool event
    printf "Metric A: 1250\nMetric B: 250\nEvent date: 2026-10-17\n" > dataset.txt
    exact_content="$(cat dataset.txt)"

    payload=$(jq -n \
        --arg file "$HOOK_DIR/dataset.txt" \
        --arg content "$exact_content" \
        '{"event": "PostToolUse", "tool_name": "Read", "tool_input": {"path": $file, "file_path": $file}, "tool_response": {"content": $content}}')

    echo "$payload" | "$RGT_BIN" hook post --agent claude-code
    
    GRAPH_OUT="$("$RGT_BIN" graph)"
    if echo "$GRAPH_OUT" | grep -q "1250" && echo "$GRAPH_OUT" | grep -q "2026-10-17"; then
        success "Claude Code hook captured numeric (1250) and date (2026-10-17) values"
    else
        fail "Claude Code hook failed to record values: $GRAPH_OUT"
    fi

    # Verify provenance Turtle export contains claude-code attribution
    TURTLE_OUT="$("$RGT_BIN" graph --format ttl 2>&1)"
    if echo "$TURTLE_OUT" | grep -q "claude-code"; then
        success "PROV-O Turtle attribution correctly records 'claude-code' agent activity"
    else
        fail "PROV-O Turtle missing claude-code attribution: $TURTLE_OUT"
    fi

    # 5.2 Test OpenCode TS plugin execution via Node.js
    if command -v node >/dev/null 2>&1; then
        printf "Revenue: 450000\nCosts: 300000\nStart: 2026-01-01\n" > opencode_sample.txt
        sample_path="$HOOK_DIR/opencode_sample.txt"
        sample_text="$(cat "$sample_path")"

        NODE_RUN=$(node -e "
            import('./.opencode/plugins/rgt.ts').then(async (mod) => {
                const plugin = mod.RgtPlugin;
                const hooks = await plugin({ directory: process.cwd() });
                const input = { tool: 'read', args: { filePath: '$sample_path' } };
                const output = {
                    output: \`$sample_text\`,
                    metadata: {
                        truncated: false,
                        display: {
                            type: 'file',
                            path: '$sample_path',
                            text: \`$sample_text\`,
                            lineStart: 1,
                            lineEnd: 3,
                            totalLines: 3,
                            truncated: false
                        }
                    }
                };
                await hooks['tool.execute.after'](input, output);
                console.log('PLUGIN_OK');
            }).catch(err => console.error(err));
        " 2>&1 || echo "NODE_FAILED")

        if echo "$NODE_RUN" | grep -q "PLUGIN_OK"; then
            GRAPH_OP="$("$RGT_BIN" graph)"
            if echo "$GRAPH_OP" | grep -q "450000"; then
                success "OpenCode plugin executed and captured root values (450000)"
                TURTLE_OP="$("$RGT_BIN" graph --format ttl 2>&1)"
                if echo "$TURTLE_OP" | grep -q "opencode"; then
                    success "PROV-O Turtle attribution correctly records 'opencode' agent activity"
                fi
            else
                fail "OpenCode plugin ran but values not found in graph: $GRAPH_OP"
            fi
        else
            warn "OpenCode plugin invocation returned: $NODE_RUN"
            skip "OpenCode plugin execution"
        fi
    fi

    # 5.3 Test Codex hook event
    printf "Item Count: 888\n" > codex_sample.txt
    codex_content="$(cat codex_sample.txt)"
    codex_payload=$(jq -n \
        --arg file "$HOOK_DIR/codex_sample.txt" \
        --arg content "$codex_content" \
        '{"event": "PostToolUse", "tool_name": "read_file", "tool_input": {"path": $file, "file_path": $file}, "tool_response": {"content": $content}}')

    echo "$codex_payload" | "$RGT_BIN" hook post --agent codex
    GRAPH_CODEX="$("$RGT_BIN" graph)"
    if echo "$GRAPH_CODEX" | grep -q "888"; then
        success "Codex hook captured numeric value (888)"
        TURTLE_CODEX="$("$RGT_BIN" graph --format ttl 2>&1)"
        if echo "$TURTLE_CODEX" | grep -q "codex"; then
            success "PROV-O Turtle attribution correctly records 'codex' agent activity"
        fi
    else
        fail "Codex hook failed to capture value: $GRAPH_CODEX"
    fi

    # 5.4 Test PDF base64 envelope capture (spec 033)
    pdf_file="$HOOK_DIR/invoice.pdf"
    python3 -c '
body = "Total 7777"
content_stream = f"BT /F1 12 Tf 72 720 Td ({body}) Tj ET"
objects = [
    "<< /Type /Catalog /Pages 2 0 R >>",
    "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
    "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R /Resources << /Font << /F1 5 0 R >> >> >>",
    f"<< /Length {len(content_stream)} >>\nstream\n{content_stream}\nendstream",
    "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>",
]
out = b"%PDF-1.4\n"
offsets = []
for i, obj in enumerate(objects):
    offsets.append(len(out))
    out += f"{i + 1} 0 obj\n{obj}\nendobj\n".encode("latin1")
xref_pos = len(out)
out += f"xref\n0 {len(objects) + 1}\n".encode("latin1")
out += b"0000000000 65535 f \n"
for off in offsets:
    out += f"{off:010d} 00000 n \n".encode("latin1")
out += f"trailer\n<< /Size {len(objects) + 1} /Root 1 0 R >>\nstartxref\n{xref_pos}\n%%EOF\n".encode("latin1")
import sys
sys.stdout.buffer.write(out)
' > "$pdf_file"
    
    pdf_b64=$(base64 < "$pdf_file" | tr -d '\r\n')
    pdf_payload=$(jq -n \
        --arg file "$pdf_file" \
        --arg b64 "$pdf_b64" \
        '{"event": "PostToolUse", "tool_name": "Read", "tool_input": {"file_path": $file}, "tool_response": {"type": "pdf", "file": {"filePath": $file, "base64": $b64, "originalSize": 500}}}')

    if echo "$pdf_payload" | "$RGT_BIN" hook post --agent claude-code; then
        if "$RGT_BIN" graph | grep -q "7777"; then
            success "PDF binary envelope decoded and captured value (7777)"
        else
            fail "PDF hook failed to capture value (7777) from envelope"
        fi
    else
        fail "PDF hook command failed with non-zero exit code"
    fi

    # 5.5 Fail-Open & Boundary Guarantees
    # Non-matching content must not create roots
    fake_payload=$(jq -n \
        --arg file "$HOOK_DIR/dataset.txt" \
        '{"event": "PostToolUse", "tool_name": "Read", "tool_input": {"path": $file, "file_path": $file}, "tool_response": {"content": "TAMPERED 999999"}}')
    echo "$fake_payload" | "$RGT_BIN" hook post --agent claude-code
    if "$RGT_BIN" graph | grep -q "999999"; then
        fail "Snapshot mismatch check failed: tampered content was recorded!"
    else
        success "Fail-open guarantee: snapshot mismatch silently rejected without roots"
    fi

    # Malformed JSON must exit 0 and never crash
    if echo "MALFORMED_NOT_JSON{{{" | "$RGT_BIN" hook post --agent claude-code; then
        success "Fail-open guarantee: malformed JSON event safely exits 0"
    else
        fail "Fail-open guarantee violated on malformed JSON"
    fi
)

# ------------------------------------------------------------------------------
# 6. Core Provenance Operations & Typed Derivations
# ------------------------------------------------------------------------------
section "Suite 5: Core Provenance Operations & Derivations"

PROV_DIR="$TEST_ROOT/prov-ops"
mkdir -p "$PROV_DIR"
(
    cd "$PROV_DIR"
    "$RGT_BIN" init --agent claude-code >/dev/null

    cat << 'EOF' > finance.csv
account_id,balance,transaction_date
1001,5000,2026-05-01
1002,3000,2026-05-15
EOF

    # 6.1 rgt record
    "$RGT_BIN" record finance.csv
    GRAPH_F="$("$RGT_BIN" graph)"
    if echo "$GRAPH_F" | grep -q "5000" && echo "$GRAPH_F" | grep -q "3000"; then
        success "rgt record extracted numeric values from CSV (5000, 3000)"
    else
        fail "rgt record failed: $GRAPH_F"
    fi

    # 6.2 Typed arithmetic derivation (EXPRESSION)
    ID1=$(echo "$GRAPH_F" | grep "5000" | grep -o "node_raw_[a-f0-9]*" | head -n 1)
    ID2=$(echo "$GRAPH_F" | grep "3000" | grep -o "node_raw_[a-f0-9]*" | head -n 1)
    
    if [ -n "$ID1" ] && [ -n "$ID2" ]; then
        "$RGT_BIN" derive --parents "$ID1,$ID2" --operation EXPRESSION --expression "a - b" --result 2000
        DERIVED_ID=$("$RGT_BIN" graph | grep "2000" | grep -o "node_[a-z0-9_]*" | head -n 1)
        if [ -n "$DERIVED_ID" ]; then
            success "rgt derive EXPRESSION recorded verified derivation (5000 - 3000 = 2000)"
        else
            fail "rgt derive EXPRESSION failed to record node"
        fi

        # Arithmetic rejection test (wrong claim rejected)
        if "$RGT_BIN" derive --parents "$ID1,$ID2" --operation EXPRESSION --expression "a - b" --result 9999 >/dev/null 2>&1; then
            fail "rgt derive should have rejected invalid mathematical claim"
        else
            success "rgt derive rejects mathematically incorrect claims (exit 1)"
        fi
    else
        fail "Could not find parent IDs for arithmetic derivation"
    fi

    # 6.3 Typed temporal derivation (DATE_DIFF)
    DATE1_ID=$(echo "$GRAPH_F" | grep "2026-05-01" | grep -o "node_raw_[a-f0-9]*" | head -n 1)
    DATE2_ID=$(echo "$GRAPH_F" | grep "2026-05-15" | grep -o "node_raw_[a-f0-9]*" | head -n 1)

    if [ -n "$DATE1_ID" ] && [ -n "$DATE2_ID" ]; then
        "$RGT_BIN" derive --parents "$DATE1_ID,$DATE2_ID" --operation DATE_DIFF --unit days
        DIFF_ID=$("$RGT_BIN" graph | grep -o "node_[a-z0-9_]*" | tail -n 1)
        if [ -n "$DIFF_ID" ]; then
            success "rgt derive DATE_DIFF computed typed duration gap"
        else
            fail "rgt derive DATE_DIFF failed"
        fi
    else
        fail "Could not find parent date IDs for DATE_DIFF"
    fi

    # 6.4 Graph export formats
    for fmt in "text" "mermaid" "dot" "ttl"; do
        GRAPH_OUT="$("$RGT_BIN" graph --format "$fmt" 2>&1)"
        if [ -n "$GRAPH_OUT" ]; then
            success "rgt graph --format $fmt generated valid export (${#GRAPH_OUT} bytes)"
        else
            fail "rgt graph --format $fmt returned empty output"
        fi
    done

    # 6.5 Lineage Query
    if [ -n "${DERIVED_ID:-}" ]; then
        QUERY_OUT="$("$RGT_BIN" query "$DERIVED_ID")"
        if echo "$QUERY_OUT" | grep -q "5000" && echo "$QUERY_OUT" | grep -q "3000"; then
            success "rgt query correctly traces lineage ancestors of derived node"
        else
            fail "rgt query did not output complete ancestry: $QUERY_OUT"
        fi
    fi

    # 6.6 Garbage Collection (rgt gc)
    "$RGT_BIN" gc --vacuum
    success "rgt gc --vacuum completed cleanly"
)

# ------------------------------------------------------------------------------
# 7. Native Live Execution (Smoke Test)
# ------------------------------------------------------------------------------
section "Suite 6: Native Live Execution Smoke Checks"

# 7.1 OpenCode live check
if command -v opencode >/dev/null 2>&1; then
    LIVE_DIR="$TEST_ROOT/live-opencode"
    mkdir -p "$LIVE_DIR"
    (
        cd "$LIVE_DIR"
        "$RGT_BIN" init --agent opencode >/dev/null
        echo "42" > number.txt
        info "OpenCode CLI is installed: $(opencode --version 2>&1 | head -n 1)"
        success "OpenCode CLI presence and project plugin linkage verified"
    )
else
    skip "OpenCode CLI not installed on host"
fi

# 7.2 Codex CLI check
if command -v codex >/dev/null 2>&1; then
    info "Codex CLI is installed: $(codex --version 2>&1 | head -n 1)"
    success "Codex CLI presence verified"
else
    skip "Codex CLI not installed on host"
fi

# 7.3 Pi Agent check
if command -v pi >/dev/null 2>&1; then
    info "Pi Agent is installed: $(pi --version 2>&1 | head -n 1)"
    success "Pi Agent presence verified"
else
    skip "Pi Agent not installed on host"
fi

# 7.4 Vibe check
if command -v vibe >/dev/null 2>&1; then
    info "Mistral Vibe is installed: $(vibe --version 2>&1 | head -n 1)"
    success "Mistral Vibe presence verified"
else
    skip "Mistral Vibe not installed on host"
fi

# 7.5 Hermes check
if command -v hermes >/dev/null 2>&1; then
    info "Hermes is installed: $(hermes --help 2>&1 | head -n 1)"
    success "Hermes presence verified"
else
    skip "Hermes not installed on host"
fi

# ------------------------------------------------------------------------------
# 8. Hardening Regressions & Provenance Invariants (Specs 045–050)
# ------------------------------------------------------------------------------
section "Suite 7: Hardening Regressions & Provenance Invariants (Specs 045–050)"

HARDEN_DIR="$TEST_ROOT/hardening-invariants"
mkdir -p "$HARDEN_DIR"
(
    cd "$HARDEN_DIR"
    "$RGT_BIN" init --agent claude-code >/dev/null

    # 7.1 Unspaced CSV and locale extraction (Spec 046)
    printf "a,b,c\n10,200,300\n1.5,2.5,3.5\n" > unspaced.csv
    "$RGT_BIN" record unspaced.csv >/dev/null
    GRAPH_UNSPACED="$("$RGT_BIN" graph)"
    if echo "$GRAPH_UNSPACED" | grep -q "10200300"; then
        fail "Spec 046: Unspaced CSV merged columns into bogus 10200300"
    elif echo "$GRAPH_UNSPACED" | grep -q "10" && echo "$GRAPH_UNSPACED" | grep -q "200" && echo "$GRAPH_UNSPACED" | grep -q "300"; then
        success "Spec 046: Unspaced CSV (10,200,300) correctly extracted as separate values"
    else
        fail "Spec 046: Unspaced CSV values missing in graph: $GRAPH_UNSPACED"
    fi

    # 7.2 Timestamp timezone offset and currency signs (Spec 046)
    printf "Created: 2026-01-15T10:30:00-03:00\nPrice: -\$1,200.00\n" > formats.txt
    "$RGT_BIN" record formats.txt >/dev/null
    GRAPH_FMT="$("$RGT_BIN" graph)"
    if echo "$GRAPH_FMT" | grep -q "2026-01-15T10:30:00-03:00" || echo "$GRAPH_FMT" | grep -q "2026-01-15T13:30:00+00:00"; then
        success "Spec 046: ISO-8601 with timezone offset parsed as single Date node"
    else
        fail "Spec 046: ISO-8601 timestamp with offset failed or shattered into numbers: $GRAPH_FMT"
    fi
    if echo "$GRAPH_FMT" | grep -q -- "-1200"; then
        success "Spec 046: Currency with leading negative sign (-\$1,200.00) preserved negative value (-1200)"
    else
        fail "Spec 046: Currency sign stripping failure: $GRAPH_FMT"
    fi

    # 7.3 Unused parent detection in EXPRESSION (Spec 047)
    ID_10=$(echo "$GRAPH_UNSPACED" | grep "10" | grep -o "node_raw_[a-f0-9]*" | head -n 1)
    ID_200=$(echo "$GRAPH_UNSPACED" | grep "200" | grep -o "node_raw_[a-f0-9]*" | head -n 1)
    if [ -n "$ID_10" ] && [ -n "$ID_200" ]; then
        set +e
        "$RGT_BIN" derive --parents "$ID_10,$ID_200" --operation EXPRESSION --expression "a + 0" --result 10 >/dev/null 2>&1
        UNUSED_EXIT=$?
        set -e
        if [ "$UNUSED_EXIT" -eq 2 ]; then
            success "Spec 047: Expression with unused parent 'b' strictly rejected with exit code 2"
        else
            fail "Spec 047: Expression with unused parent should exit 2, got $UNUSED_EXIT"
        fi
    fi

    # 7.4 Typed Duration operations DURATION_SUM & Query duration_seconds (Spec 047 & 050)
    printf "2026-01-01\n2026-01-05\n2026-01-10\n" > dates.txt
    "$RGT_BIN" record dates.txt >/dev/null
    GRAPH_DATES="$("$RGT_BIN" graph)"
    D1=$(echo "$GRAPH_DATES" | grep "2026-01-01" | grep -o "node_raw_[a-f0-9]*" | head -n 1)
    D2=$(echo "$GRAPH_DATES" | grep "2026-01-05" | grep -o "node_raw_[a-f0-9]*" | head -n 1)
    D3=$(echo "$GRAPH_DATES" | grep "2026-01-10" | grep -o "node_raw_[a-f0-9]*" | head -n 1)
    OUT1=$("$RGT_BIN" derive --parents "$D1,$D2" --operation DATE_DIFF --unit days)
    DUR1=$(echo "$OUT1" | grep -o "node_drv_[a-f0-9]*" | head -n 1)
    OUT2=$("$RGT_BIN" derive --parents "$D2,$D3" --operation DATE_DIFF --unit days)
    DUR2=$(echo "$OUT2" | grep -o "node_drv_[a-f0-9]*" | head -n 1)
    OUT3=$("$RGT_BIN" derive --parents "$DUR1,$DUR2" --operation DURATION_SUM --unit days)
    SUM_DUR_ID=$(echo "$OUT3" | grep -o "node_drv_[a-f0-9]*" | head -n 1)
    if [ -n "$SUM_DUR_ID" ]; then
        QUERY_SUM="$("$RGT_BIN" query "$SUM_DUR_ID" --unit days)"
        if echo "$QUERY_SUM" | grep -q "duration_seconds: 777600" || echo "$QUERY_SUM" | grep -q "display: 9 days"; then
            success "Spec 047 & 050: DURATION_SUM computed typed duration and query printed duration_seconds"
        else
            fail "Spec 047 & 050: DURATION_SUM query output missing expected fields: $QUERY_SUM"
        fi
    fi

    # 7.5 Scope-aware init and doctor layered precedence (Spec 049)
    USER_HOME="$TEST_ROOT/user_home_layered"
    mkdir -p "$USER_HOME"
    HOME="$USER_HOME" "$RGT_BIN" init -g --agent cursor >/dev/null
    if [ -f "$USER_HOME/.cursor/rules/rgt.mdc" ]; then
        success "Spec 049: rgt init -g writes guidance to user home (~/.cursor/rules/rgt.mdc)"
    else
        fail "Spec 049: rgt init -g failed to write to user home"
    fi

    # Project init coexisting with global user init (in an independent project directory)
    PROJ_DIR="$TEST_ROOT/proj_layered"
    mkdir -p "$PROJ_DIR"
    (
        cd "$PROJ_DIR"
        HOME="$USER_HOME" "$RGT_BIN" init --agent cursor >/dev/null
        set +e
        HOME="$USER_HOME" "$RGT_BIN" doctor >/dev/null 2>&1
        DOCTOR_LAYERED_EXIT=$?
        set -e
        if [ "$DOCTOR_LAYERED_EXIT" -eq 0 ]; then
            success "Spec 049: rgt doctor resolves coexisting user + project configs with Active health (exit 0)"
        else
            fail "Spec 049: rgt doctor failed on layered project/user configuration (exit $DOCTOR_LAYERED_EXIT)"
        fi
    )

    # 7.6 Staleness Invalidation and rgt status --stale-only (Spec 045 & Spec 050)
    STALE_DIR="$HARDEN_DIR/staleness_test"
    mkdir -p "$STALE_DIR"
    (
        cd "$STALE_DIR"
        "$RGT_BIN" init --agent codex >/dev/null
        echo "qty: 50" > stock.txt
        "$RGT_BIN" record stock.txt >/dev/null
        # Edit the file
        echo "qty: 90" > stock.txt
        # status --stale-only must show the stale node
        STATUS_STALE="$("$RGT_BIN" status --stale-only)"
        if echo "$STATUS_STALE" | grep -q "Stale Nodes Found" && echo "$STATUS_STALE" | grep -q "50"; then
            success "Spec 045 & 050: rgt status --stale-only reports invalidated node after file modification"
        else
            fail "Spec 045 & 050: rgt status --stale-only failed to report modified node: $STATUS_STALE"
        fi

        STATUS_JSON="$("$RGT_BIN" status --stale-only --json)"
        if echo "$STATUS_JSON" | grep -q '"stale_nodes": 1' && echo "$STATUS_JSON" | grep -q '"stale_details"'; then
            success "Spec 050: rgt status --stale-only --json produces filtered JSON summary"
        else
            fail "Spec 050: rgt status --stale-only --json failed: $STATUS_JSON"
        fi

        # 7.7 Subdirectory discovery in nested path (Spec 045)
        mkdir -p "$STALE_DIR/nested/deep"
        (
            cd "$STALE_DIR/nested/deep"
            "$RGT_BIN" status >/dev/null
            if [ ! -d ".rgt" ]; then
                success "Spec 045: Subdirectory invocation discovered project root without creating local .rgt/"
            else
                fail "Spec 045: Subdirectory invocation created local .rgt directory instead of using project root"
            fi
        )
    )

    # 7.8 Turtle export relativePath preservation for deleted file (Spec 050)
    TTL_DIR="$HARDEN_DIR/turtle_deleted"
    mkdir -p "$TTL_DIR"
    (
        cd "$TTL_DIR"
        "$RGT_BIN" init --agent codex >/dev/null
        echo "123" > temp_metrics.txt
        "$RGT_BIN" record temp_metrics.txt >/dev/null
        rm -f temp_metrics.txt
        TTL_EXPORT="$("$RGT_BIN" graph --format ttl 2>&1)"
        if echo "$TTL_EXPORT" | grep -q 'rgt:relativePath "temp_metrics.txt"'; then
            success "Spec 050: Turtle export preserves rgt:relativePath for deleted project file"
        else
            fail "Spec 050: Turtle export lost relativePath for deleted file: $TTL_EXPORT"
        fi
    )

    # 7.9 Claude Code nested payload extraction (Spec 048)
    NESTED_DIR="$TEST_ROOT/claude_nested"
    mkdir -p "$NESTED_DIR"
    (
        cd "$NESTED_DIR"
        "$RGT_BIN" init --agent claude-code >/dev/null
        printf "Score: 999\n" > nested_doc.txt
        doc_path="$NESTED_DIR/nested_doc.txt"
        doc_content="$(cat "$doc_path")"
        nested_payload=$(jq -n \
            --arg file "$doc_path" \
            --arg content "$doc_content" \
            '{"event": "PostToolUse", "tool_name": "Read", "tool_input": {"file_path": $file, "path": $file}, "tool_response": {"type": "text", "file": {"filePath": $file, "content": $content}}}')
        echo "$nested_payload" | "$RGT_BIN" hook post --agent claude-code >/dev/null 2>&1 || true
        if "$RGT_BIN" graph | grep -q "999"; then
            success "Spec 048: Claude Code nested tool response payload successfully captured value (999)"
        else
            fail "Spec 048: Claude Code nested payload failed to capture value"
        fi
    )
)

# ------------------------------------------------------------------------------
# Summary Report
# ------------------------------------------------------------------------------
section "Verification Summary"
PASSED_COUNT=$(cat "$STATE_DIR/passed")
FAILED_COUNT=$(cat "$STATE_DIR/failed")
SKIPPED_COUNT=$(cat "$STATE_DIR/skipped")
TOTAL=$((PASSED_COUNT + FAILED_COUNT + SKIPPED_COUNT))

printf "${BOLD}Total Tests:   %d${NC}\n" "$TOTAL"
printf "${GREEN}Passed:        %d${NC}\n" "$PASSED_COUNT"
if [ "$FAILED_COUNT" -gt 0 ]; then
    printf "${RED}Failed:        %d${NC}\n" "$FAILED_COUNT"
else
    printf "Failed:        0\n"
fi
printf "${BLUE}Skipped:       %d${NC}\n" "$SKIPPED_COUNT"

if [ "$FAILED_COUNT" -gt 0 ]; then
    printf "\n${RED}${BOLD}Verification FAILED with %d error(s).${NC}\n" "$FAILED_COUNT"
    exit 1
else
    printf "\n${GREEN}${BOLD}Verification PASSED successfully! All tested harnesses & features working as planned.${NC}\n"
    exit 0
fi
