# Quickstart: Derivation Verification

**Feature**: `011-derivation-verification` | **Date**: 2026-08-08

## Prerequisites

- Rust toolchain, repository cloned, `cargo build`
- `.rgt/store.db` initialized (`cargo run -- init`)
- Two number nodes recorded (use `rgt status` to get their IDs)

## Validation Scenarios

### Scenario 1: Expression verification passes

```bash
# Record two number nodes first, then verify
rgt verify --parents <node_a>,<node_b> --operation EXPRESSION --expression "a + b" --result 300
echo $?   # should be 0
```

### Scenario 2: Expression verification fails

```bash
rgt verify --parents <node_a>,<node_b> --operation EXPRESSION --expression "a + b" --result 500
echo $?   # should be 1
# stderr should show: expected: 300, provided: 500
```

### Scenario 3: Date diff verification

```bash
# Record two Date nodes (2026-01-01 and 2026-01-11), then verify
rgt verify --parents <d1>,<d2> --operation DATE_DIFF --result 864000
echo $?   # should be 0
```

### Scenario 4: Unknown operation passes through

```bash
rgt verify --parents <n1> --operation CUSTOM --result 42
echo $?   # should be 0
# stderr should warn: operation 'CUSTOM' is not verifiable
```

### Scenario 5: Invalid parent node

```bash
rgt verify --parents nonexistent --operation EXPRESSION --expression "a" --result 0
echo $?   # should be 2
# stderr: parent node 'nonexistent' not found
```

### Scenario 6: Run test suite

```bash
cargo test --all -- --test-threads=1
```

**Expected**: All tests pass. Verification unit tests, CLI contract tests, and existing tests all green.
