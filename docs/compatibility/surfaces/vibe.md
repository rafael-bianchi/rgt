# Mistral Vibe

**RGT canonical agent**: `vibe`. RGT writes a `pre_tool` TOML entry and recording guidance in project `AGENTS.md` or, for `rgt init -g`, `~/.vibe/AGENTS.md`. Vibe documents these as loaded instructions, subject to project trust. Reload or start a new Vibe session after initialization. The old `~/.vibe/prompts/rgt.md` path is a custom replacement system prompt that loads only when `system_prompt_id` selects it; simply installing that file does not activate it. RGT no longer writes it. `rgt doctor` checks the loaded instruction path separately from hook registration, but cannot prove actual loading.

The pre-tool event cannot establish successful completion or consumed content. The current registration schema, version/platform, and live loading are unverified. No stable successful-read path is established and no automatic value-capture claim is made. The read tool/result fields and host no-op response are unknown. Follow the loaded instructions to run `rgt record <file>` after a successful read until a supported completion event with full content is established.

**Vendor source**: [Mistral Vibe README](https://github.com/mistralai/mistral-vibe/blob/main/README.md). **Comparative source**: [`evidence.md`](../../../specs/044-verify-harness-compatibility/evidence.md#mistral-vibe).
