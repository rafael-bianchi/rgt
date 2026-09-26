# CLI Contract: Exact Duration Operations and Unit Display

**Feature**: [../spec.md](../spec.md) | **Date**: 2026-09-25

## Commands and flags

| Command / operation | Parents | `--result` | `--result-unit` | `--unit` | Recorded kind |
|---|---|---|---|---|---|
| `derive --operation DATE_DIFF` | Exactly two ordered Dates | Optional assertion | Optional only with result; default seconds | Optional display | Duration |
| `derive --operation DURATION_SUM` | 2–26 distinct Durations | Optional assertion | Optional only with result; default seconds | Optional display | Duration |
| `derive --operation DURATION_AVG` | 2–26 distinct Durations | Optional assertion | Optional only with result; default seconds | Optional display | Duration |
| `verify` with any temporal operation | Same parent rules | Required assertion | Optional; default seconds | Not accepted | No write |
| `derive/verify --operation EXPRESSION` | Existing rules | Required, existing numeric semantics | Not accepted | Not accepted | Number for derive |
| `query <duration-id>` | Requested node must be Duration when `--unit` is used | Not accepted | Not accepted | Optional display | No write |

Accepted unit names are the exact lowercase strings `seconds`, `minutes`, `hours`, `days`, and `weeks`. `months`, `years`, abbreviations, and case variants are invalid. Conversion scales are 1, 60, 3,600, 86,400, and 604,800 seconds respectively. Unit flags on unrelated commands/operations are invalid input.

### Automatic derivation

```text
rgt derive --parents <date1>,<date2> --operation DATE_DIFF --unit days
Derived node: node_drv_<id>
Duration: 45 days (3888000 seconds)
```

When `--unit` is absent, an otherwise valid existing `DATE_DIFF` invocation retains its single `Derived node: ...` stdout line. New typed operations without `--unit` also print the node ID. `--result-unit` never controls display.

```text
rgt derive --parents <date1>,<date2> --operation DATE_DIFF \
  --result 45 --result-unit days --unit minutes
Derived node: node_drv_<same-id>
Duration: 64800 minutes (3888000 seconds)
```

`verify` remains silent on success and records nothing. An asserted result on `derive` is checked before any node/edge write. A valid but incorrect claim produces a nonzero verification error naming its selected claim unit and exact expected/supplied seconds.

### Exact claim syntax

Temporal `--result` accepts an optional sign, decimal digits with an optional point, and an optional `e`/`E` exponent; for example `45`, `+45.0`, `.1`, `1.5`, and `4.5e1`. Parse the original token as a decimal quantity, multiply by the selected unit scale exactly, and accept it only if it yields a representable whole number of seconds. Preserve the existing `EXPRESSION` numeric semantics. Both `--result=-1` and `--result -1` should be accepted as argument forms so sign validation occurs in RGT rather than being mistaken for a flag.

Examples: `0.1 --result-unit minutes` is 6 seconds; `0.01 --result-unit minutes` requires 0.6 seconds and is invalid; `1.0000000000000001 --result-unit seconds` is distinct from one second and is invalid. `NaN`, infinity, malformed tokens, out-of-range values, and unreasonable token/exponent lengths are invalid. The implementation must bound parsing work without rounding the input through f64.

### Selected-unit display

Display uses the exact signed seconds. A finite decimal conversion is shown exactly; a repeating decimal is deterministically rounded for display, marked approximate, and accompanied by exact seconds. For example, one second shown in minutes is approximately `0.016666666667 minutes (1 second)`. Rounding affects only presentation.

For `query <duration-id> --unit days`, preserve the existing query object and its existing `value` string. Add only to the requested node's lineage step:

```json
{
  "node_id": "node_drv_<id>",
  "value_kind": "DURATION",
  "value": "45d",
  "duration_seconds": "3888000",
  "selected_unit_display": {
    "unit": "days",
    "decimal": "45",
    "approximate": false,
    "text": "45 days"
  }
}
```

The snippet shows the additional fields in the requested `lineage_steps` entry, not a replacement for the whole response. `duration_seconds` is a decimal string so large signed i64 values remain exact for JSON readers. Other lineage steps keep their current fields and display. The default query still wraps the result under `lineage`; `query --json` still returns the result directly. With no `--unit`, neither response gains fields or changes its current shape. `query --unit` for Number/Date fails rather than assigning temporal meaning.

### Errors and compatibility

| Situation | Exit | Graph effect |
|---|---:|---|
| Successful derive, verify, or query | 0 | Derive inserts/reuses only a correct typed result; verify/query write nothing. |
| Well-formed but wrong asserted result | 1 | No node or edge written. |
| Reversed Date parents, corrupt temporal parent, type mismatch, fractional-second computed result, or arithmetic overflow | 1 | No node or edge written. |
| Unknown unit/operation, inapplicable flag, malformed or unrepresentable claim, too few/many or repeated parents | 2 | No node or edge written. |

Existing seconds-based `DATE_DIFF --result <seconds>` and successful `verify` retain their meaning. Existing `EXPRESSION` with Date or Duration parents remains available and produces a Number, but emits a stderr warning: Date parents are Unix timestamp seconds, Duration parents are elapsed seconds, and typed duration operations should be used for sum/average. The warning does not change the success exit code or stdout result.
