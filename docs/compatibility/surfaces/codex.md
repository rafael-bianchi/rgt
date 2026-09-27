# Codex CLI

**RGT canonical agent**: `codex`. **Registration**: intended native `PostToolUse` hook in `.codex/hooks.json`, plus `AGENTS.md` guidance. Codex requires project hook trust review; a file on disk does not show it was loaded. Live version/platform and invocation are unverified.

## Candidate read path

`codex-post-tool-response` is restricted to a `PostToolUse` file-read result whose complete `tool_response` can be reconstructed and byte-matched to the current file. The exact tool name and result shape are version-dependent and must be observed. Truncated, partial, filtered, absent, non-file, and path-only results are excluded. RGT accepts the generic `tool_input.path`/`file_path` plus `tool_response.content` form only as a candidate adapter; it does not infer success from event name alone.

RGT no-op output, trust acceptance, original result preservation, and error flow have not been verified in a real Codex client. The exact documented hook protocol is [Codex hooks](https://developers.openai.com/codex/hooks); it lists `PostToolUse`, `tool_response`, layered hook configuration, and project trust. No version/platform or graph result is claimed.

**Comparative source**: [`evidence.md`](../../../specs/044-verify-harness-compatibility/evidence.md#codex).
