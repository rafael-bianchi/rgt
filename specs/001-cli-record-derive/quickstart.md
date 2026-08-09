# Quickstart Validation: CLI Record & Derive Commands

**Feature**: 001-cli-record-derive

This guide documents runnable validation scenarios that prove the feature works end-to-end. Run these after implementation to verify correctness.

## Prerequisites

- RGT installed from the feature branch (`cargo install --git https://github.com/rafael-bianchi/rgt --branch feat/014-cli-record-derive`)
- Shell with `sqlite3` available for DB inspection
- Temporary directory for test fixtures

## Scenarios

### Scenario 1: Record Root Values

**Purpose**: Verify `rgt record` extracts numbers and dates from a file and creates root nodes.

**Setup**:
```bash
mkdir -p /tmp/rgt-record-test && cd /tmp/rgt-record-test
rm -rf .rgt
rgt init --agent codex

cat > data.csv << 'EOF'
category,amount
revenue,120000
costs,60000
profit,60000
tax_rate,0.21
EOF

cat > schedule.txt << 'EOF'
milestone: 2026-01-15
deadline: 2026-06-30
EOF
```

**Steps**:
```bash
# 1. Record budget data
rgt record data.csv
# Expected stdout: Recorded 5 values from data.csv
# Expected exit: 0

# 2. Record schedule data
rgt record schedule.txt
# Expected stdout: Recorded 2 values from schedule.txt
# Expected exit: 0

# 3. Check graph state
rgt status
# Expected: Total Nodes >= 7, Active Nodes = Total, Stale Nodes = 0

# 4. Verify date node exists
sqlite3 .rgt/store.db "SELECT count(*) FROM tracked_nodes WHERE value_kind='DATE'"
# Expected: 2 (two dates from schedule.txt)
```

**Teardown**: `rm -rf /tmp/rgt-record-test`

---

### Scenario 2: Derive and Verify Expression

**Purpose**: Verify `rgt derive` records a correct derivation and rejects an incorrect one.

**Setup**: Run Scenario 1 first, or recreate fixtures in a new temp dir.

**Steps**:
```bash
# 1. Get node IDs for revenue (120000) and costs (60000)
REV=$(sqlite3 .rgt/store.db "SELECT id FROM tracked_nodes WHERE number_val=120000 AND line_number=2")
COSTS=$(sqlite3 .rgt/store.db "SELECT id FROM tracked_nodes WHERE number_val=60000 AND line_number=3")
echo "Revenue: $REV"
echo "Costs: $COSTS"

# 2. Derive profit = revenue - costs (should pass)
rgt derive --parents "$REV","$COSTS" --operation EXPRESSION --expression "a - b" --result 60000
# Expected stdout: Derived node: node_drv_<hex>
# Expected exit: 0

# 3. Attempt wrong derivation (should fail)
rgt derive --parents "$REV","$COSTS" --operation EXPRESSION --expression "a - b" --result 99999
# Expected stderr: Error: verification failed...
# Expected exit: 1

# 4. Verify derived node exists in graph
rgt status
# Expected: Total Nodes = 8 (7 roots + 1 derived), Active Nodes = 8

# 5. Check derivation edge exists
sqlite3 .rgt/store.db "SELECT count(*) FROM derivation_edges"
# Expected: 2 (one edge per parent)
```

---

### Scenario 3: Date Diff Derivation

**Purpose**: Verify DATE_DIFF derivation and rejection of invalid inputs.

**Steps**:
```bash
# 1. Get date node IDs
D1=$(sqlite3 .rgt/store.db "SELECT id FROM tracked_nodes WHERE date_val='2026-01-15T00:00:00+00:00'")
D2=$(sqlite3 .rgt/store.db "SELECT id FROM tracked_nodes WHERE date_val='2026-06-30T00:00:00+00:00'")

# 2. Derive date difference (166 days = 14,342,400 seconds)
rgt derive --parents "$D1","$D2" --operation DATE_DIFF --result 14342400
# Expected: exit 0, derived node ID printed

# 3. Wrong date diff (should fail)
rgt derive --parents "$D1","$D2" --operation DATE_DIFF --result 1
# Expected: exit 1

# 4. Single parent (invalid input)
rgt derive --parents "$D1" --operation DATE_DIFF --result 100
# Expected: exit 2, error about requiring exactly 2 parents
```

---

### Scenario 4: Staleness Cascade with Derived Nodes

**Purpose**: Verify that modifying a source file marks both root nodes and their derived dependents as stale.

**Steps**:
```bash
# 1. Record original data and derive profit
rgt record data.csv
rgt derive --parents ... --operation EXPRESSION --expression "a - b" --result 60000

# 2. Check all fresh
rgt status --stale-only
# Expected: Total Nodes > 0, Stale Nodes = 0

# 3. Modify source file
sed -i '' 's/revenue,120000/revenue,150000/' data.csv

# 4. Check staleness
rgt status
# Expected: All nodes from data.csv marked stale with reason SOURCE_FILE_MODIFIED
# Expected: Derived profit node also marked stale (cascaded)

# 5. Re-record updated file
rgt record data.csv
# Expected: New root node for 150000 created; derived profit node still stale

# 6. Re-derive with updated values
NEW_REV=$(sqlite3 .rgt/store.db "SELECT id FROM tracked_nodes WHERE number_val=150000")
rgt derive --parents "$NEW_REV","$COSTS" --operation EXPRESSION --expression "a - b" --result 90000
# Expected: exit 0, new derived node for profit=90000

# 7. Check old derived node still stale, new one active
rgt status
# Expected: Two derived nodes — one stale (old profit), one active (new profit)
```

---

### Scenario 5: Value Limit Enforcement

**Purpose**: Verify the 10,000 value soft limit.

**Steps**:
```bash
# 1. Generate a file with 10,500 numbers
python3 -c "
for i in range(10500):
    print(i)
" > large.txt

# 2. Record it
rgt record large.txt 2>stderr.log
# Expected stdout: Recorded 10000 values from large.txt
# Expected stderr.log: Warning: value limit (10000) reached, excess values skipped for large.txt
# Expected exit: 0

# 3. Verify exactly 10,000 nodes created
sqlite3 .rgt/store.db "SELECT count(*) FROM tracked_nodes WHERE source_doc_id=(SELECT id FROM source_documents WHERE file_path LIKE '%large.txt')"
# Expected: 10000 (within rounding)
```

---

### Scenario 6: AGENTS.md Template Update

**Purpose**: Verify `rgt init --agent codex` includes record/derive instructions.

**Steps**:
```bash
mkdir -p /tmp/rgt-agent-test && cd /tmp/rgt-agent-test
rgt init --agent codex --force

# Check AGENTS.md contains new instructions
grep "rgt record" AGENTS.md
# Expected: line referencing rgt record

grep "rgt derive" AGENTS.md
# Expected: line referencing rgt derive

rm -rf /tmp/rgt-agent-test
```

---

## Success Criteria Mapping

| SC | Verified By |
|---|---|
| SC-001 (<1s record) | Scenario 1 — manual timing or benchmark |
| SC-002 (100% correct reject) | Scenario 2 step 3, Scenario 3 steps 3-4 |
| SC-003 (reuse verification) | Scenario 2 step 2 (same expression engine as `rgt verify`) |
| SC-004 (full agent workflow) | Scenarios 1-4 together (no hook interception needed) |
| SC-005 (existing tests pass) | Run `cargo test` before merge |
| SC-006 (exit codes 0/1/2) | All scenarios verify exit codes |
