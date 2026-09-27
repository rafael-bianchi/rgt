# Contract: Initialization and Local Diagnostics

## `rgt init [-g] [--agent <name>] [--force]`

Normal initialization inspects the named client's active registration location. It adds RGT when absent, replaces a recognized obsolete RGT-owned entry, and leaves a current entry unchanged. It preserves unrelated settings, hook commands, plugins, comments, and guidance. Before modifying an existing artifact, it keeps a backup. A second normal run must add no callback and leave the active entry stable.

| Existing state | Normal-init outcome |
|---|---|
| No active RGT entry | Add one current registration or supported guidance. |
| One current RGT entry | No change; report already current. |
| One recognized obsolete RGT-owned entry | Replace it automatically; report migrated and the backup path. |
| Duplicate clearly RGT-owned entries | Consolidate to one current registration when the complete edit is safe; otherwise report conflict. |
| Mixed RGT and non-RGT commands, unknown owner, malformed configuration, or user-owned artifact | Do not change the affected content; report the exact path, conflict, and recovery action. |

Ownership recognition must inspect the executable/marker and host structure. A foreign executable followed by `hook pre` or `hook post` is not RGT-owned. An RGT command inside a mixed callback group does not grant ownership of the whole group. `--force` remains an explicit user action but must not silently delete unrelated content. Per-agent failures are reported independently so one broken client does not prevent safe configuration of another. Existing exit meanings remain: 0 for completed initialization, 1 for an expected configuration failure, 2 for invalid input such as an unknown agent.

Codex and Windsurf receive native hook registrations during `rgt init -g` and normal initialization. Codex uses a documented `PostToolUse` command hook in the active `.codex/hooks.json` layer; project hooks may remain untrusted until reviewed in the client. Windsurf uses the hook file selected by its version, with `.devin/hooks.json` taking precedence over legacy `.windsurf/hooks.json` in current Cascade documentation, and cannot run hooks in Restricted Mode. The installer must inspect which path the selected host loads and avoid writing a shadowed or duplicate entry. It must not bypass Codex trust or Windsurf workspace restrictions.

An event hook records values only where an observed client contract can deliver a successful completed read with reconstructable full source content and a valid neutral response. The documented Windsurf `post_read_code` event contains `file_path` but no source bytes: install the native hook for the constitution, return a neutral result, and provide loadable instruction guidance for value recording; do not reread the path and claim automatic capture. Codex `PostToolUse` may supply a qualifying `tool_response`, but only a tested full-read adapter may create roots. For command-hook clients, initialization must supply a host-compatible fallback launcher when a missing RGT executable would otherwise break the response, or rely on a documented and tested host fail-open outcome. Never call a generated file “verified” merely because it was written.

## `rgt doctor`

Doctor checks the project store and each detected/selected client surface's **local** RGT registration or guidance. It parses the active artifact and checks for the expected command/entrypoint, event phase, referenced executable, and duplicate or obsolete entries as locally inspectable. For Codex it reports that project-hook trust must be checked in `/hooks`; for Windsurf it reports any locally observable Restricted Mode or shadowed hook path and otherwise states that workspace activation must be checked in the client. A generic settings file, any `AGENTS.md`, or a nonempty graph is insufficient proof of an active hook. Copilot Chat and Copilot CLI receive separate findings.

| Local finding | Doctor meaning |
|---|---|
| Current RGT entry with locally usable command/entrypoint | Registration healthy locally; live client invocation unverified here. |
| Codex entry requiring trust review or Windsurf entry in an inactive/shadowed workspace, where locally detectable | Configuration needs host action or path repair; never “healthy live capture.” |
| Loadable guidance, no automatic read hook | Instruction-only. |
| Missing, obsolete, malformed, duplicate, unresolvable executable, or conflicting entry | Warning or error with path and recovery action; never “healthy hook.” |

Doctor does not run an agent session and does not report historical evidence as a current machine check. Versioned live verification appears in the support matrix and [compatibility evidence](compatibility-evidence.schema.json). Preserve the existing process convention: exit 0 for healthy/advisory findings and 1 for errors. Diagnostic messages must distinguish registration health from capture proof.
