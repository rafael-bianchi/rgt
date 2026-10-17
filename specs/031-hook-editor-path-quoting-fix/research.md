# Research: Hook Editor Path Quoting Fix

**Feature**: [spec.md](./spec.md) | **Date**: 2026-08-31

Decisions grounded in `src/hooks/editor.rs` (`is_rgt_command` at :400-416), `src/hooks/installer.rs` (`shell_quote` at :231-233, its call sites at :405/:459/:520/:549/:563), and the existing tests `tests/unit/hook_editors.rs` / `tests/contract/hook_installer.rs`.

## 1. Root cause (FR-001)

**Finding**: `shell_quote` wraps the resolved path in literal double quotes: `format!("\"{}\"", path.display())`. The command written is `"/usr/local/bin/rgt" hook post`. `is_rgt_command` reads it with `cmd.split_whitespace()`: for `"/Applications/My App/rgt" hook post`, the tokens are `["\"/Applications/My", "App/rgt\"", "hook", "post"]` — the first token is the truncated `"/Applications/My` (only its leading `"` is trimmed by `trim_matches('"')`), and the second token `App/rgt"` is mistaken for the subcommand, so the predicate returns false. `element_has_rgt_command` (`editor.rs:373`) then treats the existing entry as foreign and the merge path (`editor.rs:274` filter) may append a duplicate on re-init. Verified by code reasoning; the verification report flagged this as medium-confidence, never empirically reproduced — hence the test requirement.

## 2. Quote-aware tokenizer (FR-001, FR-002, FR-003)

**Decision**: Replace the `split_whitespace()` head of `is_rgt_command` with a small helper that extracts the executable token honoring one leading `"` (the exact form `shell_quote` emits). Implementation:

```rust
/// First shell token of `cmd`, honoring a single leading `"` pair (the form
/// `shell_quote` emits). Returns the token and the remainder after it.
fn split_first_token(cmd: &str) -> (String, &str) {
    let cmd = cmd.trim_start();
    if let Some(rest) = cmd.strip_prefix('"') {
        // Close at the next unescaped `"`; the remainder starts after it.
        let mut chars = rest.char_indices();
        let mut end = rest.len();
        let mut escaped = false;
        for (i, c) in chars {
            if escaped { escaped = false; continue; }
            if c == '\\' { escaped = true; continue; }
            if c == '"' { end = i; break; }
        }
        let token = rest[..end].to_string();
        let after = &rest[end + 1..];
        (token, after)
    } else {
        let end = cmd.find(char::is_whitespace).unwrap_or(cmd.len());
        (cmd[..end].to_string(), &cmd[end..])
    }
}
```

Then `is_rgt_command` becomes:

```rust
fn is_rgt_command(cmd: &str) -> bool {
    let (exe, rest) = split_first_token(cmd);
    let mut rest_tokens = rest.split_whitespace();
    if rest_tokens.next() != Some("hook") { return false; }
    if !matches!(rest_tokens.next(), Some("pre") | Some("post")) { return false; }
    // (trailing --agent tokens are not inspected)
    let base = exe.rsplit(['/', '\\']).next().unwrap_or(&exe);
    let is_bare_rgt = base == "rgt" || base == "rgt.exe";
    let is_absolute = exe.starts_with('/') || exe.starts_with('\\')
        || exe.as_bytes().get(1) == Some(&b':');
    is_bare_rgt || is_absolute
}
```

**Rationale**: The predicate only needs the *first* token (the executable); the remaining tokens (`hook pre|post [--agent X]`) are already space-separable since only the path is quoted. This is a ~15-line dependency-free state machine (Constitution XI). The unquoted branch preserves the exact current behavior for bare/unquoted forms.

**Alternatives considered**: A full POSIX shell tokenizer / `shlex` crate — rejected: overkill and a new dependency. Matching on the unquoted resolved path threaded from the installer (plan option b) — rejected: couples the editor to installer state and is not future-proof. Simply `trim_matches('"')` on the whole string — insufficient: `split_whitespace` still splits inside the quoted path.

## 3. Windows and `--agent` forms (FR-002)

**Finding**: Windows path `C:\Program Files\rgt.exe hook post` — after quoting becomes `"C:\Program Files\rgt.exe" hook post`; the tokenizer's quoted branch extracts `C:\Program Files\rgt.exe`, and `is_absolute` recognizes the drive letter via `exe.as_bytes().get(1) == Some(&b':')`. The `--agent` suffix (`"/..." hook post --agent claude-code`) is naturally handled because the predicate only reads the first three tokens.

## 4. Test strategy (SC-001, SC-002, SC-003)

**Decision**:
- Unit tests in `tests/unit/hook_editors.rs` (extend the existing file) calling `is_rgt_command` directly for: spaced quoted path (true), spaced nested `hooks[]` form via `element_has_rgt_command` (true), spaced non-RGT `"/usr/bin/rtk" hook post` (false), bare `rgt hook post` (true), unquoted `/usr/local/bin/rgt hook post` (true), Windows `C:\Program Files\rgt.exe hook post` (true), `--agent` variant (true), escaped-quote pathological input (no panic, conservative false or true without crashing).
- Contract test in `tests/contract/hook_installer.rs`: run `detect_and_configure_hooks_with_config_and_exe` twice with `injected_exe()` set to a spaced path (e.g. `/Applications/My App/rgt`), asserting the second run configures (idempotent update) and the settings.json contains exactly one `command` entry for that path — no duplicate.

**Rationale**: Constitution IX — this exact case was previously reasoned-only and must now be empirically verified. The contract test reuses the existing `set_cwd`/`injected_exe` harness with a spaced path.
