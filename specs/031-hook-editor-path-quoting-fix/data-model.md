# Data Model: Hook Editor Path Quoting Fix

**Feature**: [spec.md](./spec.md) | **Date**: 2026-08-31

No persisted data changes.

## 1. HookCommand (transient)

The `command` string in a hook config entry:

```
<exe> hook pre|post [--agent <name>]
```

where `<exe>` is either the bare name (`rgt`/`rgt.exe`), an unquoted absolute path, or a shell-quoted absolute path (`"..."`) as written by `shell_quote`.

## 2. Recognition contract

| Command form | Recognized as RGT? |
|--------------|--------------------|
| `"/Applications/My App/rgt" hook post` | YES (was NO — the bug) |
| `"/Applications/My App/rgt" hook post --agent claude-code` | YES |
| `"C:\Program Files\rgt.exe" hook pre` | YES |
| `/usr/local/bin/rgt hook post` | YES (unchanged) |
| `rgt hook post` | YES (unchanged) |
| `"/usr/local/bin/rtk" hook post` | NO (spaced non-RGT) |
| `rtk hook post` | NO (unchanged) |
