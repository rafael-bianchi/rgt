# Windsurf Cascade

**RGT canonical agent**: `windsurf`. RGT is intended to register a native hook and install guidance. Hook loading remains unverified. Windsurf hooks do not load in Restricted Mode.

## Read path and limits

`windsurf-post-read-code-path-only` is the documented `post_read_code` event. Its payload provides `file_path` (which can be a directory); it does not provide completed source bytes. It can support invocation evidence only and must not create value nodes. Automatic value capture for this path is instruction-only.

Windsurf documentation describes system-level paths, a user-level `~/.codeium/windsurf/hooks.json` path, and workspace `.devin/hooks.json`. Workspace `.windsurf/hooks.json` is legacy and is used only when `.devin/hooks.json` is absent or defines no hooks. System hooks take precedence over user and workspace hooks. RGT must inspect precedence and merge the active artifact rather than create a shadowed legacy file. The required neutral response and live event/result preservation have not been natively observed.

**Vendor source**: [Cascade hooks](https://docs.windsurf.com/windsurf/cascade/hooks), checked 2026-09-26. **Comparative source**: [`evidence.md`](../../../specs/044-verify-harness-compatibility/evidence.md#windsurf).
