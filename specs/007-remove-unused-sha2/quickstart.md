# Quickstart: Remove Unused sha2 Dependency

## Validation Scenarios

This feature is an investigation confirming whether the `sha2` dependency can be safely removed. No code changes are applied.

### Scenario 1: Confirm sha2 usage in source

**Prerequisites**: Repository cloned at current working directory.

```bash
grep -rn "sha2\|Sha256\|Sha512" src/
```

**Expected**: Returns at least one match. Actual result confirms usage in `src/updater/checksum.rs`.

### Scenario 2: Verify build succeeds with sha2 present

```bash
cargo build --release
```

**Expected**: Compilation succeeds with sha2 as a dependency.

### Scenario 3: Verify tests pass

```bash
cargo test --all
```

**Expected**: All tests pass, including `src/updater/checksum.rs` unit tests.

### Scenario 4: Verify updater module imports checksum

```bash
grep -rn "checksum" src/updater/
```

**Expected**: `src/updater/github.rs` references `checksum::parse_checksums`, `checksum::find_checksum`, and `checksum::verify_checksum`.

## Closure Recommendation

Update GitHub issue #4 with investigation findings and close as "not a bug" — sha2 is an active dependency required for release artifact checksum verification.
