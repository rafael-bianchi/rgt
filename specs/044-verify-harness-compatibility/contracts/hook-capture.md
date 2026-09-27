# Contract: Passive Hook Capture

**CLI surface**: `rgt hook <pre|post> --agent <canonical-name-or-alias>`, invoked by a native client hook or thin plugin. Input is one native event from stdin. The CLI and plugin must leave the host command, result, approval, and error unchanged.

## Eligibility

| Input condition | Capture result | Host result |
|---|---|---|
| Before-tool event, even with a path or proposed content | No value nodes | Allow the host action to proceed. |
| Post-tool failure or cancellation | No value nodes | Preserve the host outcome. |
| Post-tool event with unknown outcome and no completed result | No value nodes | Allow; report the read path as unverified in support evidence. |
| Explicit success with only a path, or a partial/filtered/unmappable result | No source-backed value nodes | Preserve the original result; disclose that read path's capture limit. |
| Confirmed successful completed read whose full source bytes and line positions can be reconstructed and matched to the current file | Extract and record numeric/date values, then attach the canonical capture agent | Preserve the original result. |
| Successful read with no interpretable value | No value nodes | Return a host-valid no-op. |
| Empty, malformed, BOM-prefixed, unexpected, oversized, or timed-out event | No spurious value nodes; BOM may be stripped for parsing | Fail open with a host-valid response. |

Success may be reported explicitly by the native event or shown by a completed read result. A success flag proves completion, but not which bytes were consumed. Phase alone, command text alone, and file existence are insufficient. A path-only event must not trigger a disk reread that RGT presents as content the client consumed. For source-backed roots, a documented adapter must reconstruct the full source bytes and original line positions from the completed result; the current file snapshot must match those bytes before its fingerprint is stored. A changed snapshot or partial/filtered result fails open without roots. Codex `PostToolUse` may provide a result-bearing `tool_response`; the adapter must reject truncated or non-file responses. Windsurf `post_read_code` is successful-read evidence but its documented path-only payload is insufficient for value nodes. Both hooks remain passive. When a client cannot deliver a qualifying read event and content, its read capture is instruction-only if it can load guidance.

## Host response rule

The wrapper must emit the exact neutral response required by the observed client contract on success, no-op, timeout, and internal failure, including when the RGT executable is missing or moved. Do not assume empty stdout is valid for all clients. A host requiring JSON receives valid JSON on every path; one requiring no output receives no output. The response must not contain a command rewrite, replacement tool result, approval decision, or error suppression. A command-hook surface needs a host-compatible fallback launcher or a documented and tested host fail-open outcome before it can claim automatic support. The client-specific response form is accepted only after vendor documentation or a no-cost native event check establishes it for the named read path/version/platform. Input is capped at 8 MiB and its read has an elapsed deadline even when a slow stream never reaches EOF. Plugin children and direct command-hook processes have an enforced 800 ms invocation deadline or a documented, tested host timeout/fail-open outcome; timeout stops value processing and leaves time for a host-valid response within one second. Measure invocation-to-neutral-response time for 64 KiB text, 1 MiB binary-envelope, oversized, no-EOF, and stalled-child cases on direct hooks and plugins; failure paths must fail open within one second without values.

## Normalization boundary

The Rust adapter supplies `surface_id`, canonical `agent_name`, `phase`, `read_target`, `outcome_evidence`, and `completed_content` to the common capture path. The thin plugin may forward native fields and frame a required neutral host response. It may not decide whether content is trustworthy, parse values, write the graph, or infer success from a before event.

The existing PDF envelope path uses the same eligibility gate before decoding or storing a value. Existing Claude Code `additionalContext` behavior must still respect its documented host response and must not substitute for evidence that a read succeeded.

## Idempotency and attribution

One successful action delivered twice, or as a before/after pair, yields at most one root node per existing root identity and one `(node_id, agent_name)` association for the same canonical agent. Graph attribution names that canonical agent; Copilot Chat and CLI are distinguished by compatibility evidence, not separate graph agents. Capture attribution does not claim source authorship or derivation calculation. Distinct canonical agents may each have an association to a shared root node.

## Verification conditions

For every read path claimed verified automatic value capture, retain a redacted native successful-read observation and a no-op/error observation for the same read-path ID, version, and platform, plus graph value records containing node IDs, number/date values, source lines, and canonical agent attribution. The ID resolves to a documented tool and accepted completed-result form; other paths and excluded forms inherit no claim. Confirm registration loading, full-snapshot content evidence, neutral response acceptance, and unchanged tool result. A per-read-path synthetic matrix checks successful, empty, malformed, repeated, and no-op/error events; per-client installer checks cover obsolete registrations. Test BOM, missing-executable, slow no-EOF input, and direct-command/plugin timeout behavior through deterministic local replay. A different version/platform starts unverified. The evidence format is [compatibility-evidence.schema.json](compatibility-evidence.schema.json).
