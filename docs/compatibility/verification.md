# Local Harness Compatibility Verification

This file records local checks for the harness compatibility feature. Synthetic fixtures and process-wrapper tests do not establish that a native client loads a hook or provides a successful full-content read event. The later [native OpenCode acceptance section](#native-opencode-read-acceptance-2026-09-26) records one real-client tuple.

## Recheck Commands

Run from the repository root. These commands do not start GitHub workflows, an AI session, or a live updater request. Update-check tests inject release metadata locally and forbid downloads.

```sh
rtk proxy cargo test --test test_harness_compatibility
rtk proxy cargo test --test test_doctor_contract
rtk proxy cargo test --test test_hooks_contract
rtk proxy cargo test --test test_hook_parsers
rtk proxy cargo test --test test_hook_installer_contract
rtk proxy cargo test --test test_hooks_installer
rtk proxy cargo test --test test_cli_version
rtk proxy cargo test --test test_pdf_capture_contract
rtk proxy cargo test --test test_turtle_capture
rtk proxy cargo test --test test_hooks_installer test_global_init_detects_codex_claude_cursor_and_windsurf_together
rtk proxy cargo fmt --all -- --check
rtk proxy cargo clippy --all-targets -- -D warnings
rtk proxy cargo test
```

The default Cargo suite checks installer output and update decisions without a network dependency or latency performance assertion. The offline package smoke check retains a 30-second safety timeout. To inspect a release build separately, run `rtk proxy cargo build --release --offline --locked`; this may require dependencies to be cached locally. The latency runner below is also a separate manual check. Record the platform, binary, observed times, and exit status when using it; a synthetic wrapper result does not establish native-client capture. A live `rgt update --check` contacts GitHub Releases and is an optional manual check, outside the default suite.

Run the manual latency report with a locally built RGT executable:

```sh
rtk proxy cargo build --bin rgt
rtk proxy cargo bench --bench bench_hook_capture -- "$PWD/target/debug/rgt"
```

The runner checks the generated POSIX direct-command wrapper on Unix and the generated PowerShell direct-command wrapper on Windows, along with the generated Pi and OpenCode plugins. Cases include 64 KiB and 1 MiB text success-shaped, no-op, malformed-shape, and repeated callbacks; OpenCode's successful `opencode-read-display-text` path with exact source text; 1 MiB PDF binary-envelope success-shaped, no-op, malformed, and repeated plugin events; direct PDF envelope cases; oversized events; direct-hook stdin held open without EOF; and stalled direct and plugin children. Each plugin case uses an isolated project, checks the callback's `undefined` neutral return and unchanged input, and checks actual node and capture-association counts. The OpenCode cases also check the stored Number 42, source line 1, and `opencode` capture attribution after two callbacks. The Pi adapter currently does not accept this read path, so its success-shaped events add no values. The plugin cases load the generated extensions and report first and repeated callback times in one Node process. These synthetic cases test the wrapper path only; they do not establish that Pi emits these shapes or promote additional native client support. This macOS run executes the POSIX wrapper; Windows PowerShell behavior has generated-command contract coverage, but no Windows runtime was available.

The runner exits nonzero and prints `INCOMPLETE` when Node.js is unavailable or a required stalled-child case cannot run on the current platform. On Windows, ordinary direct cases invoke the generated PowerShell wrapper; the current local stalled-child stubs are Unix-only, so stalled direct and plugin coverage is reported as incomplete instead of passing as skipped. Graph counts come from the already initialized `.rgt/store.db`, opened read-only; a missing store or any open/query error prints a case-specific diagnostic and exits nonzero. The runner does not create or initialize a replacement store while reading counts.

## Limits and Evidence Rules

- The hook input cap is 8 MiB. Input reading has a 250 ms elapsed deadline; the hook command reserves an 800 ms total deadline.
- Plugin subprocess wrappers use an 800 ms child timeout. The manual runner reports observed wall-clock timings without a flaky timing assertion in unit tests.
- A native automatic-capture claim requires a loaded registration and matching native success plus no-op/error evidence for the same read path, client version, RGT version, OS, and architecture. The success must include values with node IDs and source lines, an unchanged host result, accepted neutral response, and an exact current-file snapshot match.
- Keep source contents, private paths, credentials, and session identifiers out of the evidence ledger. Record path mappings and value summaries only.
- `rgt doctor` checks native registration and recording guidance at each documented local file path separately. Its CLI names missing, incomplete, unowned, managed, and hand-maintained guidance; incomplete instructions cause exit 1, while usable instruction-only guidance remains a warning with exit 0. An installer-managed RGT block needs its heading, recording command, and closing marker; hand-maintained instructions with the command but no closing marker receive an ownership warning. Cursor rules also need `alwaysApply: true` inside valid frontmatter. A readable file does not independently verify that Cursor, Gemini CLI, Copilot Chat, Pi, Hermes, Vibe, or another client loaded it, nor can it prove trust approval, hook invocation, or read capture.
- Hermes plugin registration is active only when the plugin file and the matching project or user `.hermes/config.toml` have a valid `plugins.enabled` array containing `rgt`. Doctor reports a missing or disabled entry as obsolete and malformed TOML or entry shape as malformed, with the matching config path and repair command. Hermes recording guidance is a separate finding.

## Local Results: 2026-09-26

Commands run locally:

```text
rtk proxy cargo build --bin rgt                                      exit 0
rtk proxy cargo bench --bench bench_hook_capture -- "$PWD/target/debug/rgt"  exit 0
```

Manual latency output from an earlier runner version (cold start can affect individual samples):

| Case | Payload | Time | Result |
|---|---:|---:|---|
| Direct successful read | 64 KiB | 19.1 ms | exit 0, neutral output, one root |
| Plugin wrapper, resultless Pi event | 64 KiB | 75.0 ms | exit 0, neutral output, no new roots |
| Direct repeated read | 64 KiB twice | 25.9 ms | exit 0, neutral output, one root total |
| Direct no-op | 64 KiB | 5.3 ms | exit 0, neutral output, zero roots |
| Direct malformed | 64 KiB | 3.4 ms | exit 0, neutral output, zero roots |
| Direct successful read | 1 MiB | 66.7 ms | exit 0, neutral output, one root |
| Plugin wrapper, resultless Pi event | 1 MiB | 76.8 ms | exit 0, neutral output, no new roots |
| PDF binary-envelope success | 1 MiB JSON / 787 KiB PDF | 52.3 ms | exit 0, one root, Claude additional context |
| PDF binary-envelope repeated | 1 MiB JSON | 52.4 ms | exit 0, one root total |
| PDF binary-envelope no-op | 1 MiB JSON | 29.3 ms | exit 0, neutral output, zero roots |
| PDF binary-envelope malformed | 1 MiB JSON | 10.0 ms | exit 0, neutral output, zero roots |
| Direct repeated read | 1 MiB twice | 135.1 ms | exit 0, neutral output, one root total |
| Direct no-op | 1 MiB | 30.0 ms | exit 0, neutral output, zero roots |
| Direct malformed | 1 MiB | 4.2 ms | exit 0, neutral output, zero roots |
| Oversized input | 8 MiB + 1 byte | 5.1 ms | exit 0, neutral output, zero roots |
| Direct hook input held open | no EOF | 262.6 ms | exit 0, neutral output, zero roots |
| Plugin child stall | 800 ms timeout | 889.2 ms total | neutral wrapper response, timeout observed |

The earlier runner passed its recorded cases. One cold subprocess-wrapper sample took 1.13 seconds; the runner did not import the generated plugin and measured only later runs at 73.9 ms and 75.0 ms. The updated report below measures the generated plugin's first and repeated callback directly. Neither run is an observation from OpenCode, Pi, or another native harness.

## Updated Generated-Plugin Timing: 2026-09-26

The generated Pi extension was exercised locally through Node after isolating each project and checking graph effects, callback return, and input immutability. Launch-to-response includes starting Node, importing the extension, and completing both callbacks. All plugin rows returned `undefined`, preserved the input, and left zero nodes and capture associations. The success-shaped read event remains ineligible for Pi capture by design.

| Case | Input | First callback | Repeated callback | Launch-to-response | Result |
|---|---:|---:|---:|---:|---|
| Completed-read-shaped text | 64 KiB | 4.7 ms | 4.6 ms | 91.0 ms | exit 0, neutral return, unchanged input, zero nodes/associations |
| No-op text event | 64 KiB | 5.0 ms | 4.1 ms | 83.9 ms | exit 0, zero nodes/associations |
| Malformed-shape text event | 64 KiB | 4.6 ms | 4.8 ms | 83.9 ms | exit 0, zero nodes/associations |
| Completed-read-shaped text | 1 MiB | 8.5 ms | 8.5 ms | 94.1 ms | exit 0, neutral return, unchanged input, zero nodes/associations |
| No-op text event | 1 MiB | 8.5 ms | 8.0 ms | 92.3 ms | exit 0, zero nodes/associations |
| Malformed-shape text event | 1 MiB | 8.4 ms | 7.6 ms | 93.1 ms | exit 0, zero nodes/associations |
| PDF envelope, success-shaped | 1 MiB JSON / 787 KiB PDF | 7.6 ms | 6.6 ms | 83.4 ms | exit 0, zero nodes/associations; Pi capture ineligible |
| PDF envelope, no-op | 1 MiB JSON / 787 KiB PDF | 7.9 ms | 6.9 ms | 85.3 ms | exit 0, zero nodes/associations |
| PDF envelope, malformed base64 | 1 MiB | 7.5 ms | 6.2 ms | 82.2 ms | exit 0, zero nodes/associations |
| PDF envelope, repeated success-shaped event | 1 MiB JSON / 787 KiB PDF | 7.8 ms | 6.4 ms | 83.3 ms | exit 0, zero nodes/associations; Pi capture ineligible |
| Oversized event | 8 MiB + 1 byte | 7.4 ms | 6.2 ms | 91.4 ms | exit 0, zero nodes/associations |
| Stalled child | 2 second stub | 804.3 ms | 803.4 ms | 1.691 s for both callbacks | each callback returned under one second; zero nodes/associations |

The timing report passed. The stalled row is two sequential callbacks, so its combined process time exceeds one second while each individual callback remains below one second. Synthetic payloads establish only local wrapper latency, not a live Pi completion-event contract. The earlier version of this table reported one root for its success-shaped plugin text events; that root came from a direct-hook case sharing the same project. The isolated rerun above confirms that Pi added no graph data.

## Native Client Availability

Local inspection on 2026-09-26 found `codex-cli 0.157.1` at `/opt/homebrew/bin/codex` and an Antigravity application in `/Applications`; Claude Code and Windsurf command-line executables were not found. No native client was launched because this environment does not establish that an isolated Codex or Antigravity session would be no-cost. The evidence ledger therefore remains `[]`; native read paths, client versions, and host loading are unverified. The support matrix must not promote synthetic results to verified support.

## Feature Version Command

The local CLI exposes both `rgt --version` and `rgt version`; both printed `rgt 0.6.0` in the local check. The contract test checks that each prints `rgt ${CARGO_PKG_VERSION}` and exits successfully.

## Final Local Gate: 2026-09-26

```text
rtk proxy cargo test --test test_store_integrity hook_capture_records_batch_atomically  exit 0
rtk proxy cargo test --test test_turtle_capture only_eligible_read_paths_persist_capture_lineage_across_processes  exit 0
rtk proxy cargo test --test test_turtle capture_snapshot_rejects_unknown_agents_dangling_nodes_and_duplicate_pairs  exit 0
rtk proxy cargo test  exit 0
rtk proxy cargo fmt --all -- --check  exit 0
rtk proxy cargo clippy --all-targets -- -D warnings  exit 0
```

The full suite includes ignored manual timing tests. Capture attribution export now accepts Codex as a known event-capable agent; the local Turtle integration test verifies Codex and Claude associations for matching source values. Other candidate client dialects without qualifying completion evidence remain no-ops. These are local contract results and do not promote any client to live-verified support.

## Convergence Follow-up: 2026-09-26

```text
rtk proxy cargo test  exit 0
rtk proxy cargo test --test test_hook_installer_contract direct_hook_fallback_commands_are_generated_for_unix_and_windows  exit 0
rtk proxy cargo test --test test_doctor_contract codex_health_distinguishes_bare_malformed_obsolete_duplicate_and_unresolvable_entries  exit 0
rtk proxy cargo fmt --all -- --check  exit 0
rtk proxy cargo clippy --all-targets -- -D warnings  exit 0
rtk proxy cargo build --bin rgt  exit 0
rtk proxy cargo bench --bench bench_hook_capture -- target/debug/rgt  exit 0
```

The manual report exercised the generated Pi extension and passed the cold/repeated, oversized, 1 MiB envelope, and stalled-child cases recorded above. No GitHub workflow or paid native-client session was run. US1 live acceptance remains open and every client/read-path support claim remains unverified.

## Convergence Deadline and Evidence Rerun: 2026-09-26

The direct command runner now invokes the generated shell wrapper, buffers child output until successful completion, and drops partial output after its 800 ms timeout. The wrapper explicitly forwards host stdin to the background RGT process. A stalled direct child and stalled plugin child both fail open without graph values.

Machine: macOS Darwin 27.0, arm64. Commands and final exit codes:

```text
rtk proxy cargo fmt --all -- --check                                      exit 0
rtk proxy cargo test                                                      exit 0
rtk proxy cargo clippy --all-targets -- -D warnings                       exit 0
rtk proxy cargo build --bin rgt                                           exit 0
rtk proxy cargo bench --bench bench_hook_capture -- target/debug/rgt      exit 0
```

Selected final manual latency results:

| Case | Input | Invocation-to-response | Result |
|---|---:|---:|---|
| Direct success, cold sample | 64 KiB text | 266.2 ms | exit 0, one root |
| Direct success | 1 MiB text | 76.9 ms | exit 0, one root |
| Direct no-op | 1 MiB | 38.0 ms | exit 0, zero roots |
| Direct malformed | 1 MiB | 11.5 ms | exit 0, zero roots |
| Direct repeated | 1 MiB, twice | 155.4 ms | exit 0, one root total |
| Plugin no-op | 64 KiB | 77.5 ms launch-to-response | exit 0, zero roots |
| Plugin malformed shape | 64 KiB | 78.6 ms launch-to-response | exit 0, zero roots |
| Plugin no-op | 1 MiB | 89.0 ms launch-to-response | exit 0, zero roots |
| Plugin malformed shape | 1 MiB | 86.8 ms launch-to-response | exit 0, zero roots |
| Direct oversized event | 8 MiB + 1 byte | 14.5 ms | exit 0, zero roots |
| Plugin oversized event | 8 MiB + 1 byte | 91.6 ms launch-to-response | exit 0, zero roots |
| Direct stdin held open | no EOF | 275.3 ms | exit 0, zero roots |
| Stalled direct command child | 2 second stub | 818.7 ms | exit 0, neutral output, zero roots, child terminated |
| Stalled plugin child | 2 second stub | 802.5 ms first callback; 803.6 ms repeated | both callbacks below 1 second |

The first attempt with the new shell wrapper exited 101 after a broken pipe exposed that background shell commands do not inherit host stdin by default. Adding explicit stdin forwarding fixed it; the rerun above passed. The 64 KiB cold direct sample was the slowest normal case and stayed below one second. Native client acceptance remains pending; these local checks do not change support tiers.

## Phase 10 Convergence Rerun: 2026-09-26

Machine: macOS Darwin, arm64. Commands and final exit codes:

```text
rtk proxy cargo fmt --all -- --check                                      exit 0
rtk proxy cargo build --bin rgt                                           exit 0
rtk proxy cargo bench --bench bench_hook_capture -- "$PWD/target/debug/rgt"  exit 0
```

Selected direct-hook measurements from the isolated rerun:

| Case | Input | Invocation-to-response | Result |
|---|---:|---:|---|
| Direct successful read, cold sample | 64 KiB text | 438.2 ms | exit 0, neutral output, one root |
| Direct repeated read | 64 KiB, twice | 47.9 ms | exit 0, neutral output, one root total |
| Direct no-op / malformed | 64 KiB | 16.6 / 13.6 ms | exit 0, zero roots |
| Direct successful read | 1 MiB text | 81.8 ms | exit 0, neutral output, one root |
| Direct PDF envelope success / repeat | 1 MiB JSON / 787 KiB PDF | 62.7 / 60.3 ms | exit 0, one root total |
| Direct PDF envelope no-op / malformed | 1 MiB JSON | 37.5 / 17.7 ms | exit 0, zero roots |
| Direct oversized event | 8 MiB + 1 byte | 14.1 ms | exit 0, zero roots |
| Direct stdin held open | no EOF | 274.8 ms | exit 0, neutral output, zero roots |
| Stalled direct command child | 2 second stub | 824.3 ms | exit 0, neutral output, zero roots, child terminated |
| Stalled Pi plugin child | 2 second stub | 804.3 / 803.4 ms per callback | both callbacks under one second; zero roots/associations |

The updated generated-plugin cases and their measured results are in [Updated Generated-Plugin Timing](#updated-generated-plugin-timing-2026-09-26). Plugin cases returned the neutral `undefined` result, preserved the input object, and left zero nodes and capture associations. These local synthetic checks do not establish native Pi behavior. No test suite, GitHub workflow, or paid native-client session was run in this rerun; US1 live acceptance remains open.

## Phase 11 Convergence Rerun: 2026-09-26

Machine: macOS, arm64. The manual runner completed with exit 0. It read node and capture-association counts from the preinitialized stores in read-only mode. Both stalled-child cases recorded their start markers and verified that the recorded child PIDs had terminated after the callback deadline.

```text
rtk proxy cargo fmt --all -- --check                                      exit 0
rtk proxy cargo build --bin rgt                                           exit 0
rtk proxy cargo bench --bench bench_hook_capture -- "$PWD/target/debug/rgt"  exit 0
```

Selected manual measurements:

| Case | Input | Invocation-to-response | Result |
|---|---:|---:|---|
| Direct successful read, cold sample | 64 KiB text | 426.2 ms | exit 0, neutral output, one root |
| Direct repeated read | 64 KiB, twice | 42.8 ms | exit 0, neutral output, one root total |
| Direct successful read | 1 MiB text | 75.2 ms | exit 0, neutral output, one root |
| Plugin no-op | 1 MiB text | 83.8 ms launch-to-response | exit 0, neutral return, unchanged input, zero nodes/associations |
| Plugin oversized event | 8 MiB + 1 byte | 90.2 ms launch-to-response | exit 0, neutral return, zero nodes/associations |
| Direct hook input held open | no EOF | 274.9 ms | exit 0, neutral output, zero roots |
| Stalled direct command child | 2 second stub | 823.7 ms | exit 0, neutral output, zero roots, started and terminated |
| Stalled plugin child | 2 second stub | 804.2 / 803.2 ms per callback | both callbacks under one second, zero nodes/associations, started and terminated |

This macOS host exercised both stalled-child paths. A Windows runtime was unavailable; the current stub implementations report those cases as `INCOMPLETE` with a nonzero runner exit on unsupported non-Unix platforms. A failure-path smoke run with Node removed from `PATH` emitted `INCOMPLETE` for each plugin and plugin-stall case and exited 1 as expected. No test suite, GitHub workflow, or paid native-client session was run for this rerun. US1 live acceptance remains open.

## Phase 12 Implementation Check: 2026-09-26

The stalled Pi runner now assigns separate start-marker and PID files to its first and repeated callbacks, checks that both child processes terminated, and verifies that the evidence validator rejects a one-child record. On Windows, direct cases now invoke the generated PowerShell hook command; the Unix path continues to run the generated POSIX wrapper. The Windows stalled-child stub remains unsupported and reports `INCOMPLETE`, which makes the runner exit nonzero rather than treating skipped coverage as a pass.

Local checks on macOS arm64:

```text
rtk proxy cargo fmt --all -- --check                                      exit 0
rtk proxy cargo check --bench bench_hook_capture                         exit 0
rtk proxy cargo check --target x86_64-pc-windows-gnu --bench bench_hook_capture  exit 101
```

The Windows cross-target check stopped in native dependency build scripts because `x86_64-w64-mingw32-gcc` is not installed; it did not reach compilation of RGT's Windows benchmark branch. No Windows runtime was available, so PowerShell execution remains unverified. No test suite or GitHub workflow was run for this implementation pass.

## Phase 13 Manual Benchmark Rerun: 2026-09-26

Machine: macOS arm64. Both commands exited 0:

```text
rtk proxy cargo build --bin rgt                                      exit 0
rtk proxy cargo bench --bench bench_hook_capture -- "$PWD/target/debug/rgt"  exit 0
```

Every runner row reported `PASS`. Selected measurements, including the cold plugin launch and both individual stalled callbacks:

| Case | Invocation-to-response | Graph / response result |
|---|---:|---|
| Direct successful read, 64 KiB | 426.6 ms | neutral, one root |
| Direct successful read, 1 MiB | 75.3 ms | neutral, one root |
| Direct repeated read, 64 KiB / 1 MiB | 43.8 / 152.4 ms | neutral, one root total per project |
| Direct no-op / malformed, 64 KiB | 12.8 / 11.3 ms | neutral, zero roots |
| Direct no-op / malformed, 1 MiB | 37.0 / 11.6 ms | neutral, zero roots |
| Direct PDF envelope success / repeat | 61.6 / 60.6 ms | one root total |
| Direct PDF envelope no-op / malformed | 36.6 / 18.2 ms | neutral, zero roots |
| Plugin text success-shaped, 64 KiB | 80.7 ms launch; 4.4 / 3.8 ms callbacks | neutral, unchanged input, zero nodes/associations |
| Plugin text no-op / malformed, 64 KiB | 76.3 / 75.0 ms launch | neutral, unchanged input, zero nodes/associations |
| Plugin text success-shaped / no-op / malformed, 1 MiB | 82.9 / 82.5 / 83.8 ms launch | neutral, unchanged input, zero nodes/associations |
| Plugin PDF envelope success / no-op / malformed / repeat | 83.8 / 82.7 / 83.0 / 82.4 ms launch | neutral, unchanged input, zero nodes/associations |
| Direct / plugin oversized event | 14.4 / 90.7 ms | neutral, zero nodes/associations |
| Direct stdin held open without EOF | 274.5 ms | neutral, zero roots |
| Stalled plugin child, first / repeated callback | 804.6 / 803.6 ms each; 1.678 s total | both started, both terminated, both PIDs recorded; missing-second check rejected one-child evidence; neutral, zero nodes/associations |
| Stalled direct child | 823.5 ms | neutral, zero roots, started and terminated |

All measured callback and direct-hook cases stayed below one second. The combined plugin process time exceeds one second because it runs two sequential 800 ms timeout callbacks; the host callback responses each met the limit. This run validates the per-callback evidence check on macOS. Windows runtime and PowerShell execution remain unverified, as recorded above. No test suite, GitHub workflow, or paid native-client session was run.

## Native OpenCode Read Acceptance: 2026-09-26

A disposable project on macOS arm64 used OpenCode CLI 1.18.32, RGT 0.6.0, and `opencode/mimo-v2.6-flash-free`. OpenCode's model catalog reported zero input, output, and cache cost for this model, and each step in the session reported `"cost":0`. No GitHub workflow ran. The project contained a two-line `sample.txt` with the synthetic values `314159` and `2026-09-26`, plus an `empty.txt` line without numeric or date values; neither file ended in a newline.

```text
rtk proxy cargo build --bin rgt                                      exit 0
rtk proxy <built-rgt> init --agent opencode                          exit 0, project plugin configured
rtk proxy <built-rgt> doctor                                         OpenCode registration active
rtk proxy opencode run --model opencode/mimo-v2.6-flash-free --format json 'Use the read tool on sample.txt and empty.txt, in that order. Say DONE only after both read calls succeed.'  exit 0
rtk proxy <built-rgt> status                                         2 active nodes, 0 stale nodes
rtk proxy <built-rgt> graph --format ttl                             two line-mapped entities and opencode capture association
```

The build ran from the repository root. The other commands ran from the disposable project, with `<built-rgt>` standing for the absolute path to the locally built executable.

The native session emitted two completed `read` tool results with the expected file text and line bounds. The installed plugin used `tool.execute.after`, returned no value, and did not modify either result; OpenCode accepted both and continued to its final `DONE` response. RGT created `node_raw_0de4b029d92d2b2ac78c700558b8ed3d` as Number `314159` on line 1 and `node_raw_c366873855668e51c0b935fdd506d7b7` as Date `2026-09-26` on line 2. Turtle included `prov:wasAssociatedWith` for `opencode` on both capture activities. The no-value read through the same path left the graph at two nodes. The redacted pair is in [`evidence/observations.json`](evidence/observations.json).

An initial diagnostic run with `opencode run --pure` completed the reads but created zero nodes because `--pure` disables external plugins. Removing that flag made the project plugin load. OpenCode's observed `metadata.display.text` omitted a terminal newline when one was present in the source; RGT requires an exact snapshot match and therefore leaves such reads uncaptured. Other client versions, platforms, global plugin loading, and other read result forms remain unverified.

After adding the redacted pair and the narrow parser/installer fixes, local `rtk proxy cargo test` exited 0 for the full suite, `rtk proxy cargo clippy --all-targets -- -D warnings` exited 0, `rtk proxy cargo fmt --all -- --check` exited 0, and `rtk git diff --check` exited 0. These commands ran locally; no GitHub Actions or CI/CD workflow was started.

## Phase 15 Local OpenCode Plugin Timing: 2026-09-26

The manual runner loaded the generated OpenCode plugin in Node and delivered successful `tool.execute.after` reads through `opencode-read-display-text`. Each source had exactly one line, no terminal newline, and one Number `42`. Two callbacks used the same event. The runner checked a neutral `undefined` callback return, unchanged event and result, one node on source line 1, and exactly one `opencode` capture association. The process output was a single parseable JSON timing record; all checks passed.

```text
rtk proxy cargo build --bin rgt                                      exit 0
rtk proxy cargo bench --bench bench_hook_capture -- target/debug/rgt  exit 0
```

| Source size | Input JSON | First callback | Repeated callback | Launch to response | Graph and response |
|---|---:|---:|---:|---:|---|
| 64 KiB | 65,897 B | 13.7 ms | 13.0 ms | 97.3 ms | Number 42, line 1, `opencode` association; neutral and unchanged |
| 1 MiB | 1,048,937 B | 47.4 ms | 46.8 ms | 164.7 ms | Number 42, line 1, `opencode` association; neutral and unchanged |

Both local plugin paths stayed below one second. These measurements are synthetic plugin invocations on this macOS host, not additional native OpenCode evidence. No GitHub workflow or paid model session ran.

The Phase 15 local verification also completed `rtk proxy cargo test`, `rtk proxy cargo clippy --all-targets -- -D warnings`, `rtk proxy cargo fmt --all -- --check`, and `rtk git diff --check` with exit 0. The focused parser, replay, installer, and doctor tests passed after the convergence edits.

## Phase 17 Local Guidance and Packaging Gate: 2026-09-26

The earlier `test_quickstart_cargo_package_clean` invoked `cargo package --allow-dirty` with no offline mode or deadline and included Cargo's package verification build. That made it an unbounded local check and limited the value of the earlier full-suite result. The test now invokes `cargo package --allow-dirty --offline --locked --no-verify`, kills a child that exceeds 30 seconds, and checks the package command's exit status. `--no-verify` skips the package verification build; the separate complete Cargo test covers compilation and behavior. The package test passed in 0.22 seconds on this host.

```text
rtk proxy cargo test --test test_distribution_quickstart test_quickstart_cargo_package_clean  exit 0
rtk proxy cargo test --test test_hooks_installer                                    exit 0
rtk proxy cargo test --test test_doctor_contract                                   exit 0
rtk proxy cargo test --test test_hook_installer_contract                           exit 0
rtk proxy cargo fmt --all -- --check                                               exit 0
rtk proxy cargo clippy --all-targets -- -D warnings                                exit 0
rtk proxy cargo test                                                               exit 0, complete local suite without exclusions
```

The first full-suite run stopped at two installer assertions for Vibe's obsolete prompt path; those assertions were updated to the documented loaded `AGENTS.md` path, and the second complete run exited 0. Doctor tests also exposed and verified a fix for an unquoted executable path in the generated Hermes Python plugin. The Pi, Hermes, and Vibe instruction checks are local artifact checks only. No hosted workflow, CI/CD run, or paid native-client session was started.

## Phase 18 Local Registration and Distribution Checks: 2026-09-26

Cursor's RGT-owned rule repair now handles LF and CRLF frontmatter in local and global initialization. The deterministic distribution tests inject update metadata instead of contacting GitHub Releases and check installer output without a latency pass threshold. Hermes doctor checks `plugins.enabled` in the config matching the plugin scope; missing, disabled, and malformed enablement no longer yields an active registration. The focused tests and final local quality checks completed:

```text
rtk proxy cargo test --test test_distribution_quickstart --test test_distribution_performance --test test_installer_script  exit 0
rtk proxy cargo test --test test_doctor_contract hermes_doctor_checks                                      exit 0
rtk proxy cargo fmt --all -- --check                                                                     exit 0
rtk proxy cargo clippy --all-targets -- -D warnings                                                     exit 0
rtk proxy cargo test                                                                                     exit 0, complete local suite without exclusions
rtk git diff --check                                                                                     exit 0
```

These are local deterministic checks. The optional offline release build and manual latency runner described above were not run in Phase 18. No hosted workflow or native-client verification ran in that pass; the earlier narrow OpenCode native evidence remains separate.

## Phase 19 Cursor and Hermes Follow-up Checks: 2026-09-26

Cursor regression tests now cover a description containing `alwaysApply: false` before the actual setting, conflicting duplicate keys, LF/CRLF preservation, local/global init, backups, reruns, and doctor output. Hermes editor and fake-home tests cover an active inline TOML config, adding missing `rgt` while preserving other entries and a backup, and conflicts for malformed array contents or value shapes. The completed local checks were:

```text
rtk proxy cargo test --test test_hooks_installer cursor_                    exit 0
rtk proxy cargo test --test test_hook_editors toml_ensure_array_value_    exit 0
rtk proxy cargo test --test test_hooks_installer hermes_inline_enablement_reinitializes_safely_in_both_scopes  exit 0
rtk proxy cargo fmt --all -- --check                                       exit 0
rtk proxy cargo clippy --all-targets -- -D warnings                       exit 0
rtk proxy cargo test                                                       exit 0, complete local suite without exclusions
rtk git diff --check                                                       exit 0
```

The release build, manual latency runner, hosted workflows, and native-client sessions were not run in Phase 19. Existing verified OpenCode evidence applies only to its recorded version, platform, and read path.
