# Contributing to RGT

## Project Structure

```
src/
├── cli/           # CLI subcommands: init, status, query, graph, update, verify
├── query/         # Query module: provenance, stale listing
├── store/         # SQLite: db connection, queries, schema
├── types/         # Data types: node, edge, value
├── graph/         # DAG: engine (petgraph), invalidation
├── detection/     # File change detection: tier1, tier2 (BLAKE3)
├── hooks/         # Agent hook installer and parser
├── updater/       # Self-update: github API, checksum, platform
├── verify/        # Derivation verification: expression, date_diff
├── lib.rs         # Library root
└── main.rs        # Binary entry point

tests/
├── contract/      # Schema and protocol validation
├── integration/   # End-to-end flows
└── unit/          # Isolated logic tests
```

## Setup

```bash
git clone https://github.com/rafael-bianchi/rgt.git
cd rgt
cargo build
```

## Running Tests

```bash
cargo test --all -- --test-threads=1
```

Tests use `--test-threads=1` because integration tests change the working directory via `std::env::set_current_dir` and share a static `CWD_MUTEX`.

Individual test categories:

```bash
cargo test --lib                          # unit tests
cargo test --test test_hooks_installer    # hook installer integration
cargo test --test test_derivation_chain   # derivation chain E2E
cargo test verify                         # verification module
```

## Linting

```bash
cargo fmt --all -- --check
cargo clippy --all-targets
```

## Commit Conventions

Use Conventional Commits: `type: description (#N)`. Types: `feat`, `fix`, `docs`, `style`, `refactor`, `test`, `chore`.

## Branching

- Branch from `develop`
- PR to `develop`
- `main` is promoted automatically by release-please

## License

MIT OR Apache-2.0. All contributions must be compatible with both licenses.
