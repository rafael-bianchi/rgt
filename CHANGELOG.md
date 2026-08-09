# Changelog

## [0.3.0](https://github.com/rafael-bianchi/rgt/compare/v0.2.0...v0.3.0) (2026-08-09)


### Features

* add derivation verification CLI subcommand ([d71f8bf](https://github.com/rafael-bianchi/rgt/commit/d71f8bfd43365767263690a817c6427da0e54ce9))
* add derivation verification CLI subcommand ([e8b60db](https://github.com/rafael-bianchi/rgt/commit/e8b60db1063b0eee21d4230ae86a0ac2e33600a7))
* add rgt record and rgt derive CLI subcommands ([79764d8](https://github.com/rafael-bianchi/rgt/commit/79764d88bd326abdc2f5594295b9ebe707ebcdd6))
* remove MCP server, replace with CLI-only active surface ([bb3c1ff](https://github.com/rafael-bianchi/rgt/commit/bb3c1ff4fc07abecb4d3c952fe29289eb05e0343))
* remove MCP server, replace with CLI-only active surface (RTK alignment) ([aaa4c73](https://github.com/rafael-bianchi/rgt/commit/aaa4c73e032349e9f52859a0c72fb02ab5cbd479))

## 0.2.0 (2026-08-03)


### Features

* add --agent flag to rgt init and agent-specific hook tests ([#2](https://github.com/rafael-bianchi/rgt/issues/2)) ([9e16228](https://github.com/rafael-bianchi/rgt/commit/9e162287f97d365986fb1aa9773a33ac05a9f725))
* add --agent flag to rgt init and agent-specific hook tests ([#2](https://github.com/rafael-bianchi/rgt/issues/2)) ([568db35](https://github.com/rafael-bianchi/rgt/commit/568db35115195bf036f23a90522fcafd8db5cb84))
* CI/CD pipeline security & delivery hardening ([70e712b](https://github.com/rafael-bianchi/rgt/commit/70e712bc82fe2d63d1bd5e7404bafd97be0e6bca))
* improve Duration display from raw seconds to human-readable format ([#5](https://github.com/rafael-bianchi/rgt/issues/5)) ([d52a41c](https://github.com/rafael-bianchi/rgt/commit/d52a41c8682f93cbdde6f534302d2436a4e579e2))
* improve Duration display from raw seconds to human-readable format ([#5](https://github.com/rafael-bianchi/rgt/issues/5)) ([6bf761a](https://github.com/rafael-bianchi/rgt/commit/6bf761af1c4833601c91d02b709e68c99f450906))
* initial implementation of RGT (Rust Graph Tracker) v0.1.0 ([b6b7bb7](https://github.com/rafael-bianchi/rgt/commit/b6b7bb7f12bde5f2de4492739c7ddd351d995312))
* multi-channel distribution & installation infrastructure ([8a07a13](https://github.com/rafael-bianchi/rgt/commit/8a07a138fcd1d990d3385aa14fa996dafb34357f))


### Bug Fixes

* add CWD mutex guard to all test files using set_current_dir ([697364c](https://github.com/rafael-bianchi/rgt/commit/697364c99981d6a3a0f7d5a6d9dcc8ba530bbeb7))
* add CWD mutex guard to all test files using set_current_dir ([773c96b](https://github.com/rafael-bianchi/rgt/commit/773c96bcc1c1291f08654ee90a96a4c83e930a15))
* address Copilot feedback on PR [#6](https://github.com/rafael-bianchi/rgt/issues/6) ([8d1eb40](https://github.com/rafael-bianchi/rgt/commit/8d1eb40c69beb3aab6de763094914200e1c1a658))
* address Copilot feedback on PR [#6](https://github.com/rafael-bianchi/rgt/issues/6) ([2d2334a](https://github.com/rafael-bianchi/rgt/commit/2d2334a1fc45ff30bd0cd3e2956a04888219d429))
* correct agent hook formats per official docs and RTK patterns ([cc50fc5](https://github.com/rafael-bianchi/rgt/commit/cc50fc5d4af6096d4eac719c274772c45097af90))
* correct agent hook formats per official docs and RTK patterns ([62d51d5](https://github.com/rafael-bianchi/rgt/commit/62d51d5effc680d18e486d8860c5f307e465b49e)), closes [#6](https://github.com/rafael-bianchi/rgt/issues/6)
* handle_query_provenance BFS traversal for deep derivation chains ([#1](https://github.com/rafael-bianchi/rgt/issues/1)) ([30b6a56](https://github.com/rafael-bianchi/rgt/commit/30b6a563ee705bb5854bd2a4e1b66bfebb8245e8))
* handle_query_provenance BFS traversal for deep derivation chains ([#1](https://github.com/rafael-bianchi/rgt/issues/1)) ([2a3e9a1](https://github.com/rafael-bianchi/rgt/commit/2a3e9a129de09273506bfa6757d1571a964c841a))
* hold CWD mutex for entire test duration to prevent race conditions ([1459633](https://github.com/rafael-bianchi/rgt/commit/1459633411d8fe33395bbce93ae70e680f2990df))
* hold CWD mutex for entire test duration to prevent race conditions ([dbccf35](https://github.com/rafael-bianchi/rgt/commit/dbccf351f0d35562f6a937ade2f468d0352f53b8))
* trigger release-please on develop instead of main ([9f4b65b](https://github.com/rafael-bianchi/rgt/commit/9f4b65b2b58aca5cdc20005d58954d5e2ecb3401))
* trigger release-please on develop instead of main ([b5c0198](https://github.com/rafael-bianchi/rgt/commit/b5c01984e6f4523cbf2d29eb52d597e4df2e6a15))
* update distribution channels and fix CI ([6099984](https://github.com/rafael-bianchi/rgt/commit/6099984d3de7cd74577ffc88ebe2be19478e358e))
* update distribution channels and fix CI ([#5](https://github.com/rafael-bianchi/rgt/issues/5)) ([634f6cd](https://github.com/rafael-bianchi/rgt/commit/634f6cdf35f5fa91e6c91318f285277c35d8bcd0))
* use aarch64-linux-gnu-strip for cross-compiled binaries ([a43a04c](https://github.com/rafael-bianchi/rgt/commit/a43a04c361145966d01fe38e8459b0b958dd2a8c))
* use aarch64-linux-gnu-strip for cross-compiled binaries ([d5b391f](https://github.com/rafael-bianchi/rgt/commit/d5b391f4fb1339f07cc3d9becf791ad0193fac98))
* use package name as manifest key for release-please ([bd2960b](https://github.com/rafael-bianchi/rgt/commit/bd2960bb13866402a9d5923543db62af060c27b2))
* use package name as manifest key for release-please ([7a28ad2](https://github.com/rafael-bianchi/rgt/commit/7a28ad24dd25244d1a682794f201e0d0b6b01222))


### Miscellaneous Chores

* release 0.2.0 ([3f6a5cd](https://github.com/rafael-bianchi/rgt/commit/3f6a5cd8de77d1db5debf755edaad225a805209b))
