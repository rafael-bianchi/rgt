//! Matches authenticated [`FulcioClaims`] against a caller's
//! [`GithubPolicy`] (DESIGN.md "Identity policy").
//!
//! This is step 10 of [`crate::Verifier::verify_digest`]'s chain, run only
//! after the certificate chain, DSSE signature, and SCT have all already
//! verified — so every claim read here is authenticated, not
//! attacker-controlled statement content. Every failure is a distinct,
//! mechanical [`PolicyError`] variant (DESIGN.md "Core decisions" item
//! 8): this module never infers intent, it only reports which expected
//! value did not match which found value.
//!
//! `claims.runner_environment` is extracted by [`crate::fulcio`] but
//! deliberately not matched here: DESIGN.md's `GithubPolicy` sketch has
//! no runner-requirement field, so v0.1 policy matching covers source
//! identity, signer identity, and revision only. A `RunnerPolicy` is left
//! for a future task if GitHub-hosted-only enforcement is ever wanted.

use crate::error::{Error, PolicyError};
use crate::fulcio::FulcioClaims;
use crate::policy::{GithubPolicy, RefPolicy, RepositoryIdentity, WorkflowRevisionPolicy};

/// The GitHub Actions OIDC issuer every workflow-identity certificate is
/// issued under. Hard-pinned (DESIGN.md "Identity policy": "issuer pinned
/// internally ... not configurable") — never read from a policy.
const EXPECTED_ISSUER: &str = "https://token.actions.githubusercontent.com";

/// The prefix every GitHub repository/workflow identity URI this crate
/// models starts with.
const GITHUB_URI_PREFIX: &str = "https://github.com/";

/// The required prefix of a signer's workflow path (DESIGN.md "Identity
/// policy" / Fulcio's own certificate profile: workflow identities are
/// always a `.github/workflows/` file).
const WORKFLOW_PATH_PREFIX: &str = ".github/workflows/";

/// The verified identity strings [`crate::verifier::VerificationReport`]
/// needs, once [`match_policy`] has confirmed the certificate's claims
/// satisfy the caller's policy.
///
/// Shaped like [`crate::verifier::VerifiedCertificateIdentity`] (the
/// report's certificate-derived-facts field, the only place these strings
/// are used) but kept as its own crate-private type so this module never
/// has to depend on `verifier`'s public API.
#[derive(Debug)]
pub(crate) struct MatchedIdentity {
    /// The authenticated OIDC issuer (always [`EXPECTED_ISSUER`] by the
    /// time this is constructed).
    pub(crate) issuer: String,
    /// The authenticated source repository, `"owner/name"`, in the case
    /// the certificate itself carried (not normalized to the policy's
    /// case).
    pub(crate) source_repository: String,
    /// The authenticated source ref.
    pub(crate) source_ref: String,
    /// The authenticated signer-workflow repository, `"owner/name"`.
    pub(crate) signer_repository: String,
    /// The authenticated signer-workflow file path.
    pub(crate) signer_workflow_path: String,
}

/// Matches `claims` (already authenticated by the X.509 chain, DSSE
/// signature, and SCT checks) against `policy`.
///
/// Checks run in this order, each surfacing its own failure immediately:
/// issuer; source repository (owner/name, then owner id, then repository
/// id, if pinned); source ref; source commit (if pinned); signer
/// repository; signer workflow path; signer revision.
///
/// # Errors
///
/// Returns [`PolicyError::MissingIdentityClaim`] if a Fulcio extension a
/// check needs is absent from the certificate,
/// [`PolicyError::MalformedIdentityClaim`] if a present claim is not in
/// the shape this crate requires (the source repository URI or the SAN
/// URI does not parse as `https://github.com/{owner}/{name}[...]`), and
/// otherwise the specific `PolicyError::*Mismatch` variant for whichever
/// check first disagrees with `policy`.
pub(crate) fn match_policy(
    claims: &FulcioClaims,
    policy: &GithubPolicy,
) -> Result<MatchedIdentity, Error> {
    let issuer = require_claim(claims.issuer.as_deref(), "issuer")?;
    if issuer != EXPECTED_ISSUER {
        return Err(Error::Policy(PolicyError::IssuerMismatch {
            expected: EXPECTED_ISSUER.to_owned(),
            found: issuer.to_owned(),
        }));
    }

    let source_uri = require_claim(
        claims.source_repository_uri.as_deref(),
        "source_repository_uri",
    )?;
    let (source_owner, source_name) = parse_github_repo_uri(source_uri, "source_repository_uri")?;
    check_repository_match(
        source_owner,
        source_name,
        &policy.source.repository,
        |expected, found| PolicyError::SourceRepositoryMismatch { expected, found },
    )?;
    check_pinned_id(
        policy.source.repository.owner_id(),
        claims.source_repository_owner_id.as_deref(),
        "source_repository_owner_id",
        |expected, found| PolicyError::SourceOwnerIdMismatch { expected, found },
    )?;
    check_pinned_id(
        policy.source.repository.repository_id(),
        claims.source_repository_id.as_deref(),
        "source_repository_id",
        |expected, found| PolicyError::SourceRepositoryIdMismatch { expected, found },
    )?;

    let source_ref = require_claim(
        claims.source_repository_ref.as_deref(),
        "source_repository_ref",
    )?;
    check_ref_policy(source_ref, &policy.source.git_ref)?;

    if let Some(expected_commit) = &policy.source.commit {
        let found = require_claim(
            claims.source_repository_digest.as_deref(),
            "source_repository_digest",
        )?;
        let expected = expected_commit.to_hex();
        if !found.eq_ignore_ascii_case(&expected) {
            return Err(Error::Policy(PolicyError::SourceCommitMismatch {
                expected,
                found: found.to_owned(),
            }));
        }
    }

    let san_uri = require_claim(claims.san_uri.as_deref(), "san_uri")?;
    let signer = parse_signer_san_uri(san_uri)?;
    check_repository_match(
        signer.owner,
        signer.name,
        &policy.signer.repository,
        |expected, found| PolicyError::SignerRepositoryMismatch { expected, found },
    )?;
    if signer.workflow_path != policy.signer.path.as_str() {
        return Err(Error::Policy(PolicyError::SignerWorkflowPathMismatch {
            expected: policy.signer.path.as_str().to_owned(),
            found: signer.workflow_path.to_owned(),
        }));
    }
    check_signer_revision(claims, &signer, &policy.signer.revision)?;

    Ok(MatchedIdentity {
        issuer: issuer.to_owned(),
        source_repository: format!("{source_owner}/{source_name}"),
        source_ref: source_ref.to_owned(),
        signer_repository: format!("{}/{}", signer.owner, signer.name),
        signer_workflow_path: signer.workflow_path.to_owned(),
    })
}

/// Requires `claim` to be present, naming `name` in the error if not.
fn require_claim<'a>(claim: Option<&'a str>, name: &'static str) -> Result<&'a str, Error> {
    claim.ok_or(Error::Policy(PolicyError::MissingIdentityClaim {
        claim: name,
    }))
}

fn malformed_claim(claim: &'static str, reason: impl Into<String>) -> Error {
    Error::Policy(PolicyError::MalformedIdentityClaim {
        claim,
        reason: reason.into(),
    })
}

/// Parses a bare GitHub repository URI, `https://github.com/{owner}/{name}`
/// exactly (no further path segments, no trailing slash).
fn parse_github_repo_uri<'a>(
    uri: &'a str,
    claim: &'static str,
) -> Result<(&'a str, &'a str), Error> {
    let rest = uri
        .strip_prefix(GITHUB_URI_PREFIX)
        .ok_or_else(|| malformed_claim(claim, format!("missing \"{GITHUB_URI_PREFIX}\" prefix")))?;
    let (owner, name) = rest
        .split_once('/')
        .ok_or_else(|| malformed_claim(claim, "missing '/' between owner and repository name"))?;
    if owner.is_empty() || name.is_empty() {
        return Err(malformed_claim(claim, "empty owner or repository name"));
    }
    if name.contains('/') {
        return Err(malformed_claim(
            claim,
            "unexpected extra path segment after repository name",
        ));
    }
    Ok((owner, name))
}

/// The parsed pieces of a signer-workflow SAN URI,
/// `https://github.com/{owner}/{name}/{workflow_path}@{ref}`.
#[derive(Debug)]
struct SignerUriParts<'a> {
    owner: &'a str,
    name: &'a str,
    /// Always starts with [`WORKFLOW_PATH_PREFIX`].
    workflow_path: &'a str,
    git_ref: &'a str,
}

/// Parses a signer-workflow SAN URI: `owner`/`name` are the first two
/// `/`-separated path segments after the `https://github.com/` prefix
/// (repository owners and names never themselves contain `/`). Exactly
/// one `@` must separate `{workflow_path}` from `{ref}`. Additional `@`
/// characters are rejected because, without an escaping rule, it is
/// impossible to determine whether they belong to the workflow path or
/// the ref. Accepting such an ambiguous identity could make one workflow
/// path appear to be another during policy matching.
fn parse_signer_san_uri(uri: &str) -> Result<SignerUriParts<'_>, Error> {
    const CLAIM: &str = "san_uri";

    let rest = uri
        .strip_prefix(GITHUB_URI_PREFIX)
        .ok_or_else(|| malformed_claim(CLAIM, format!("missing \"{GITHUB_URI_PREFIX}\" prefix")))?;
    let (repo_and_path, git_ref) = rest
        .split_once('@')
        .ok_or_else(|| malformed_claim(CLAIM, "missing '@' revision separator"))?;
    if git_ref.contains('@') {
        return Err(malformed_claim(
            CLAIM,
            "ambiguous additional '@' in workflow identity",
        ));
    }
    if git_ref.is_empty() {
        return Err(malformed_claim(CLAIM, "empty revision after '@'"));
    }

    let mut parts = repo_and_path.splitn(3, '/');
    let owner = parts
        .next()
        .filter(|s| !s.is_empty())
        .ok_or_else(|| malformed_claim(CLAIM, "missing repository owner"))?;
    let name = parts
        .next()
        .filter(|s| !s.is_empty())
        .ok_or_else(|| malformed_claim(CLAIM, "missing repository name"))?;
    let workflow_path = parts
        .next()
        .ok_or_else(|| malformed_claim(CLAIM, "missing workflow path"))?;
    if !workflow_path.starts_with(WORKFLOW_PATH_PREFIX) {
        return Err(malformed_claim(
            CLAIM,
            format!("workflow path does not start with \"{WORKFLOW_PATH_PREFIX}\""),
        ));
    }

    Ok(SignerUriParts {
        owner,
        name,
        workflow_path,
        git_ref,
    })
}

/// Compares an authenticated `owner`/`name` pair against `expected`,
/// ASCII-case-insensitively (DESIGN.md "Identity policy": GitHub logins
/// and repository names are not case-sensitive identity).
fn check_repository_match(
    owner: &str,
    name: &str,
    expected: &RepositoryIdentity,
    make_err: impl FnOnce(String, String) -> PolicyError,
) -> Result<(), Error> {
    if owner.eq_ignore_ascii_case(expected.owner()) && name.eq_ignore_ascii_case(expected.name()) {
        Ok(())
    } else {
        Err(Error::Policy(make_err(
            format!("{}/{}", expected.owner(), expected.name()),
            format!("{owner}/{name}"),
        )))
    }
}

/// Checks an optional pinned numeric id (owner id or repository id)
/// against the decimal-string claim, only when `expected` is `Some` — a
/// policy that never pinned an id has nothing to check.
fn check_pinned_id(
    expected: Option<u64>,
    found_claim: Option<&str>,
    claim_name: &'static str,
    make_err: impl FnOnce(String, String) -> PolicyError,
) -> Result<(), Error> {
    let Some(expected_id) = expected else {
        return Ok(());
    };
    let found = require_claim(found_claim, claim_name)?;
    let expected = expected_id.to_string();
    if found == expected {
        Ok(())
    } else {
        Err(Error::Policy(make_err(expected, found.to_owned())))
    }
}

/// Checks an authenticated source ref against the policy's ref
/// requirement: byte-exact for [`RefPolicy::Exact`], [`glob_match`] for
/// [`RefPolicy::Glob`].
fn check_ref_policy(found_ref: &str, policy: &RefPolicy) -> Result<(), Error> {
    let (matched, expected) = match policy {
        RefPolicy::Exact(expected) => (found_ref == expected, expected.clone()),
        RefPolicy::Glob(pattern) => (glob_match(pattern, found_ref), pattern.clone()),
    };
    if matched {
        Ok(())
    } else {
        Err(Error::Policy(PolicyError::SourceRefMismatch {
            expected,
            found: found_ref.to_owned(),
        }))
    }
}

/// Checks the signer workflow's own revision against the policy's
/// [`WorkflowRevisionPolicy`].
///
/// `Sha` compares against `claims.build_signer_digest` (Fulcio OID `.10`,
/// "Build Signer Digest"), not `build_config_digest` (OID `.19`, "Build
/// Config Digest"): the two are populated identically on every fixture
/// this crate has (a top-level, non-reusable workflow signed directly),
/// so fixture data alone cannot distinguish them, but they answer
/// different questions in general. The build *signer* is the workflow
/// that actually holds the signing identity — the same identity the SAN
/// URI itself carries ([`SignerUriParts`] is parsed from the SAN, and
/// `build_signer_uri` is identical to the SAN URI on every observed
/// fixture). The build *config* is whichever specific (potentially
/// reusable, potentially different-repository) workflow file was
/// invoked to produce the build. [`crate::policy::SignerPolicy`] exists
/// specifically to pin "which workflow signed" independently of reusable
/// -workflow indirection (DESIGN.md "Core decisions" item 6), so its
/// `Sha` revision must bind to the signer identity, not the build config.
fn check_signer_revision(
    claims: &FulcioClaims,
    signer: &SignerUriParts<'_>,
    policy: &WorkflowRevisionPolicy,
) -> Result<(), Error> {
    match policy {
        WorkflowRevisionPolicy::Any => Ok(()),
        WorkflowRevisionPolicy::Ref(expected_ref) => {
            if signer.git_ref == expected_ref.as_str() {
                Ok(())
            } else {
                Err(Error::Policy(PolicyError::SignerRevisionMismatch {
                    expected: expected_ref.clone(),
                    found: signer.git_ref.to_owned(),
                }))
            }
        }
        WorkflowRevisionPolicy::Sha(expected_sha) => {
            let found =
                require_claim(claims.build_signer_digest.as_deref(), "build_signer_digest")?;
            let expected = expected_sha.to_hex();
            if found.eq_ignore_ascii_case(&expected) {
                Ok(())
            } else {
                Err(Error::Policy(PolicyError::SignerRevisionMismatch {
                    expected,
                    found: found.to_owned(),
                }))
            }
        }
    }
}

/// Matches `text` against `pattern`, where `*` in `pattern` matches any
/// sequence of characters (including none, including `/`) and every other
/// character must match literally, byte-for-byte.
///
/// The standard greedy-with-backtracking two-pointer algorithm (as used
/// for shell-style wildcard matching restricted to a single wildcard
/// character): linear extra space, and — unlike a naive recursive
/// backtracker — polynomial (not exponential) time even on adversarial
/// inputs like many repeated `*`s, since at most one "last star" position
/// is ever retried at a time.
fn glob_match(pattern: &str, text: &str) -> bool {
    let p = pattern.as_bytes();
    let t = text.as_bytes();
    let (mut pi, mut ti) = (0usize, 0usize);
    // The most recent `*`'s position in `p`, and how much of `t` it had
    // consumed the last time it was tried.
    let mut star: Option<(usize, usize)> = None;

    while ti < t.len() {
        if pi < p.len() && p[pi] == b'*' {
            star = Some((pi, ti));
            pi += 1;
        } else if pi < p.len() && p[pi] == t[ti] {
            pi += 1;
            ti += 1;
        } else if let Some((star_pi, star_ti)) = star {
            // Backtrack: the last `*` consumes one more character of `t`.
            pi = star_pi + 1;
            ti = star_ti + 1;
            star = Some((star_pi, ti));
        } else {
            return false;
        }
    }
    while p.get(pi) == Some(&b'*') {
        pi += 1;
    }
    pi == p.len()
}

#[cfg(test)]
mod tests {
    use super::{
        EXPECTED_ISSUER, SignerUriParts, check_ref_policy, glob_match, malformed_claim,
        match_policy, parse_github_repo_uri, parse_signer_san_uri,
    };
    use crate::error::{Error, PolicyError};
    use crate::fulcio::FulcioClaims;
    use crate::policy::{
        CommitSha, GithubPolicy, RefPolicy, RepositoryIdentity, SignerPolicy, SourcePolicy,
        WorkflowPath, WorkflowRevisionPolicy,
    };

    // -------------------------------------------------------------
    // glob_match
    // -------------------------------------------------------------

    #[test]
    fn glob_star_alone_matches_anything() -> Result<(), Box<dyn std::error::Error>> {
        for text in ["", "a", "refs/heads/trunk", "a/b/c"] {
            if !glob_match("*", text) {
                return Err(format!("expected \"*\" to match {text:?}").into());
            }
        }
        Ok(())
    }

    #[test]
    fn glob_prefix_star_matches_suffix_including_slashes() -> Result<(), Box<dyn std::error::Error>>
    {
        if !glob_match("refs/tags/*", "refs/tags/v0.4.0") {
            return Err("expected refs/tags/* to match refs/tags/v0.4.0".into());
        }
        if !glob_match("refs/tags/*", "refs/tags/nested/v0.4.0") {
            return Err("expected * to match a sequence containing '/'".into());
        }
        if glob_match("refs/tags/*", "refs/heads/trunk") {
            return Err("expected refs/tags/* to reject refs/heads/trunk".into());
        }
        Ok(())
    }

    #[test]
    fn glob_star_in_the_middle_requires_trailing_literal() -> Result<(), Box<dyn std::error::Error>>
    {
        if !glob_match("refs/*/v1", "refs/heads/v1") {
            return Err("expected refs/*/v1 to match refs/heads/v1".into());
        }
        if !glob_match("refs/*/v1", "refs/heads/extra/v1") {
            return Err(
                "expected * to absorb multiple segments before the trailing literal".into(),
            );
        }
        if glob_match("refs/*/v1", "refs/heads/v2") {
            return Err("expected refs/*/v1 to reject a non-matching trailing literal".into());
        }
        if glob_match("refs/*/v1", "refs/v1") {
            return Err(
                "expected refs/*/v1 to require a '/' before the trailing literal even when \
                 '*' matches zero characters"
                    .into(),
            );
        }
        Ok(())
    }

    #[test]
    fn glob_exact_pattern_with_no_star_requires_exact_match()
    -> Result<(), Box<dyn std::error::Error>> {
        if !glob_match("refs/heads/trunk", "refs/heads/trunk") {
            return Err("expected identical strings to match".into());
        }
        if glob_match("refs/heads/trunk", "refs/heads/trunk2") {
            return Err("expected a literal pattern to reject a longer string".into());
        }
        Ok(())
    }

    #[test]
    fn glob_rejects_when_no_star_can_absorb_the_difference()
    -> Result<(), Box<dyn std::error::Error>> {
        if glob_match("refs/tags/*", "refs/heads/v1") {
            return Err("expected a mismatched literal prefix to reject".into());
        }
        Ok(())
    }

    /// Adversarial repeated-star pattern that is the classic worst case for
    /// a naive recursive backtracker (exponential blowup trying every
    /// split point). The iterative two-pointer algorithm here must still
    /// return promptly and correctly reject a text with no trailing `b`.
    #[test]
    fn glob_adversarial_repeated_stars_do_not_blow_up() -> Result<(), Box<dyn std::error::Error>> {
        let pattern = "*a*a*a*a*a*a*a*a*a*a*a*a*a*a*a*a*a*a*a*b";
        let all_as = "a".repeat(40);
        if glob_match(pattern, &all_as) {
            return Err(
                "expected pattern requiring a trailing 'b' to reject an all-'a' text".into(),
            );
        }
        let all_as_then_b = format!("{all_as}b");
        if !glob_match(pattern, &all_as_then_b) {
            return Err("expected the same pattern to match once a trailing 'b' is present".into());
        }
        Ok(())
    }

    #[test]
    fn glob_empty_pattern_only_matches_empty_text() -> Result<(), Box<dyn std::error::Error>> {
        if !glob_match("", "") {
            return Err("expected empty pattern to match empty text".into());
        }
        if glob_match("", "x") {
            return Err("expected empty pattern to reject non-empty text".into());
        }
        Ok(())
    }

    // -------------------------------------------------------------
    // check_ref_policy
    // -------------------------------------------------------------

    #[test]
    fn ref_policy_exact_matches_byte_for_byte() -> Result<(), Box<dyn std::error::Error>> {
        check_ref_policy(
            "refs/heads/trunk",
            &RefPolicy::Exact("refs/heads/trunk".to_owned()),
        )?;
        Ok(())
    }

    #[test]
    fn ref_policy_exact_rejects_mismatch() -> Result<(), Box<dyn std::error::Error>> {
        match check_ref_policy(
            "refs/heads/trunk",
            &RefPolicy::Exact("refs/tags/v2.96.0".to_owned()),
        ) {
            Err(Error::Policy(PolicyError::SourceRefMismatch { .. })) => Ok(()),
            other => Err(format!("expected SourceRefMismatch, got {other:?}").into()),
        }
    }

    #[test]
    fn ref_policy_glob_delegates_to_glob_match() -> Result<(), Box<dyn std::error::Error>> {
        check_ref_policy(
            "refs/heads/trunk",
            &RefPolicy::Glob("refs/heads/*".to_owned()),
        )?;
        match check_ref_policy(
            "refs/heads/trunk",
            &RefPolicy::Glob("refs/tags/*".to_owned()),
        ) {
            Err(Error::Policy(PolicyError::SourceRefMismatch { .. })) => Ok(()),
            other => Err(format!("expected SourceRefMismatch, got {other:?}").into()),
        }
    }

    // -------------------------------------------------------------
    // parse_github_repo_uri
    // -------------------------------------------------------------

    #[test]
    fn parses_bare_repo_uri() -> Result<(), Box<dyn std::error::Error>> {
        let (owner, name) = parse_github_repo_uri("https://github.com/cli/cli", "x")?;
        if owner != "cli" || name != "cli" {
            return Err(format!("unexpected owner/name: {owner}/{name}").into());
        }
        Ok(())
    }

    #[test]
    fn rejects_repo_uri_missing_prefix() -> Result<(), Box<dyn std::error::Error>> {
        expect_malformed(parse_github_repo_uri("http://github.com/cli/cli", "x"))
    }

    #[test]
    fn rejects_repo_uri_with_extra_segment() -> Result<(), Box<dyn std::error::Error>> {
        expect_malformed(parse_github_repo_uri(
            "https://github.com/cli/cli/extra",
            "x",
        ))
    }

    #[test]
    fn rejects_repo_uri_missing_name() -> Result<(), Box<dyn std::error::Error>> {
        expect_malformed(parse_github_repo_uri("https://github.com/cli", "x"))
    }

    // -------------------------------------------------------------
    // parse_signer_san_uri
    // -------------------------------------------------------------

    #[test]
    fn parses_real_shaped_san_uri() -> Result<(), Box<dyn std::error::Error>> {
        let SignerUriParts {
            owner,
            name,
            workflow_path,
            git_ref,
        } = parse_signer_san_uri(
            "https://github.com/cli/cli/.github/workflows/deployment.yml@refs/heads/trunk",
        )?;
        if owner != "cli" || name != "cli" {
            return Err(format!("unexpected owner/name: {owner}/{name}").into());
        }
        if workflow_path != ".github/workflows/deployment.yml" {
            return Err(format!("unexpected workflow_path: {workflow_path}").into());
        }
        if git_ref != "refs/heads/trunk" {
            return Err(format!("unexpected git_ref: {git_ref}").into());
        }
        Ok(())
    }

    #[test]
    fn rejects_san_uri_with_ambiguous_additional_at() -> Result<(), Box<dyn std::error::Error>> {
        expect_malformed(parse_signer_san_uri(
            "https://github.com/cli/cli/.github/workflows/deployment.yml@refs/heads/weird@name",
        ))
    }

    #[test]
    fn rejects_san_uri_missing_workflow_prefix() -> Result<(), Box<dyn std::error::Error>> {
        expect_malformed(parse_signer_san_uri(
            "https://github.com/cli/cli/not-a-workflow-path@refs/heads/trunk",
        ))
    }

    #[test]
    fn rejects_san_uri_missing_revision() -> Result<(), Box<dyn std::error::Error>> {
        expect_malformed(parse_signer_san_uri(
            "https://github.com/cli/cli/.github/workflows/deployment.yml",
        ))
    }

    fn expect_malformed<T: std::fmt::Debug>(
        result: Result<T, Error>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        match result {
            Err(Error::Policy(PolicyError::MalformedIdentityClaim { .. })) => Ok(()),
            other => Err(format!("expected MalformedIdentityClaim, got {other:?}").into()),
        }
    }

    #[test]
    fn malformed_claim_helper_wraps_reason() -> Result<(), Box<dyn std::error::Error>> {
        match malformed_claim("x", "reason") {
            Error::Policy(PolicyError::MalformedIdentityClaim { claim, reason }) => {
                if claim != "x" || reason != "reason" {
                    return Err("unexpected claim/reason".into());
                }
                Ok(())
            }
            other => Err(format!("expected MalformedIdentityClaim, got {other:?}").into()),
        }
    }

    // -------------------------------------------------------------
    // match_policy: the full function, against synthetic (not real-
    // certificate-derived) claims and policies. tests/verify_e2e.rs
    // covers this same function end to end through the real chain and
    // real fixture; these tests exist to cheaply cover branches a real
    // certificate can't easily exercise (missing claims) and revision
    // policies (`Ref`/`Sha`) the e2e suite doesn't vary.
    // -------------------------------------------------------------

    const SIGNER_DIGEST: &str = "b300f2ec7ec9dc9addc39b2ad88c54097ded7ca0";
    const SOURCE_DIGEST: &str = "b300f2ec7ec9dc9addc39b2ad88c54097ded7ca1";

    /// Claims shaped like the real `cli/cli` golden fixture (see
    /// `src/fulcio.rs`'s own tests), so the "correct" case below is
    /// realistic rather than arbitrary.
    fn synthetic_claims() -> FulcioClaims {
        FulcioClaims {
            issuer: Some(EXPECTED_ISSUER.to_owned()),
            san_uri: Some(
                "https://github.com/cli/cli/.github/workflows/deployment.yml@refs/heads/trunk"
                    .to_owned(),
            ),
            build_signer_digest: Some(SIGNER_DIGEST.to_owned()),
            source_repository_uri: Some("https://github.com/cli/cli".to_owned()),
            source_repository_ref: Some("refs/heads/trunk".to_owned()),
            source_repository_id: Some("212613049".to_owned()),
            source_repository_owner_id: Some("59704711".to_owned()),
            source_repository_digest: Some(SOURCE_DIGEST.to_owned()),
            ..FulcioClaims::default()
        }
    }

    fn synthetic_policy(
        revision: WorkflowRevisionPolicy,
    ) -> Result<GithubPolicy, Box<dyn std::error::Error>> {
        let source = SourcePolicy {
            repository: RepositoryIdentity::parse("cli/cli")?
                .with_owner_id(59_704_711)
                .with_repository_id(212_613_049),
            git_ref: RefPolicy::Exact("refs/heads/trunk".to_owned()),
            commit: None,
        };
        let signer = SignerPolicy {
            repository: RepositoryIdentity::parse("cli/cli")?,
            path: WorkflowPath::new(".github/workflows/deployment.yml")?,
            revision,
        };
        Ok(GithubPolicy::builder()
            .source(source)
            .signer(signer)
            .build()?)
    }

    #[test]
    fn match_policy_succeeds_with_synthetic_claims_and_any_revision()
    -> Result<(), Box<dyn std::error::Error>> {
        let matched = match_policy(
            &synthetic_claims(),
            &synthetic_policy(WorkflowRevisionPolicy::Any)?,
        )
        .map_err(|e| format!("expected Ok, got {e:?}"))?;
        if matched.source_repository != "cli/cli" || matched.signer_repository != "cli/cli" {
            return Err("unexpected repository in MatchedIdentity".into());
        }
        Ok(())
    }

    #[test]
    fn match_policy_rejects_ambiguous_workflow_identity_with_any_revision()
    -> Result<(), Box<dyn std::error::Error>> {
        let claims = FulcioClaims {
            san_uri: Some(
                "https://github.com/cli/cli/.github/workflows/deployment.yml@evil.yaml@refs/heads/trunk"
                    .to_owned(),
            ),
            ..synthetic_claims()
        };
        expect_malformed(match_policy(
            &claims,
            &synthetic_policy(WorkflowRevisionPolicy::Any)?,
        ))
    }

    #[test]
    fn match_policy_ref_revision_matches() -> Result<(), Box<dyn std::error::Error>> {
        let policy = synthetic_policy(WorkflowRevisionPolicy::Ref("refs/heads/trunk".to_owned()))?;
        match_policy(&synthetic_claims(), &policy)
            .map_err(|e| format!("expected Ok, got {e:?}"))?;
        Ok(())
    }

    #[test]
    fn match_policy_ref_revision_mismatch_is_rejected() -> Result<(), Box<dyn std::error::Error>> {
        let policy = synthetic_policy(WorkflowRevisionPolicy::Ref("refs/heads/other".to_owned()))?;
        match match_policy(&synthetic_claims(), &policy) {
            Err(Error::Policy(PolicyError::SignerRevisionMismatch { expected, found })) => {
                if expected == "refs/heads/other" && found == "refs/heads/trunk" {
                    Ok(())
                } else {
                    Err(format!("unexpected expected/found: {expected}/{found}").into())
                }
            }
            other => Err(format!("expected Policy(SignerRevisionMismatch), got {other:?}").into()),
        }
    }

    #[test]
    fn match_policy_sha_revision_matches_build_signer_digest()
    -> Result<(), Box<dyn std::error::Error>> {
        let policy = synthetic_policy(WorkflowRevisionPolicy::Sha(CommitSha::new(SIGNER_DIGEST)?))?;
        match_policy(&synthetic_claims(), &policy)
            .map_err(|e| format!("expected Ok, got {e:?}"))?;
        Ok(())
    }

    #[test]
    fn match_policy_sha_revision_mismatch_is_rejected() -> Result<(), Box<dyn std::error::Error>> {
        let policy = synthetic_policy(WorkflowRevisionPolicy::Sha(CommitSha::new(
            &"c".repeat(40),
        )?))?;
        match match_policy(&synthetic_claims(), &policy) {
            Err(Error::Policy(PolicyError::SignerRevisionMismatch { .. })) => Ok(()),
            other => Err(format!("expected Policy(SignerRevisionMismatch), got {other:?}").into()),
        }
    }

    #[test]
    fn match_policy_missing_issuer_claim_is_rejected() -> Result<(), Box<dyn std::error::Error>> {
        let claims = FulcioClaims {
            issuer: None,
            ..synthetic_claims()
        };
        match match_policy(&claims, &synthetic_policy(WorkflowRevisionPolicy::Any)?) {
            Err(Error::Policy(PolicyError::MissingIdentityClaim { claim: "issuer" })) => Ok(()),
            other => {
                Err(format!("expected MissingIdentityClaim(\"issuer\"), got {other:?}").into())
            }
        }
    }

    #[test]
    fn match_policy_signer_repository_mismatch_is_rejected()
    -> Result<(), Box<dyn std::error::Error>> {
        let claims = FulcioClaims {
            san_uri: Some(
                "https://github.com/other-org/other-repo/.github/workflows/deployment.yml\
                 @refs/heads/trunk"
                    .to_owned(),
            ),
            ..synthetic_claims()
        };
        match match_policy(&claims, &synthetic_policy(WorkflowRevisionPolicy::Any)?) {
            Err(Error::Policy(PolicyError::SignerRepositoryMismatch { expected, found })) => {
                if expected == "cli/cli" && found == "other-org/other-repo" {
                    Ok(())
                } else {
                    Err(format!("unexpected expected/found: {expected}/{found}").into())
                }
            }
            other => {
                Err(format!("expected Policy(SignerRepositoryMismatch), got {other:?}").into())
            }
        }
    }

    #[test]
    fn match_policy_repository_id_mismatch_is_rejected() -> Result<(), Box<dyn std::error::Error>> {
        let claims = FulcioClaims {
            source_repository_id: Some("1".to_owned()),
            ..synthetic_claims()
        };
        match match_policy(&claims, &synthetic_policy(WorkflowRevisionPolicy::Any)?) {
            Err(Error::Policy(PolicyError::SourceRepositoryIdMismatch { expected, found })) => {
                if expected == "212613049" && found == "1" {
                    Ok(())
                } else {
                    Err(format!("unexpected expected/found: {expected}/{found}").into())
                }
            }
            other => {
                Err(format!("expected Policy(SourceRepositoryIdMismatch), got {other:?}").into())
            }
        }
    }

    #[test]
    fn match_policy_source_commit_matches_when_pinned() -> Result<(), Box<dyn std::error::Error>> {
        let mut policy = synthetic_policy(WorkflowRevisionPolicy::Any)?;
        policy.source.commit = Some(CommitSha::new(SOURCE_DIGEST)?);
        match_policy(&synthetic_claims(), &policy)
            .map_err(|e| format!("expected Ok, got {e:?}"))?;
        Ok(())
    }

    #[test]
    fn match_policy_source_commit_mismatch_is_rejected() -> Result<(), Box<dyn std::error::Error>> {
        let mut policy = synthetic_policy(WorkflowRevisionPolicy::Any)?;
        policy.source.commit = Some(CommitSha::new(&"d".repeat(40))?);
        match match_policy(&synthetic_claims(), &policy) {
            Err(Error::Policy(PolicyError::SourceCommitMismatch { .. })) => Ok(()),
            other => Err(format!("expected Policy(SourceCommitMismatch), got {other:?}").into()),
        }
    }
}
