# Changelog

## [0.8.0](https://github.com/rafael-bianchi/rgt/compare/v0.7.0...v0.8.0) (2026-10-19)


### Features

* agent-instructions feature parity (doctor, gc, --number-format) ([763dcd3](https://github.com/rafael-bianchi/rgt/commit/763dcd317020c40981d16bdfd626bcfb29066de6))
* **harnesses:** add multi-agent harness testing environment, Sakura server runner, and GitHub Actions workflow ([faac2b1](https://github.com/rafael-bianchi/rgt/commit/faac2b1f1c63873b30b85cdf56a0ab80f89ffabc))
* MCP server mode (rgt mcp) exposing record/verify/derive/query/status as tools ([26d374f](https://github.com/rafael-bianchi/rgt/commit/26d374fc4ec0ea6204204951fc966a9a5ac4d25c))
* store staleness, schema v3 migration and integrity hardening (spec 045) ([ec7e19c](https://github.com/rafael-bianchi/rgt/commit/ec7e19c40b9189b119ee2fa352fe66fc91c87e7e))


### Bug Fixes

* **047:** harden arithmetic verification, tolerance, and duration formatting ([bfabdcb](https://github.com/rafael-bianchi/rgt/commit/bfabdcba5bd6a951f9664cb0f2fa7f523de9fb83))
* **048:** harden harness hook payloads, OpenCode capture, and Windows shell parsing ([d5e6af2](https://github.com/rafael-bianchi/rgt/commit/d5e6af2339b99fb44a4b1f3d043f6f1164181245))
* address post-review audit edge cases, clippy lints, and formatting ([52f15c4](https://github.com/rafael-bianchi/rgt/commit/52f15c4aa4cf2010057256b7c5673b63dd6b0c85))
* **cli:** harden status, query, turtle export, updater, and build hygiene (Spec 050) ([4b6ad56](https://github.com/rafael-bianchi/rgt/commit/4b6ad569d9286efa52efc0f2130e963d7afb9a82))
* **dist:** harden install.sh POSIX compliance and fix CD release permissions & Homebrew updates ([8d23f43](https://github.com/rafael-bianchi/rgt/commit/8d23f43ee25cef87efbea8c86e892628280e2236))
* **doctor:** discover project root dynamically instead of requiring store.db in cwd ([add0389](https://github.com/rafael-bianchi/rgt/commit/add038973fb2036903478e845bb3bf311854d649))
* **extract:** harden value extraction, date masking, and stdin recording ([44e0a59](https://github.com/rafael-bianchi/rgt/commit/44e0a59143b08e1f9fc92252d0aacb51a77f6ff5))
* **hooks:** harden agent installation, guidance scopes, detection, and doctor diagnostics (Spec 049) ([2f82ff3](https://github.com/rafael-bianchi/rgt/commit/2f82ff3b7491202df5cebd0254444b115b69b143))
* remediate pass-2 adversarial review findings across harnesses, provenance graph, store, MCP, and CI/CD (Spec 052) ([430bbcf](https://github.com/rafael-bianchi/rgt/commit/430bbcf6959d4eb2f765a7a9151ee184e243527f))
* remediate pass-3 critical review findings across store, parser, harnesses, MCP, updater, and CI/CD (Spec 053) ([841a788](https://github.com/rafael-bianchi/rgt/commit/841a788a013aadba626c4351cb6e73a7218b51ef))
* rgt query default prints human-readable lineage, --json is clean JSON ([bb0f50d](https://github.com/rafael-bianchi/rgt/commit/bb0f50dbc4b6dc6c3c27a19e59681ab9fee0d8e7))

## [0.7.0](https://github.com/rafael-bianchi/rgt/releases/tag/v0.7.0) (2026-09-26)

### Features and fixes

* Register native Codex and Windsurf hooks and repair clearly RGT-owned obsolete registrations while preserving unrelated client settings.
* Capture values only from successful completed reads whose full content matches the current source snapshot; failed, before-only, partial, and path-only events create no source-backed values.
* Show local hook and guidance health in `rgt doctor`, with a versioned compatibility matrix that separates configured integrations from verified automatic capture.
* Print the running build version with `rgt --version` or `rgt version`.

### Release note

This release is built locally while GitHub Actions are disabled. Assets have SHA-256 checksums but no GitHub Artifact Attestation. Updating to v0.7.0 requires `rgt update --yes --skip-attestation`; checksum verification remains enabled. Automatic value capture is verified only for the OpenCode read path and version/platform listed in the compatibility matrix. Other installed hooks and guidance do not yet have live-client capture verification.

## [0.6.0](https://github.com/rafael-bianchi/rgt/releases/tag/v0.6.0) (2026-09-26)

### Features

* Export project-scoped PROV-O Turtle with typed values, supported derivation activities, and captured agent attribution.
* Derive exact whole-second date gaps automatically, sum or average Duration nodes without losing their type, and choose fixed display units when deriving or querying.
* Capture text extracted from Claude Code PDF reads when the tool supplies a PDF envelope.

### Release note

This release is built locally while GitHub Actions are disabled. Assets have SHA-256 checksums but no GitHub Artifact Attestation. Updating to v0.6.0 requires `rgt update --yes --skip-attestation`; checksum verification remains enabled.

## [0.5.0](https://github.com/rafael-bianchi/rgt/releases/tag/v0.5.0) (2026-08-30)

Installer preservation, update integrity, numeric and date extraction fixes, wider agent capture, locale-aware recording, `rgt doctor`, and store maintenance. See the linked release for details.

## [0.4.0](https://github.com/rafael-bianchi/rgt/releases/tag/v0.4.0) (2026-08-22)

### Features

* expand agent hook coverage to all 13 RTK-supported agents: `--agent` flag, auto-detection of installed agents, per-agent stdin normalizers, and thin glue plugins/rules files for Copilot, Gemini, Mistral Vibe, OpenCode, Pi, Hermes, Cline/Roo Code, Antigravity, and Kilo. Fix Copilot CLI config path resolution on macOS.
