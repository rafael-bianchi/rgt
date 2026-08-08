# CLI Contract: `rgt derive`

**Feature**: 001-cli-record-derive
**Contract type**: CLI subcommand

## Synopsis

```
rgt derive --parents <IDS> --operation <OP> [--expression <EXPR>] --result <VAL>
```

## Arguments

| Argument | Required | Type | Description |
|---|---|---|---|
| `--parents <IDS>` | Yes | Comma-separated list of node IDs | Parent node IDs from `tracked_nodes`. Order determines variable binding: parent[0]=a, parent[1]=b, parent[2]=c, ... |
| `--operation <OP>` | Yes | String | `EXPRESSION` or `DATE_DIFF` |
| `--expression <EXPR>` | Conditional | String (formula) | Required for EXPRESSION. Supports `+`, `-`, `*`, `/`, `%`, `^`, parentheses, and float literals. |
| `--result <VAL>` | Yes | Float | Claimed result to verify against. |

## Behavior

1. Parse `--parents` into a list of node IDs (comma-separated, whitespace trimmed).
2. Look up each parent node by ID in `tracked_nodes`. If any parent not found: exit 2 with error message naming the missing ID.
3. Call `verify::verify(&parent_values, &operation, expression, result)`:
   - **On success (Ok(()))**: Proceed to step 4.
   - **On failure (Err(msg))**: Exit 1 with descriptive error message. No nodes or edges inserted.
   - **On invalid input in verify**: Exit 2 with error message (e.g., wrong parent count for DATE_DIFF, non-date parent for DATE_DIFF).
4. Generate derived node ID via `TrackedNode::generate_derived_id(parent_ids, operation, &value)`.
5. Create a `TrackedNode` with `node_type = DERIVED`, appropriate `value_kind` (NUMBER for EXPRESSION, DURATION for DATE_DIFF), and `is_stale = 0`.
6. Insert derived node via `insert_tracked_node` (idempotent UPSERT by node ID).
7. For each parent node ID: insert derivation edge via `insert_derivation_edge(parent_id, child_id, operation, expression)` (ON CONFLICT DO NOTHING).
8. Print derived node ID to stdout.

## Exit Codes

| Code | Meaning | When |
|---|---|---|
| 0 | Success | Derivation verified and recorded |
| 1 | Verification failed | Claimed result does not match computed value; no write |
| 2 | Invalid input | Unknown parent ID, wrong parent count, wrong parent type, unknown operation, missing expression |

## Output

**Stdout (success)**:
```
Derived node: <node_id>
```

**Stderr (verification failure, exit 1)**:
```
Error: verification failed for <OP> "<EXPR>"
  expected <computed>, got <claimed>
  hint: retry with the correct result
```

**Stderr (invalid input, exit 2)**:
```
Error: parent node '<id>' not found in database
Error: DATE_DIFF requires exactly 2 Date parents, got <N>
Error: DATE_DIFF parent[0] must be a Date, got <kind>
Error: Missing --expression for EXPRESSION operation
```

## Idempotency

Re-deriving with the same parameters (same parent IDs, operation, expression, result):
- Derived node is re-upserted (UPSERT by deterministic ID) — no duplicate.
- Derivation edges are unchanged (ON CONFLICT DO NOTHING prevents duplicates).
- Exit code is 0 (verification still passes since parent values haven't changed).

## Examples

```bash
# Expression derivation: profit = revenue - costs
rgt derive \
  --parents node_raw_8339a98b07bd,node_raw_c66b083d0622 \
  --operation EXPRESSION \
  --expression "a - b" \
  --result 40000
# Output: Derived node: node_drv_a1b2c3d4e5f6

# Date diff derivation: project duration
rgt derive \
  --parents node_raw_abb5e914900e,node_raw_2c1bc0192988 \
  --operation DATE_DIFF \
  --result 14342400
# Output: Derived node: node_drv_f6e5d4c3b2a1

# Wrong result: rejected
rgt derive \
  --parents node_raw_8339a98b07bd,node_raw_c66b083d0622 \
  --operation EXPRESSION \
  --expression "a - b" \
  --result 99999
# Exit 1: Error: verification failed for EXPRESSION "a - b"
#   expected 40000, got 99999
#   hint: retry with the correct result

# Unknown parent ID
rgt derive \
  --parents nonexistent_node \
  --operation EXPRESSION \
  --expression "a" \
  --result 42
# Exit 2: Error: parent node 'nonexistent_node' not found in database
```
