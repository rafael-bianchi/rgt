# CLI Contract: `rgt record`

**Feature**: 001-cli-record-derive
**Contract type**: CLI subcommand

## Synopsis

```
rgt record <FILE> [--stdin]
```

## Arguments

| Argument | Required | Type | Description |
|---|---|---|---|
| `<FILE>` | Yes | Path (string) | Path to the data file to record. Used for node ID generation and source document tracking. |
| `--stdin` | No | Flag | Read file content from stdin instead of disk. `<FILE>` is still required for node ID determinism and must exist on disk for metadata. |

## Behavior

1. If `--stdin`: read content from stdin. Otherwise: read content from `<FILE>` on disk.
2. If `<FILE>` does not exist on disk: exit 2 with `Error: file not found: <FILE>`.
3. Extract numeric and ISO-8601 date values from content using the extraction engine (`hooks::parser::extract_values_from_content`).
4. Enforce soft limit of 10,000 values; print warning to stderr if exceeded.
5. Upsert source document metadata (`upsert_source_document`).
6. For each extracted value: generate root node ID, upsert tracked node (`insert_tracked_node`).
7. Print summary to stdout: `Recorded <N> values from <FILE>`.

## Exit Codes

| Code | Meaning | When |
|---|---|---|
| 0 | Success | Values recorded (including 0 values for empty files) |
| 1 | N/A | `rgt record` does not use exit code 1 |
| 2 | Invalid input | File not found, unreadable file, or metadata error |

## Output

**Stdout (success)**:
```
Recorded <N> values from <FILE>
```
Where `<N>` is the count of values actually recorded (not exceeding 10,000).

**Stdout (empty file)**:
```
Recorded 0 values from <FILE>
```

**Stderr (limit exceeded)**:
```
Warning: value limit (10000) reached, excess values skipped for <FILE>
```

**Stderr (error)**:
```
Error: file not found: <FILE>
```

**Stdout**: No output on error.

## Idempotency

Re-recording the same file with unchanged content:
- Source document metadata is refreshed (mtime, size, hash updated).
- Existing root nodes with unchanged values are re-upserted as fresh (`is_stale = 0`).
- No duplicate nodes are created (UPSERT by node ID).
- If any values changed: new nodes created for changed values; old nodes remain (will be marked stale by `rgt status`).

## Examples

```bash
# Record values from a CSV file
rgt record budget.csv
# Output: Recorded 5 values from budget.csv

# Record from stdin (e.g., agent has content from tool call)
cat budget.csv | rgt record --stdin budget.csv
# Output: Recorded 5 values from budget.csv

# Empty file
rgt record empty.txt
# Output: Recorded 0 values from empty.txt

# File not found
rgt record nonexistent.csv
# Exit 2: Error: file not found: nonexistent.csv
```
