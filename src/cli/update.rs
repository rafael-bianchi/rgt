use crate::updater::{atomic::AtomicReplace, attestation, github, platform::Platform, version};
use std::env;
use std::path::{Path, PathBuf};

/// Update failure carrying the CLI exit code to use (1 = expected error,
/// 2 = invalid input, per the constitution and `contracts/update-behavior.md`).
#[derive(Debug)]
pub struct UpdateError {
    pub message: String,
    pub code: i32,
}

fn uerr(message: impl Into<String>) -> UpdateError {
    UpdateError {
        message: message.into(),
        code: 1,
    }
}

fn invalid_input(message: impl Into<String>) -> UpdateError {
    UpdateError {
        message: message.into(),
        code: 2,
    }
}

/// Injectable I/O seam for the update pipeline.
///
/// `execute_update` uses [`LiveUpdateIo`]; contract tests inject a stub so the
/// refusal/gating behavior is exercised deterministically without network
/// (Constitution IX).
pub trait UpdateIo {
    fn fetch_release(&self, tag: Option<&str>) -> Result<github::GitHubRelease, String>;
    fn download_asset(
        &self,
        asset: &github::GitHubReleaseAsset,
        dest: &Path,
        token: Option<&str>,
    ) -> Result<(), String>;
    fn verify_checksum(
        &self,
        asset_path: &Path,
        asset_name: &str,
        checksum: &github::GitHubReleaseAsset,
        token: Option<&str>,
    ) -> Result<bool, String>;
    fn verify_attestation(&self, archive_path: &Path, token: Option<&str>) -> Result<(), String>;
}

/// The real network-backed implementation.
pub struct LiveUpdateIo;

impl UpdateIo for LiveUpdateIo {
    fn fetch_release(&self, tag: Option<&str>) -> Result<github::GitHubRelease, String> {
        let token = std::env::var("GITHUB_TOKEN").ok();
        match tag {
            Some(t) => github::fetch_release_by_tag(t, token.as_deref()),
            None => github::fetch_latest_release(token.as_deref()),
        }
    }

    fn download_asset(
        &self,
        asset: &github::GitHubReleaseAsset,
        dest: &Path,
        token: Option<&str>,
    ) -> Result<(), String> {
        github::download_asset(
            &asset.browser_download_url,
            asset.url.as_deref(),
            dest,
            token,
        )
    }

    fn verify_checksum(
        &self,
        asset_path: &Path,
        asset_name: &str,
        checksum: &github::GitHubReleaseAsset,
        token: Option<&str>,
    ) -> Result<bool, String> {
        github::verify_asset_checksum(
            asset_path,
            asset_name,
            &checksum.browser_download_url,
            checksum.url.as_deref(),
            token,
        )
    }

    fn verify_attestation(&self, archive_path: &Path, token: Option<&str>) -> Result<(), String> {
        let digest = attestation::file_sha256_hex(archive_path)?;
        let compressed = attestation::fetch_bundle(&format!("sha256:{}", digest), token)?;
        let bundle = attestation::decompress_bundle(&compressed)?;
        let policy = attestation::rgt_policy()?;
        attestation::verify_bundle(&bundle, &digest, &policy)
    }
}

/// Executes `rgt update`: checks GitHub Releases for a newer version and performs
/// an atomic in-place binary upgrade (or just checks with `--check`).
///
/// Secure by default: a release missing `checksums.txt` or a missing/unverifiable
/// attestation is refused with a non-zero exit unless the explicit `--skip-checksum`
/// / `--skip-attestation` opt-outs are passed (FR-001/FR-003).
pub fn execute_update(
    check_only: bool,
    yes: bool,
    version: Option<String>,
    skip_checksum: bool,
    skip_attestation: bool,
) -> Result<(), UpdateError> {
    execute_update_with_io(
        check_only,
        yes,
        version,
        skip_checksum,
        skip_attestation,
        &LiveUpdateIo,
    )
}

/// Same as [`execute_update`], with an injectable I/O seam for tests.
pub fn execute_update_with_io(
    check_only: bool,
    yes: bool,
    version: Option<String>,
    skip_checksum: bool,
    skip_attestation: bool,
    io: &dyn UpdateIo,
) -> Result<(), UpdateError> {
    let current_version = env!("CARGO_PKG_VERSION");

    let release = match io.fetch_release(version.as_deref()) {
        Ok(r) => r,
        Err(e) => {
            // A nonexistent --version tag is invalid input (FR-005 and
            // contracts/update-behavior.md: exit 2), not an expected update
            // error. `github::fetch_release_by_tag` reports 404 as
            // "Release not found ...".
            if version.is_some() && e.contains("Release not found") {
                return Err(invalid_input(e));
            }
            return Err(uerr(e));
        }
    };
    let latest_tag = release.tag_name.clone();
    let latest_version = latest_tag.trim_start_matches('v');

    if check_only {
        if version::latest_is_newer(latest_version, current_version).map_err(uerr)? {
            println!(
                "A new version of rgt is available: {} (current: v{})",
                latest_tag, current_version
            );
        } else {
            println!("rgt is up to date (v{})", current_version);
        }
        return Ok(());
    }

    if version.is_none() {
        // Auto-update proceeds only when the published version is strictly
        // newer (FR-004) — never on an equal or older tag.
        if !version::latest_is_newer(latest_version, current_version).map_err(uerr)? {
            println!(
                "rgt is already at the latest version (v{})",
                current_version
            );
            return Ok(());
        }
    } else {
        // Explicit --version: validate and warn on older tags (FR-005).
        version::parse_version(latest_version).map_err(invalid_input)?;
        let requested = version::parse_version(latest_version).map_err(uerr)?;
        let current = version::parse_version(current_version).map_err(uerr)?;
        if requested < current {
            println!(
                "Warning: requested tag {} is older than the installed version (v{})",
                latest_tag, current_version
            );
        }
    }

    println!(
        "Update available: {} -> {}{}",
        current_version,
        latest_tag,
        if let Some(ref tag_ver) = version {
            format!(" (requested: {})", tag_ver)
        } else {
            String::new()
        }
    );

    if !yes {
        print!("Apply update? [y/N]: ");
        use std::io::Write;
        let _ = std::io::stdout().flush();
        let mut input = String::new();
        std::io::stdin()
            .read_line(&mut input)
            .map_err(|e| uerr(format!("Input error: {}", e)))?;
        let input = input.trim().to_lowercase();
        if input != "y" && input != "yes" {
            println!("Update cancelled.");
            return Ok(());
        }
    }

    let platform = Platform::detect();
    let asset = github::find_asset(&release, &platform).ok_or_else(|| {
        uerr(format!(
            "No binary asset found for {}",
            platform.target_triple
        ))
    })?;

    let tmp_dir = tempfile::TempDir::new().map_err(|e| uerr(format!("Temp dir error: {}", e)))?;
    let archive_path = tmp_dir.path().join(&asset.name);

    println!("Downloading {}...", asset.name);
    io.download_asset(
        asset,
        &archive_path,
        std::env::var("GITHUB_TOKEN").ok().as_deref(),
    )
    .map_err(uerr)?;

    // FR-001/FR-002: checksum verification — missing checksums.txt is a hard
    // error, never a silent skip.
    let checksum_asset = github::find_checksum_asset(&release);
    match checksum_asset {
        Some(checksum) if !skip_checksum => {
            println!("Verifying SHA-256 checksum...");
            let valid = io
                .verify_checksum(
                    &archive_path,
                    &asset.name,
                    checksum,
                    std::env::var("GITHUB_TOKEN").ok().as_deref(),
                )
                .map_err(uerr)?;
            if !valid {
                return Err(uerr("Checksum verification FAILED. Aborting update."));
            }
        }
        None if !skip_checksum => {
            return Err(uerr(
                "release has no checksums.txt asset — refusing to install an unverified binary (use --skip-checksum to override)",
            ));
        }
        _ => {
            if skip_checksum {
                println!("Warning: skipping checksum verification (--skip-checksum, insecure).");
            }
        }
    }

    // FR-003: independent attestation verification.
    if !skip_attestation {
        println!("Verifying GitHub Artifact Attestation...");
        io.verify_attestation(
            &archive_path,
            std::env::var("GITHUB_TOKEN").ok().as_deref(),
        )
        .map_err(|e| uerr(format!("attestation verification failed — refusing to install: {e} (use --skip-attestation to override)")))?;
    } else {
        println!("Warning: skipping attestation verification (--skip-attestation, insecure).");
    }

    let extract_dir = tmp_dir.path().join("extracted");
    std::fs::create_dir_all(&extract_dir).map_err(|e| uerr(format!("Create dir error: {}", e)))?;

    extract_archive(&archive_path, &extract_dir)?;

    let extracted_binary = find_binary(&extract_dir)?;
    let current_exe =
        env::current_exe().map_err(|e| uerr(format!("Cannot locate current binary: {}", e)))?;

    println!("Replacing {}...", current_exe.display());
    let ar = AtomicReplace::new(&current_exe);
    ar.replace(&extracted_binary)
        .map_err(|e| uerr(format!("Atomic replace error: {}", e)))?;

    println!("Successfully updated to {}", latest_tag);
    Ok(())
}

fn extract_archive(
    archive_path: &std::path::Path,
    dest: &std::path::Path,
) -> Result<(), UpdateError> {
    let file = std::fs::File::open(archive_path).map_err(|e| uerr(format!("Open error: {}", e)))?;

    let archive_name = archive_path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy();

    if archive_name.ends_with(".tar.gz") {
        let decoder = flate2::read::GzDecoder::new(file);
        let mut archive = tar::Archive::new(decoder);
        archive
            .unpack(dest)
            .map_err(|e| uerr(format!("Tar extract error: {}", e)))?;
    } else if archive_name.ends_with(".zip") {
        let mut archive =
            zip::ZipArchive::new(file).map_err(|e| uerr(format!("Zip open error: {}", e)))?;
        archive
            .extract(dest)
            .map_err(|e| uerr(format!("Zip extract error: {}", e)))?;
    } else {
        return Err(uerr(format!("Unknown archive format: {}", archive_name)));
    }

    Ok(())
}

fn find_binary(dir: &std::path::Path) -> Result<PathBuf, UpdateError> {
    for entry in walkdir::WalkDir::new(dir) {
        let entry = entry.map_err(|e| uerr(format!("Walk error: {}", e)))?;
        if entry.file_type().is_file() {
            let name = entry.file_name().to_string_lossy();
            if name == "rgt" || name == "rgt.exe" {
                return Ok(entry.path().to_path_buf());
            }
        }
    }
    Err(uerr(
        "rgt binary not found in extracted archive".to_string(),
    ))
}
