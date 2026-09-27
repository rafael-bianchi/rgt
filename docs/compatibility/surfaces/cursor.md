# Cursor

**RGT canonical agent**: `cursor`. **RGT registration**: `.cursor/hooks.json` with pre/post Shell matchers. `rgt init` also installs project guidance in `.cursor/rules/rgt.mdc`, with an always-applied Cursor rule and an RGT-owned block. A clearly RGT-owned rule with LF or CRLF frontmatter is repaired to `alwaysApply: true` during normal initialization; unrelated content is preserved and backed up. Ambiguous ownership requires manual repair. **Live status**: unverified; no version/platform observation is recorded.

No stable file-read path is established. RGT's existing parser can inspect command text and Shell stdout, but a supported Cursor tool/result form, completion semantics, completeness, and response protocol have not been confirmed. A path or arbitrary command output is not proof that the client consumed exact source bytes. Do not infer automatic value capture from a synthetic payload.

RGT's current no-op is exit 0 with empty stdout. Whether this response is accepted by the matching Cursor version is unverified. Exclude pre-events, absent/filtered output, and shell results that do not byte-match the file snapshot.

**Vendor sources**: [Cursor hooks documentation](https://docs.cursor.com/agent/hooks), [Cursor rules](https://cursor.com/docs/rules). **Comparative source**: [`evidence.md`](../../../specs/044-verify-harness-compatibility/evidence.md#cursor).
