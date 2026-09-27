# Pi

**RGT canonical agent**: `pi`. RGT writes a TypeScript extension artifact and recording guidance. Project initialization adds an RGT block to `.pi/APPEND_SYSTEM.md`, which Pi loads for a trusted project. Global initialization adds it to `~/.pi/agent/AGENTS.md`, or the existing `AGENTS.override.md` when that file shadows it. RGT preserves other instructions and reports ambiguous RGT blocks instead of overwriting them. Reload Pi after changing instructions. `rgt doctor` checks the local instruction block separately from extension registration; neither file proves Pi loaded it.

The current named extension object and exported `onToolCall` function have not been loaded in a real Pi client. Discovery, callback phase, tool/result fields, accepted no-op response, version/platform, and graph effect remain unknown. No stable read-path ID is established and no automatic value-capture claim is made. Until completed full-content read evidence exists, follow the loaded instructions to run `rgt record <file>` after a successful read.

**Vendor source**: [Pi configuration documentation](https://github.com/earendil-works/pi/blob/main/packages/coding-agent/docs/configuration.md). **Comparative source**: [`evidence.md`](../../../specs/044-verify-harness-compatibility/evidence.md#pi).
