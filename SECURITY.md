# Security

## Reporting a vulnerability

Open a private issue or contact the maintainers via GitHub. Do not disclose a
zero-day or actively-exploited vulnerability in a public issue.

## Provenance and supply-chain posture

RGT is distributed as a single static Rust binary. Its integrity guarantees
rest on two mechanisms:

### 1. GitHub Artifact Attestations (Sigstore) for `rgt update`

`rgt update` installs only **verified** binaries: every download is checked
against the release's `checksums.txt` (SHA-256) **and** its GitHub Artifact
Attestation — a Sigstore bundle (DSSE signature, Fulcio certificate chain,
embedded SCT, Rekor v1 SET + Merkle inclusion + checkpoint) verified in-binary
against the embedded Sigstore public-good trust root, plus an identity policy
pinning the source/signer to `rafael-bianchi/rgt`.

**Dependency-maturity risk acceptance (recorded).** The signature verification
is implemented with the `attestation-verify` crate, pinned **exactly** to
`=0.1.0` in `Cargo.toml`. At the time of this writing that crate is brand-new
with essentially zero adoption, and it carries the entire trust guarantee for
`rgt update`'s signature check. We accept this interim risk because:

- The updater is **fail-closed**: any verification failure refuses the install.
  A defect in the dependency would surface as a refusal, never a silent
  acceptance of an untrusted binary.
- The trust root is the independent Sigstore public-good transparency log, not
  a second hash from the same release JSON.

The exact pin is deliberate: a future `0.1.x` of `attestation-verify` must be a
reviewed, intentional upgrade, not an incidental `cargo update` side effect.

**Untested-in-production gap.** The production verification policy
(`rgt_policy()` in `src/updater/attestation.rs`) has never been exercised
against a real signed `rafael-bianchi/rgt` release. The self-test
`verifies_real_rgt_bundle_when_fixture_present`
(`tests/unit/test_update_attestation.rs`) self-skips because no attested
release exists yet.

**Remediation trigger.** The first release built through the workflow that
signs artifacts (`attest-build-provenance` in `.github/workflows/release.yml`)
closes this gap:

```sh
scripts/capture-rgt-attestation-fixture.sh vX.Y.Z
```

That captures the real bundle as `tests/fixtures/rgt/rgt-bundle.json` +
`artifact.sha256`; the gated self-test then verifies it against the production
policy for real. If verification fails, the recorded identity differs from the
policy constants — update `src/updater/attestation.rs` (see
`tests/fixtures/rgt/README.md` for the full procedure).

### 2. Signed/verified release assets

Release archives are checksummed in `checksums.txt` and covered by the
attestation above; `install.sh` verifies the downloaded archive's checksum
before installing.

## Other notes

- No telemetry, no network calls during normal operation, no external services
  beyond the GitHub API used by `rgt update`.
- Third-party GitHub Actions used in CI are pinned to commit SHA with the
  version tag preserved as a comment, so a re-pointed mutable tag cannot
  inject steps into workflows holding elevated permissions.
