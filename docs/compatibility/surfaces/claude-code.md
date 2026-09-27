# Claude Code

**RGT canonical agent**: `claude-code` (`claude` alias). **RGT registration**: command hook in `.claude/settings.json`, with `PreToolUse` and `PostToolUse` entries. **Live status**: unverified; no client version/platform or loaded-registration observation is recorded.

## Candidate read path

`claude-read-post-content` describes a candidate `Read` tool `PostToolUse` result carrying complete `tool_response.content`. RGT's adapter accepts `Read` with `tool_input.path` or `tool_input.file_path`. `claude-bash-read-stdout` describes the candidate `Bash` path: only a recognized read command plus completed stdout is eligible. Other Bash commands are excluded. In both paths, a full snapshot must byte-match the current file before it can create nodes. These remain support candidates until a native event verifies tool names, fields, completeness, and the neutral response. Missing content, filtered/truncated output, writes/edits, and all pre-events are excluded.

RGT currently emits no stdout for no-op/ordinary events; its PDF path can emit additive `additionalContext`. The required host response and unchanged-result behavior have not been natively observed here. Exact version, platform, successful read, no-op/error, and source-line observations: **unknown**.

**Vendor source**: [Claude Code hooks](https://docs.anthropic.com/en/docs/claude-code/hooks). **Comparative source**: [`evidence.md`](../../../specs/044-verify-harness-compatibility/evidence.md#claude-code).
