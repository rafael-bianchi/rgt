# Research: Remove Unused sha2 Dependency

## Investigation: Is sha2 actually unused?

### Decision: sha2 is actively used — no removal

The `sha2` crate is declared as a dependency in `Cargo.toml` (line 32: `sha2 = "0.10"`) and is referenced in six locations within `src/updater/checksum.rs`:

| File | Line(s) | Usage |
|---|---|---|
| `src/updater/checksum.rs` | 1 | `use sha2::{Digest, Sha256};` |
| `src/updater/checksum.rs` | 39 | `pub fn compute_sha256(path: &Path) -> io::Result<String>` |
| `src/updater/checksum.rs` | 41 | `let mut hasher = Sha256::new();` |
| `src/updater/checksum.rs` | 48 | `let actual = compute_sha256(path)?;` in `verify_checksum` |
| `src/updater/checksum.rs` | 97 | `fn test_compute_and_verify_sha256()` unit test |
| `src/updater/checksum.rs` | 102 | `let hash = compute_sha256(path).unwrap();` in test |

The `checksum` module is consumed via `mod updater` in both `src/main.rs:8` and `src/lib.rs:8`, and its functions are called from `src/updater/github.rs:147-151` during release artifact verification.

### Rationale

The `sha2` dependency serves a distinct purpose from `blake3`:

- **blake3**: Used for Tier 2 file-change detection (Constitution Principle II) — fast, optimized for content hashing at scale.
- **sha2 (SHA-256)**: Used for release artifact checksum verification (`rgt update`) — the industry-standard hash expected by GitHub Releases checksum files. SHA-256 is the required format for GitHub's `sha256sum.txt` artifacts.

Removing sha2 would break the `rgt update` command's ability to verify downloaded release binaries, compromising supply-chain security.

### Alternatives Considered

| Alternative | Assessment |
|---|---|
| Replace SHA-256 with BLAKE3 for checksum verification | Rejected. GitHub Releases checksum files use SHA-256 format. RGT would be unable to verify against the standard checksum files published with each release. |
| Use `ring` crate instead of `sha2` | Rejected. `sha2` 0.10.x is well-maintained, widely audited, and already in the dependency tree. Swapping introduces unnecessary churn with zero functional benefit. |
| Remove sha2 and skip checksum verification | Rejected. This would degrade security by removing release binary integrity verification. |

### Conclusion

The issue's premise was incorrect. `sha2` is a required dependency for the `rgt update` self-update mechanism, providing SHA-256 checksum verification that matches the format of GitHub Releases checksum files. No changes are warranted.
