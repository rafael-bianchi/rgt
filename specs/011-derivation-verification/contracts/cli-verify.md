# `rgt verify` CLI Contract

**Feature**: `011-derivation-verification` | **Date**: 2026-08-08

## Usage

```
rgt verify --parents <ids> --operation <op> [--expression <expr>] --result <value>
```

## Arguments

| Argument | Required | Description |
|---|---|---|
| `--parents` | Yes | Comma-separated node IDs (order matters: [0]→a, [1]→b, ...) |
| `--operation` | Yes | `EXPRESSION` or `DATE_DIFF` |
| `--expression` | For EXPRESSION | Formula string (e.g., `"(a + b) * c / 100"`) |
| `--result` | Yes | Expected numeric result |

## Exit Codes

| Code | Meaning | Hook Action |
|---|---|---|
| 0 | Verification passed | Allow, no rewrite needed |
| 0 | Unknown operation (warning on stderr) | Allow, store without verification |
| 1 | Mismatch (expected vs provided on stderr) | Auto-correct using `hint` from stderr |
| 2 | Invalid input (missing parents, bad expression) | Deny, let LLM retry |

## Examples

### Expression — match
```
$ rgt verify --parents node_a,node_b --operation EXPRESSION --expression "a + b" --result 300
[exit 0, no output]
```

### Expression — mismatch
```
$ rgt verify --parents node_a,node_b --operation EXPRESSION --expression "a + b" --result 500
Error: verification failed for EXPRESSION "a + b"
  expected: 300
  provided: 500
  hint: retry with --result 300
[exit 1]
```

### Date diff — match
```
$ rgt verify --parents d1,d2 --operation DATE_DIFF --result 864000
[exit 0]
```

### Date diff — mismatch
```
$ rgt verify --parents d1,d2 --operation DATE_DIFF --result 864001
Error: verification failed for DATE_DIFF
  expected: 864000
  provided: 864001
  hint: retry with --result 864000
[exit 1]
```

### Unknown operation
```
$ rgt verify --parents n1 --operation CUSTOM --result 42
Warning: operation 'CUSTOM' is not verifiable — skipping verification
[exit 0]
```

### Invalid input
```
$ rgt verify --parents nonexistent --operation EXPRESSION --expression "a" --result 0
Error: parent node 'nonexistent' not found in database
[exit 2]
```
