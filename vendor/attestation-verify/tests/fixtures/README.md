# Test fixtures

Real-world fixtures captured 2026-07-29 with gh CLI against the public
repository `cli/cli`, release `v2.96.0`. Nothing here is secret: all files
are public release metadata, public attestations, and public trust material.

## github-cli/

| File | What it is |
|---|---|
| `gh_2.96.0_checksums.txt` | Small (1.9 KB) real release artifact, committed so `verify_bytes` tests can hash real bytes. |
| `gh_2.96.0_checksums.txt.sha256` | Its SHA-256 (subject digest). |
| `gh_2.96.0_linux_amd64.tar.gz.sha256` | Digest of the 12 MB tarball (artifact itself deliberately not committed; digest-only fixtures drive `verify_digest`). |
| `tarball-user-slsa-provenance.json` | **Primary v0.1 golden fixture.** Workflow-generated bundle (`initiator: user`, `actions/attest-build-provenance` path): media type `bundle.v0.3+json`, predicate `https://slsa.dev/provenance/v1`, Rekor **v1** entry (`kind: dsse, version: 0.0.1`) with **SET (inclusionPromise) + inclusionProof + checkpoint**, no TSA. The in-toto statement carries **21 subjects** (the whole release) — multi-subject statements are the normal case, not an edge case. |
| `tarball-github-release-tsa.json` | GitHub-initiated release attestation (`initiator: github`): predicate `https://in-toto.io/attestation/release/v0.2`, **no tlog entries**, RFC 3161 timestamp from GitHub's TSA. Verifies against the GitHub trust root, not public-good. v0.2 scope material. |
| `checksums-gh-download.jsonl` | Output of `gh attestation download gh_2.96.0_checksums.txt -R cli/cli` — the JSONL container form (one bundle line: the github-initiated one; the checksums artifact has no workflow provenance). |
| `attestations-api-response.redacted.json` | Shape of `GET /repos/{owner}/{repo}/attestations/sha256:{digest}` as of 2026-07: bundles are NOT inline (`bundle: null`); each entry has `initiator` (`github` \| `user`), `repository_id`, and a short-lived signed `bundle_url` that serves **raw-snappy-compressed** bundle JSON. SAS tokens redacted. Acquisition trivia lives outside the sans-io core, but the parser fixture for `BundleSet::from_github_response` input shapes starts here. |

## trusted-roots/

Split from `gh attestation trusted-root` (JSONL, two roots):

| File | What it is |
|---|---|
| `public-good.json` | Sigstore public-good trusted root: Fulcio CAs (`fulcio.sigstore.dev`, two generations), Rekor **v1** log (`rekor.sigstore.dev`, ECDSA P-256) **and** Rekor **v2** log (`log2025-1.rekor.sigstore.dev`, Ed25519), Sigstore TSA. Source of the crate's embedded root. |
| `github.json` | GitHub's own trust root: six `fulcio.githubapp.com` CA generations, six `timestamp.githubapp.com` TSAs, **no transparency logs**. Needed for `initiator: github` / private-repo verification (v0.2). |

## Reproduction

```sh
gh release download v2.96.0 --repo cli/cli --pattern 'gh_2.96.0_checksums.txt'
shasum -a 256 gh_2.96.0_checksums.txt
gh attestation download gh_2.96.0_checksums.txt -R cli/cli
gh api "repos/cli/cli/attestations/sha256:<digest>"   # bundle_url entries, raw-snappy payload
gh attestation trusted-root
```

Captured with gh 2.x on 2026-07-29. If fixtures are ever refreshed, update
the findings above — they gate the Rekor v1/v2 scope decision in DESIGN.md.
