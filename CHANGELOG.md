# Changelog

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
