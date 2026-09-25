# Turtle Export Performance Gate

The `Turtle export performance` workflow in
`.github/workflows/turtle-performance.yml` runs on pushes to
`feat/prov-o-turtle-export-pr` and also supports manual `workflow_dispatch`. It
runs the blocked stdout pipe probe on `ubuntu-24.04`, `macos-latest`, and
`windows-latest`. It runs the latency and quota-boundary probe on the standard
x86_64 `ubuntu-24.04` runner. The release gate remains pending until all three
pipe runs and the Linux latency run pass.

Each latency fixture is prepared before timing. Measurements include fresh
process startup and output to a local file, with one warm-up plus ten measured
runs. Successful cases must finish in under one second and produce the exact
expected output size. Over-limit cases must fail in under one second with
nonzero status and zero stdout. The pipe probe retains but does not read a pipe
with at least 2 MiB of valid Turtle; RGT must time out nonzero in under one
second after writing a partial prefix. Its five-second test watchdog only
detects a hung process and cannot count as a pass.

## Local macOS run

Date: 2026-09-25. These measurements are useful local evidence, but do not
substitute for the pinned GitHub-hosted Linux release gate or the cross-platform
pipe matrix.

| Field | Result |
|-------|--------|
| Host OS | macOS 26.6.2, build 25G83, arm64 |
| CPU and memory | Apple M5, 32 GiB |
| Rust | rustc 1.94.0 (4a4ef493e 2026-03-02), aarch64-apple-darwin |
| Release build | `cargo build --release --locked` passed |
| Latency and quota probe | `release_latency_boundaries` passed |
| Pipe probe | `blocked_stdout_pipe` passed |
| Hosted release decision | Pending hosted workflow matrix |

### Latency and quota boundaries

Every timed case had exit 0 when within the limit and exit 1 when over. Exact
limit cases had empty stderr. Over-limit cases had zero stdout and these
actionable errors: value count, `Turtle export supports at most 10000 recorded
values; found more`; edge count, `Turtle export supports at most 30000
derivation edges; found more`; byte count, `Turtle export exceeds the
67108864-byte output limit`.

| Case | Fixture counts | Target bytes | Warm-up ms | Ten measured elapsed times (ms) |
|------|----------------|-------------|------------|--------------------------------|
| Values at limit | 10,000 values, 0 edges, 1 source, 0 captures | 12,379,616 | 368 | 25, 19, 19, 19, 19, 18, 18, 18, 18, 18 |
| Values over limit | 10,001 values, 0 edges, 1 source, 0 captures | Error; stdout 0 | 6 | 6, 6, 6, 6, 6, 6, 6, 5, 6, 6 |
| Edges at limit | 10,000 values, 30,000 edges, 0 sources, 0 captures | 60,827,153 | 72 | 72, 68, 75, 74, 73, 69, 68, 69, 69, 69 |
| Edges over limit | 10,000 values, 30,001 edges, 0 sources, 0 captures | Error; stdout 0 | 6 | 6, 6, 6, 6, 6, 6, 6, 6, 6, 6 |
| Representative graph | 10,000 values, 8,000 edges, 6,000 sources, 10,000 captures | 39,558,938 | 149 | 149, 149, 149, 149, 154, 186, 163, 148, 149, 149 |
| Bytes at limit | 10,000 values, 8,000 edges, 6,000 sources, 10,000 captures | 67,108,864 | 179 | 181, 180, 180, 178, 181, 185, 185, 197, 190, 196 |
| Bytes over limit | 10,000 values, 8,000 edges, 6,000 sources, 10,000 captures | Error; stdout 0 (target 67,108,865) | 174 | 173, 174, 173, 175, 175, 173, 174, 175, 175, 173 |

All measured runs were below one second. The exact 64 MiB export produced
67,108,864 bytes; the 64 MiB plus one byte case failed before writing stdout.

### Blocked stdout pipe

| Runner | Image/version | CPU and memory | Rust | Fixture bytes | Elapsed | Partial bytes | Outcome |
|--------|---------------|----------------|------|---------------|---------|---------------|---------|
| Local macOS | macOS 26.6.2 build 25G83 | Apple M5, 32 GiB | 1.94.0, aarch64-apple-darwin | 3,501,058 | 688 ms | 65,536 | Pass; exit 1, stderr reported the one-second command deadline while writing stdout |
| ubuntu-24.04 | ubuntu24 20260920.314.1 | AMD EPYC 7763, 4 vCPUs, 15 GiB | 1.98.1, x86_64-unknown-linux-gnu | 3,501,058 | 402 ms | 65,536 | Pass; exit 1, deadline error while writing stdout |
| macos-latest | macos26 20260907.0351.1 | Apple M1 (Virtual), 7 GiB | 1.98.1, aarch64-apple-darwin | 3,501,058 | 500 ms | 65,536 | Pass; exit 1, deadline error while writing stdout |
| windows-latest | win25-vs2026 20260907.229.1 | AMD64 Family 26 Model 2, 16,379 MB | 1.98.1, x86_64-pc-windows-msvc | 3,501,058 | 425 ms | 0 | Observed exit 1 and deadline error; probe assertion failed because Windows exposed no partial bytes |

## Hosted run results

The [hosted workflow run](https://github.com/rafael-bianchi/rgt/actions/runs/36160369132)
used commit `39b0b22`. Its Ubuntu and macOS jobs passed. Windows built and
exited with the required deadline error in 425 ms, but the probe failed an
extra assertion that the pipe expose a partial write. Windows can keep an
anonymous-pipe write pending without exposing any bytes; the local probe now
checks the portable deadline and exit contract, pending a hosted rerun. The
complete release decision remains pending. An earlier run
([36159712200](https://github.com/rafael-bianchi/rgt/actions/runs/36159712200))
failed to build on Windows because `develop` used unstable Windows metadata
methods; commit `39b0b22` replaced that path with the existing stable fallback.

### Hosted Ubuntu latency and quota boundaries

Runner: ubuntu24 20260920.314.1, Ubuntu 24.04.5, x86_64, AMD EPYC 7763 (4
vCPUs), 15 GiB RAM, rustc 1.98.1 (48a229cea, 2026-09-01). The test ran one
warm-up and ten measured release CLI invocations per case, including process
startup and local-file output. The fixture counts below are values, edges,
source documents, and capture records in that order. Each exact-limit or
representative case exited 0 with empty stderr and the listed stdout bytes;
each over-limit case exited 1 with zero stdout and the error listed below.

| Case | Fixture counts | Stdout bytes | Warm-up ms | Ten measured elapsed times (ms) |
|------|----------------|-------------:|-----------:|--------------------------------|
| Values at limit | 10,000 / 0 / 1 / 0 | 12,379,616 | 40 | 40, 40, 40, 40, 40, 40, 40, 40, 40, 40 |
| Values over limit | 10,001 / 0 / 1 / 0 | 0 | 5 | 5, 5, 5, 5, 5, 5, 5, 5, 5, 5 |
| Edges at limit | 10,000 / 30,000 / 0 / 0 | 60,827,153 | 157 | 162, 162, 162, 162, 162, 162, 162, 157, 157, 162 |
| Edges over limit | 10,000 / 30,001 / 0 / 0 | 0 | 5 | 5, 5, 5, 5, 5, 5, 5, 5, 5, 5 |
| Representative graph | 10,000 / 8,000 / 6,000 / 10,000 | 39,558,938 | 152 | 152, 152, 152, 152, 152, 152, 152, 152, 152, 152 |
| Bytes at limit | 10,000 / 8,000 / 6,000 / 10,000 | 67,108,864 | 263 | 264, 264, 264, 264, 264, 264, 258, 264, 269, 263 |
| Bytes over limit | 10,000 / 8,000 / 6,000 / 10,000 | 0 | 243 | 253, 249, 249, 249, 244, 249, 249, 249, 248, 249 |

The over-limit errors were, respectively, `Turtle export supports at most
10000 recorded values; found more`, `Turtle export supports at most 30000
derivation edges; found more`, and `Turtle export exceeds the 67108864-byte
output limit`. The rendered target of the last case was 67,108,865 bytes;
it failed before writing stdout. All measured invocations finished in under
one second.
