# RGT compatibility patch

Imported from crates.io `attestation-verify 0.1.0`, MIT OR Apache-2.0.
Upstream: https://github.com/combinatrix-ai/attestation-verify

Only production change: `src/rekor.rs` accepts the observed compact DSSE
serialization with an empty `keyid` as well as the existing serialization
without it, exclusively when the parsed key ID is empty or absent. The
candidate hash must equal the hash in the authenticated Rekor entry. Non-empty
key IDs never receive the empty-key alternative. Payload, signature,
certificate, SET, inclusion proof, checkpoint, digest and identity checks remain.

Reason: GitHub's October 2026 RGT release bundle strips the empty key ID, but
the original submitted envelope was hashed with `keyid: ""`. The unpatched
crate only supports the Go submitter's omitted-empty-field serialization.

Replace this vendored dependency with a pinned upstream release once this
compatibility behavior and the real RGT regression fixture pass upstream.
