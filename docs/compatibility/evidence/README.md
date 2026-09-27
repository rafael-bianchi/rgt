# Compatibility Evidence Ledger

[`observations.json`](observations.json) contains one schema-conforming record per observation. The schema is [`../evidence.schema.json`](../evidence.schema.json). Record native-client observations separately from fixture replay, vendor documentation, and comparative-source findings.

Each read observation names a stable `read_path_id` resolved in the matching client surface contract. Include `rgt_version` from `rgt --version`, the client version, whether the client loaded the registration, the event phase and tool, completion/content evidence, host response acceptance, unchanged tool result, and actual number/date values with node IDs and source lines. Record `full-snapshot-match` only when reconstructed bytes equal the current file snapshot.

For a given surface, version, platform, and non-`none` read path, automatic value capture requires both a native success and a native no-op/error observation. They must use the same read-path ID. The successful record must show loaded registration, full-snapshot match, accepted neutral response, unchanged host result, and observed values/node IDs. The paired no-op/error record must show no graph values and an accepted neutral response with the result/error unchanged. Different read paths cannot be paired. Fixture, vendor documentation, and RTK comparative records never promote support.

A completed read of a file with no numeric or date values is a no-op observation when it passes through the same installed read path and adds no value nodes.

Redact private paths, source contents, credentials, and session IDs. Keep synthetic fixtures labeled synthetic. `read_path_id: none` is reserved for guidance-only or non-read observations and cannot support automatic capture.
