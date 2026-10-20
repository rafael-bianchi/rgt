//! GitHub identity policy types (DESIGN.md "Identity policy").
//!
//! This module defines *what identity to require*: source repository,
//! signer workflow, and revision. It intentionally contains no matching
//! logic — comparing a policy against a certificate's authenticated claims
//! is verification work, not parsing work, and belongs to a later task.
//! Every constructor here validates its own inputs, so any `GithubPolicy`
//! you can hold is already well-formed.

use crate::error::{Error, PolicyError};
use crate::parse_util;
use crate::trust::TransparencyLog;

fn policy_error(reason: String) -> Error {
    Error::Policy(PolicyError::InvalidConfiguration(reason))
}

/// The exact signed checkpoint-origin strings accepted for a trusted Rekor
/// signing key.
///
/// A checkpoint origin is part of the signed-note body, but it is not a
/// useful deployment identity until the note signature has been authenticated
/// under a caller-selected trusted key.  This policy therefore binds opaque,
/// byte-for-byte origin strings to the SHA-256 digest of the selected log
/// key's raw `SubjectPublicKeyInfo` bytes.  It is deliberately separate from
/// [`crate::trust::TrustStore`]: trust roots describe which keys are trusted,
/// while this type describes which deployment origins a caller accepts for
/// those keys.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckpointOriginPolicy {
    bindings: Vec<CheckpointOriginBinding>,
}

/// One log-key identity and the exact checkpoint origins allowed for it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckpointOriginBinding {
    key_id: [u8; 32],
    origins: Vec<String>,
}

impl CheckpointOriginBinding {
    /// Creates a binding from a trusted log and one or more exact origin
    /// strings. The key identity is derived internally as
    /// SHA-256(SPKI/raw public-key bytes).
    ///
    /// Empty origins and origins containing CR/LF are rejected.  Origins are
    /// otherwise opaque: no URL, case, Unicode, or whitespace normalization
    /// is performed.
    ///
    /// # Errors
    ///
    /// Returns [`PolicyError::InvalidConfiguration`] for an empty origin
    /// list, an empty origin, or an origin containing CR/LF.
    pub fn new<I, S>(log: &TransparencyLog, origins: I) -> Result<Self, Error>
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let origins = validate_checkpoint_origins(origins)?;
        Ok(Self {
            key_id: checkpoint_log_key_id(log),
            origins,
        })
    }

    /// The SHA-256 digest of the selected key's raw SPKI bytes.
    #[must_use]
    pub(crate) fn key_id(&self) -> [u8; 32] {
        self.key_id
    }

    /// The exact allowed origin strings.
    #[must_use]
    pub fn origins(&self) -> &[String] {
        &self.origins
    }
}

impl CheckpointOriginPolicy {
    /// Creates a checkpoint-origin policy from key-to-origin bindings.
    ///
    /// The policy must contain at least one binding, and every binding must
    /// contain at least one valid origin.  Duplicate key bindings are allowed
    /// and are evaluated as a union; this keeps construction deterministic
    /// while allowing callers to assemble policy fragments independently.
    ///
    /// # Errors
    ///
    /// Returns [`PolicyError::InvalidConfiguration`] when no bindings are
    /// supplied.
    pub fn new<I>(bindings: I) -> Result<Self, Error>
    where
        I: IntoIterator<Item = CheckpointOriginBinding>,
    {
        let bindings = bindings.into_iter().collect::<Vec<_>>();
        if bindings.is_empty() {
            return Err(policy_error(
                "checkpoint origin policy must not be empty".to_owned(),
            ));
        }
        Ok(Self { bindings })
    }

    /// Creates a policy containing one trusted log-key binding.
    ///
    /// This is a convenience for the common one-log deployment; use
    /// [`CheckpointOriginPolicyBuilder`] when several keys or origins are
    /// needed.
    ///
    /// # Errors
    ///
    /// Returns [`PolicyError::InvalidConfiguration`] for an empty origin
    /// list, an empty origin, or an origin containing CR/LF.
    pub fn for_log<I, S>(log: &TransparencyLog, origins: I) -> Result<Self, Error>
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self::new([CheckpointOriginBinding::new(log, origins)?])
    }

    /// Starts building a checkpoint-origin policy.
    #[must_use]
    pub fn builder() -> CheckpointOriginPolicyBuilder {
        CheckpointOriginPolicyBuilder::default()
    }

    /// Returns the policy bindings.
    #[must_use]
    pub(crate) fn bindings(&self) -> &[CheckpointOriginBinding] {
        &self.bindings
    }

    /// Returns whether `origin` is allowed for the selected SPKI digest.
    #[must_use]
    pub(crate) fn allows(&self, key_id: &[u8; 32], origin: &str) -> bool {
        self.bindings
            .iter()
            .filter(|binding| &binding.key_id == key_id)
            .any(|binding| binding.origins.iter().any(|allowed| allowed == origin))
    }
}

/// Builder for [`CheckpointOriginPolicy`].
#[derive(Debug, Clone, Default)]
pub struct CheckpointOriginPolicyBuilder {
    bindings: Vec<CheckpointOriginBinding>,
}

impl CheckpointOriginPolicyBuilder {
    /// Adds one key-to-origin binding.
    #[must_use]
    pub fn binding(mut self, binding: CheckpointOriginBinding) -> Self {
        self.bindings.push(binding);
        self
    }

    /// Adds one exact origin for a key.  Validation runs when this method is
    /// called, so malformed origins cannot enter a builder.
    ///
    /// # Errors
    ///
    /// Returns [`PolicyError::InvalidConfiguration`] for an empty origin or
    /// an origin containing CR/LF.
    pub fn allow_origin(
        mut self,
        log: &TransparencyLog,
        origin: impl Into<String>,
    ) -> Result<Self, Error> {
        let key_id = checkpoint_log_key_id(log);
        let origin = validate_checkpoint_origin(origin.into())?;
        if let Some(binding) = self.bindings.iter_mut().find(|b| b.key_id == key_id) {
            if !binding.origins.contains(&origin) {
                binding.origins.push(origin);
            }
        } else {
            self.bindings.push(CheckpointOriginBinding {
                key_id,
                origins: vec![origin],
            });
        }
        Ok(self)
    }

    /// Adds several exact origins for one log key.
    ///
    /// # Errors
    ///
    /// Returns [`PolicyError::InvalidConfiguration`] if any origin is empty
    /// or contains CR/LF.
    pub fn allow_origins<I, S>(mut self, log: &TransparencyLog, origins: I) -> Result<Self, Error>
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        for origin in origins {
            self = self.allow_origin(log, origin)?;
        }
        Ok(self)
    }

    /// Builds the policy, rejecting an empty policy.
    ///
    /// # Errors
    ///
    /// Returns [`PolicyError::InvalidConfiguration`] when no bindings were
    /// added.
    pub fn build(self) -> Result<CheckpointOriginPolicy, Error> {
        CheckpointOriginPolicy::new(self.bindings)
    }
}

fn checkpoint_log_key_id(log: &TransparencyLog) -> [u8; 32] {
    use sha2::{Digest, Sha256};

    Sha256::digest(&log.public_key.raw_bytes).into()
}

fn validate_checkpoint_origins<I, S>(origins: I) -> Result<Vec<String>, Error>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    let origins = origins
        .into_iter()
        .map(Into::into)
        .map(validate_checkpoint_origin)
        .collect::<Result<Vec<_>, _>>()?;
    if origins.is_empty() {
        return Err(policy_error(
            "checkpoint origin binding must contain at least one origin".to_owned(),
        ));
    }
    Ok(origins)
}

fn validate_checkpoint_origin(origin: String) -> Result<String, Error> {
    if origin.is_empty() {
        return Err(policy_error(
            "checkpoint origin must not be empty".to_owned(),
        ));
    }
    if origin.contains(['\r', '\n']) {
        return Err(policy_error(
            "checkpoint origin must not contain CR or LF".to_owned(),
        ));
    }
    Ok(origin)
}

/// A GitHub identity policy: the source repository/ref/commit an artifact
/// must come from, and the workflow that must have signed it.
///
/// Constructed via [`GithubPolicy::builder`]. The OIDC issuer is pinned
/// internally to `https://token.actions.githubusercontent.com` and is not
/// configurable.
#[derive(Debug, Clone)]
pub struct GithubPolicy {
    /// Where the code came from.
    pub source: SourcePolicy,
    /// Which workflow signed it.
    pub signer: SignerPolicy,
}

impl GithubPolicy {
    /// Starts building a [`GithubPolicy`].
    #[must_use]
    pub fn builder() -> GithubPolicyBuilder {
        GithubPolicyBuilder::default()
    }
}

/// Builder for [`GithubPolicy`]. Validation happens once, in
/// [`GithubPolicyBuilder::build`]; a successfully-built `GithubPolicy` is
/// guaranteed well-formed from then on (DESIGN.md "Core decisions" item
/// 5: policy is validated once at build time and reused).
#[derive(Debug, Clone, Default)]
pub struct GithubPolicyBuilder {
    source: Option<SourcePolicy>,
    signer: Option<SignerPolicy>,
}

impl GithubPolicyBuilder {
    /// Sets the source policy (where the code came from).
    #[must_use]
    pub fn source(mut self, source: SourcePolicy) -> Self {
        self.source = Some(source);
        self
    }

    /// Sets the signer policy (which workflow signed it).
    #[must_use]
    pub fn signer(mut self, signer: SignerPolicy) -> Self {
        self.signer = Some(signer);
        self
    }

    /// Builds the policy.
    ///
    /// # Errors
    ///
    /// Returns [`PolicyError::InvalidConfiguration`] if either
    /// [`GithubPolicyBuilder::source`] or [`GithubPolicyBuilder::signer`]
    /// was never called, or if the signer repository pins a numeric owner
    /// or repository id (see [`SignerPolicy`]: no certificate claim can
    /// enforce it).
    pub fn build(self) -> Result<GithubPolicy, Error> {
        let source = self
            .source
            .ok_or_else(|| policy_error("source policy is required".to_owned()))?;
        let signer = self
            .signer
            .ok_or_else(|| policy_error("signer policy is required".to_owned()))?;
        reject_signer_id_pins(&signer)?;
        Ok(GithubPolicy { source, signer })
    }
}

/// Rejects a signer policy that pins numeric ids.
///
/// [`GithubPolicy`]'s fields are public, so a caller can assemble one
/// without [`GithubPolicyBuilder`]; every path that turns a policy into
/// something enforceable must run this check.
pub(crate) fn reject_signer_id_pins(signer: &SignerPolicy) -> Result<(), Error> {
    if signer.repository.owner_id().is_some() || signer.repository.repository_id().is_some() {
        return Err(policy_error(
            "signer repository numeric id pins cannot be enforced: a Fulcio certificate carries \
             no authenticated signer-repository owner or repository id claim, only the signer's \
             owner/name in the SAN URI. Remove with_owner_id/with_repository_id from the signer \
             repository (source-side numeric id pins are enforced)"
                .to_owned(),
        ));
    }
    Ok(())
}

/// Where the code must come from: a repository, a ref policy, and
/// optionally an exact commit.
#[derive(Debug, Clone)]
pub struct SourcePolicy {
    /// The required source repository.
    pub repository: RepositoryIdentity,
    /// The required git ref.
    pub git_ref: RefPolicy,
    /// An optional exact commit requirement, independent of `git_ref`.
    pub commit: Option<CommitSha>,
}

/// Which workflow must have signed: a repository, a workflow file path,
/// and a revision policy.
///
/// Kept separate from [`SourcePolicy`] because reusable workflows make
/// "repository + workflow path" alone ambiguous: the workflow file can
/// live in a different repository than the code it signed for.
///
/// The signer repository is matched by owner/name only. A Fulcio
/// certificate authenticates numeric ids for the *source* repository
/// alone; nothing in it identifies the signer repository beyond the SAN
/// URI's owner/name. A signer repository pinned with
/// [`RepositoryIdentity::with_owner_id`] or
/// [`RepositoryIdentity::with_repository_id`] is therefore rejected at
/// [`GithubPolicyBuilder::build`] and [`crate::VerifierBuilder::build`]
/// rather than accepted and ignored, which would hand the caller a
/// rename/transfer protection they do not have.
#[derive(Debug, Clone)]
pub struct SignerPolicy {
    /// The repository the signing workflow lives in.
    pub repository: RepositoryIdentity,
    /// The workflow file path within that repository.
    pub path: WorkflowPath,
    /// The required revision of the workflow file itself.
    pub revision: WorkflowRevisionPolicy,
}

/// A GitHub repository identity: owner/name, optionally pinned to numeric
/// IDs.
///
/// Numeric owner id protects against owner rename/recreation; numeric
/// repository id protects against repository rename, transfer, or
/// recreation. Both are enforced for [`SourcePolicy`] only, where the
/// certificate carries the matching authenticated claims; pinning either
/// on a [`SignerPolicy`] is rejected as unenforceable when the policy is
/// built. Fields are private and validated at construction: an empty
/// owner or name is rejected, so any `RepositoryIdentity` you can hold is
/// well-formed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepositoryIdentity {
    owner: String,
    name: String,
    owner_id: Option<u64>,
    repository_id: Option<u64>,
}

impl RepositoryIdentity {
    /// Builds a [`RepositoryIdentity`] from separate owner and name
    /// strings.
    ///
    /// # Errors
    ///
    /// Returns [`PolicyError::InvalidConfiguration`] if `owner` or `name`
    /// is empty.
    pub fn new(owner: impl Into<String>, name: impl Into<String>) -> Result<Self, Error> {
        let owner = owner.into();
        let name = name.into();
        if owner.is_empty() {
            return Err(policy_error(
                "repository owner must not be empty".to_owned(),
            ));
        }
        if name.is_empty() {
            return Err(policy_error("repository name must not be empty".to_owned()));
        }
        Ok(RepositoryIdentity {
            owner,
            name,
            owner_id: None,
            repository_id: None,
        })
    }

    /// Builds a [`RepositoryIdentity`] by splitting `"owner/name"`.
    ///
    /// # Errors
    ///
    /// Returns [`PolicyError::InvalidConfiguration`] if `owner_slash_name`
    /// does not contain exactly one `/`, or either side is empty.
    pub fn parse(owner_slash_name: &str) -> Result<Self, Error> {
        let Some((owner, name)) = owner_slash_name.split_once('/') else {
            return Err(policy_error(format!(
                "expected \"owner/name\", got {owner_slash_name:?}"
            )));
        };
        if name.contains('/') {
            return Err(policy_error(format!(
                "expected exactly one '/' in \"owner/name\", got {owner_slash_name:?}"
            )));
        }
        Self::new(owner, name)
    }

    /// Pins the numeric owner id (protects against owner
    /// rename/recreation).
    ///
    /// Enforced for a [`SourcePolicy`] repository only; a
    /// [`SignerPolicy`] repository carrying this pin is rejected when the
    /// policy is built.
    #[must_use]
    pub fn with_owner_id(mut self, owner_id: u64) -> Self {
        self.owner_id = Some(owner_id);
        self
    }

    /// Pins the numeric repository id (protects against repository
    /// rename/transfer/recreation).
    ///
    /// Enforced for a [`SourcePolicy`] repository only; a
    /// [`SignerPolicy`] repository carrying this pin is rejected when the
    /// policy is built.
    #[must_use]
    pub fn with_repository_id(mut self, repository_id: u64) -> Self {
        self.repository_id = Some(repository_id);
        self
    }

    /// The repository owner (user or organization login).
    #[must_use]
    pub fn owner(&self) -> &str {
        &self.owner
    }

    /// The repository name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The pinned numeric owner id, if set.
    #[must_use]
    pub fn owner_id(&self) -> Option<u64> {
        self.owner_id
    }

    /// The pinned numeric repository id, if set.
    #[must_use]
    pub fn repository_id(&self) -> Option<u64> {
        self.repository_id
    }
}

/// How a git ref must be matched.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RefPolicy {
    /// The ref must match exactly, e.g. `"refs/tags/v0.4.0"`.
    Exact(String),
    /// The ref must match a glob pattern. Documented as the weaker form:
    /// prefer [`RefPolicy::Exact`] for release/updater callers.
    Glob(String),
}

/// A workflow file path, e.g. `".github/workflows/release.yml"`.
///
/// Private inner field, validated non-empty at construction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkflowPath(String);

impl WorkflowPath {
    /// Builds a [`WorkflowPath`].
    ///
    /// # Errors
    ///
    /// Returns [`PolicyError::InvalidConfiguration`] if `path` is empty.
    pub fn new(path: impl Into<String>) -> Result<Self, Error> {
        let path = path.into();
        if path.is_empty() {
            return Err(policy_error("workflow path must not be empty".to_owned()));
        }
        Ok(WorkflowPath(path))
    }

    /// The workflow path string.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// A git commit SHA-1, as 20 raw bytes.
///
/// Private inner field, validated as exactly 40 hex characters at
/// construction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommitSha([u8; 20]);

impl CommitSha {
    /// Parses a 40-hex-character commit SHA.
    ///
    /// # Errors
    ///
    /// Returns [`PolicyError::InvalidConfiguration`] if `sha` is not
    /// exactly 40 hexadecimal characters.
    pub fn new(sha: &str) -> Result<Self, Error> {
        let bytes: [u8; 20] = parse_util::strict_hex("commit_sha", sha)
            .map_err(|e| policy_error(format!("invalid commit sha: {e}")))?;
        Ok(CommitSha(bytes))
    }

    /// The commit SHA as canonical lowercase hex.
    #[must_use]
    pub fn to_hex(&self) -> String {
        hex::encode(self.0)
    }
}

/// How the signing workflow file's own revision must be matched.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkflowRevisionPolicy {
    /// Any revision of the workflow file is accepted.
    Any,
    /// The workflow file must be at a specific ref.
    Ref(String),
    /// The workflow file must be at a specific commit.
    Sha(CommitSha),
}

#[cfg(test)]
mod tests {
    use super::{
        CheckpointOriginPolicy, GithubPolicy, RepositoryIdentity, SignerPolicy, SourcePolicy,
        WorkflowPath,
    };
    use super::{RefPolicy, WorkflowRevisionPolicy};
    use crate::error::{Error, PolicyError};
    use crate::trust::{PublicKey, TransparencyLog, ValidityPeriod};

    fn synthetic_log() -> TransparencyLog {
        TransparencyLog {
            base_url: "https://example.test".to_owned(),
            hash_algorithm: "SHA2_256".to_owned(),
            public_key: PublicKey {
                raw_bytes: vec![1, 2, 3],
                key_details: "PKIX_ECDSA_P256_SHA_256".to_owned(),
                valid_for: ValidityPeriod {
                    start: 0,
                    end: None,
                },
            },
            log_id_key_id: vec![],
            checkpoint_key_id: None,
        }
    }

    #[test]
    fn repository_identity_parses_owner_slash_name() -> Result<(), Box<dyn std::error::Error>> {
        let repo = RepositoryIdentity::parse("combinatrix-ai/dlgt")?;
        if repo.owner() != "combinatrix-ai" || repo.name() != "dlgt" {
            return Err("owner/name split mismatch".into());
        }
        Ok(())
    }

    #[test]
    fn repository_identity_with_ids_round_trips() -> Result<(), Box<dyn std::error::Error>> {
        let repo = RepositoryIdentity::new("combinatrix-ai", "dlgt")?
            .with_owner_id(1)
            .with_repository_id(2);
        if repo.owner_id() != Some(1) || repo.repository_id() != Some(2) {
            return Err("numeric id round trip mismatch".into());
        }
        Ok(())
    }

    #[test]
    fn repository_identity_rejects_empty_owner() -> Result<(), Box<dyn std::error::Error>> {
        expect_policy_error(RepositoryIdentity::new("", "dlgt"))
    }

    #[test]
    fn repository_identity_rejects_empty_name() -> Result<(), Box<dyn std::error::Error>> {
        expect_policy_error(RepositoryIdentity::new("combinatrix-ai", ""))
    }

    #[test]
    fn repository_identity_parse_rejects_missing_slash() -> Result<(), Box<dyn std::error::Error>> {
        expect_policy_error(RepositoryIdentity::parse("no-slash-here"))
    }

    #[test]
    fn repository_identity_parse_rejects_extra_slash() -> Result<(), Box<dyn std::error::Error>> {
        expect_policy_error(RepositoryIdentity::parse("owner/name/extra"))
    }

    #[test]
    fn repository_identity_parse_rejects_empty_owner() -> Result<(), Box<dyn std::error::Error>> {
        expect_policy_error(RepositoryIdentity::parse("/name"))
    }

    #[test]
    fn workflow_path_rejects_empty() -> Result<(), Box<dyn std::error::Error>> {
        expect_policy_error(WorkflowPath::new(""))
    }

    #[test]
    fn commit_sha_rejects_wrong_length() -> Result<(), Box<dyn std::error::Error>> {
        expect_policy_error(super::CommitSha::new(&"a".repeat(39)))
    }

    #[test]
    fn commit_sha_round_trips_hex() -> Result<(), Box<dyn std::error::Error>> {
        let sha_hex = "a".repeat(40);
        let sha = super::CommitSha::new(&sha_hex)?;
        if sha.to_hex() != sha_hex {
            return Err("commit sha hex round trip mismatch".into());
        }
        Ok(())
    }

    #[test]
    fn github_policy_builder_requires_both_halves() -> Result<(), Box<dyn std::error::Error>> {
        expect_policy_error(GithubPolicy::builder().build())
    }

    #[test]
    fn github_policy_builder_builds_with_both_halves() -> Result<(), Box<dyn std::error::Error>> {
        let source = SourcePolicy {
            repository: RepositoryIdentity::parse("combinatrix-ai/dlgt")?
                .with_owner_id(1)
                .with_repository_id(2),
            git_ref: RefPolicy::Exact("refs/tags/v0.4.0".to_owned()),
            commit: None,
        };
        let signer = SignerPolicy {
            repository: RepositoryIdentity::parse("combinatrix-ai/dlgt")?,
            path: WorkflowPath::new(".github/workflows/release.yml")?,
            revision: WorkflowRevisionPolicy::Any,
        };
        GithubPolicy::builder()
            .source(source)
            .signer(signer)
            .build()?;
        Ok(())
    }

    #[test]
    fn github_policy_builder_rejects_signer_owner_id_pin() -> Result<(), Box<dyn std::error::Error>>
    {
        expect_policy_error(
            GithubPolicy::builder()
                .source(unpinned_source()?)
                .signer(SignerPolicy {
                    repository: RepositoryIdentity::parse("combinatrix-ai/dlgt")?.with_owner_id(1),
                    path: WorkflowPath::new(".github/workflows/release.yml")?,
                    revision: WorkflowRevisionPolicy::Any,
                })
                .build(),
        )
    }

    #[test]
    fn github_policy_builder_rejects_signer_repository_id_pin()
    -> Result<(), Box<dyn std::error::Error>> {
        expect_policy_error(
            GithubPolicy::builder()
                .source(unpinned_source()?)
                .signer(SignerPolicy {
                    repository: RepositoryIdentity::parse("combinatrix-ai/dlgt")?
                        .with_repository_id(2),
                    path: WorkflowPath::new(".github/workflows/release.yml")?,
                    revision: WorkflowRevisionPolicy::Any,
                })
                .build(),
        )
    }

    fn unpinned_source() -> Result<SourcePolicy, Error> {
        Ok(SourcePolicy {
            repository: RepositoryIdentity::parse("combinatrix-ai/dlgt")?,
            git_ref: RefPolicy::Exact("refs/tags/v0.4.0".to_owned()),
            commit: None,
        })
    }

    #[test]
    fn checkpoint_origin_policy_requires_a_binding() -> Result<(), Box<dyn std::error::Error>> {
        expect_policy_error(CheckpointOriginPolicy::builder().build())
    }

    #[test]
    fn checkpoint_origin_policy_rejects_empty_origin() -> Result<(), Box<dyn std::error::Error>> {
        let log = synthetic_log();
        expect_policy_error(CheckpointOriginPolicy::builder().allow_origin(&log, ""))
    }

    #[test]
    fn checkpoint_origin_policy_rejects_cr_lf_origin() -> Result<(), Box<dyn std::error::Error>> {
        let log = synthetic_log();
        expect_policy_error(CheckpointOriginPolicy::builder().allow_origin(&log, "rekor\nattacker"))
    }

    fn expect_policy_error<T: std::fmt::Debug>(
        result: Result<T, Error>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        match result {
            Err(Error::Policy(PolicyError::InvalidConfiguration(_))) => Ok(()),
            other => {
                Err(format!("expected PolicyError::InvalidConfiguration, got {other:?}").into())
            }
        }
    }
}
