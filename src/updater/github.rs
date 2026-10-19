use std::time::Duration;

use crate::updater::checksum;
use crate::updater::platform::Platform;
use serde::Deserialize;

const GITHUB_API_RELEASES_LATEST: &str =
    "https://api.github.com/repos/rafael-bianchi/rgt/releases/latest";
const GITHUB_API_RELEASES_TAG: &str =
    "https://api.github.com/repos/rafael-bianchi/rgt/releases/tags";

#[derive(Debug, Clone, Deserialize)]
pub struct GitHubReleaseAsset {
    pub name: String,
    pub browser_download_url: String,
    /// API asset URL (e.g. `https://api.github.com/repos/OWNER/REPO/releases/assets/{id}`).
    /// Required for authenticated downloads from private repositories.
    pub url: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GitHubRelease {
    pub tag_name: String,
    pub assets: Vec<GitHubReleaseAsset>,
}

fn build_agent() -> ureq::Agent {
    ureq::AgentBuilder::new()
        .timeout_read(Duration::from_secs(30))
        .timeout_write(Duration::from_secs(30))
        .build()
}

/// Fetches the latest GitHub release for the rgt repository.
///
/// Accepts an optional `GITHUB_TOKEN` to avoid API rate limits (429).
/// Returns a structured `GitHubRelease` or a human-readable error string.
pub fn fetch_latest_release(token: Option<&str>) -> Result<GitHubRelease, String> {
    fetch_release_from_url(GITHUB_API_RELEASES_LATEST, token)
}

use std::io::Read;

const MAX_JSON_BYTES: u64 = 16 * 1024 * 1024;
const MAX_ASSET_BYTES: u64 = 64 * 1024 * 1024;

/// Fetches a GitHub release by exact tag name (e.g., `v0.2.0` or `0.2.0`).
pub fn fetch_release_by_tag(tag: &str, token: Option<&str>) -> Result<GitHubRelease, String> {
    let clean = tag.trim_start_matches('v');
    if crate::updater::version::parse_version(clean).is_err() {
        return Err(format!("Release not found at invalid tag {}", tag));
    }
    let normalized = if tag.starts_with('v') {
        tag.to_string()
    } else {
        format!("v{tag}")
    };
    let url = format!("{}/{}", GITHUB_API_RELEASES_TAG, normalized);
    fetch_release_from_url(&url, token)
}

fn fetch_release_from_url(url: &str, token: Option<&str>) -> Result<GitHubRelease, String> {
    let agent = build_agent();
    let mut req = agent
        .get(url)
        .set("Accept", "application/vnd.github+json")
        .set("User-Agent", "rgt-updater/1.0");

    if let Some(t) = token {
        req = req.set("Authorization", &format!("Bearer {}", t));
    }

    let response = req.call().map_err(|e| {
        let err_str = e.to_string();
        if err_str.contains("status 429") {
            "GitHub API rate limit exceeded. Set GITHUB_TOKEN or wait before retrying.".to_string()
        } else if err_str.contains("404") {
            format!("Release not found at {}", url)
        } else {
            format!("Failed to fetch release: {}", err_str)
        }
    })?;

    let mut reader = response.into_reader().take(MAX_JSON_BYTES + 1);
    let mut body = String::new();
    reader
        .read_to_string(&mut body)
        .map_err(|e| format!("Read error: {}", e))?;
    if body.len() > MAX_JSON_BYTES as usize {
        return Err("release response exceeded 16 MiB limit".to_string());
    }

    serde_json::from_str::<GitHubRelease>(&body)
        .map_err(|e| format!("Failed to parse release JSON: {}", e))
}

/// Finds the platform-specific binary asset for a release.
///
/// Matches assets by the platform's expected target triple naming convention
/// (e.g., `rgt-v0.2.0-aarch64-apple-darwin.tar.gz`). Returns `None` if no asset
/// matches the platform.
pub fn find_asset<'a>(
    release: &'a GitHubRelease,
    platform: &Platform,
) -> Option<&'a GitHubReleaseAsset> {
    let expected_name = platform.asset_name(&release.tag_name);
    release.assets.iter().find(|a| a.name == expected_name)
}

/// Finds the `checksums.txt` asset in a release's asset list.
pub fn find_checksum_asset(release: &GitHubRelease) -> Option<&GitHubReleaseAsset> {
    release.assets.iter().find(|a| a.name == "checksums.txt")
}

/// Downloads a file from the given URL to the destination path.
///
/// When a token is supplied and an API asset URL is available, the download uses the
/// API endpoint (with `Accept: application/octet-stream`), which works for private
/// repositories. Otherwise it falls back to the browser download URL.
pub fn download_asset(
    url: &str,
    api_url: Option<&str>,
    dest: &std::path::Path,
    token: Option<&str>,
) -> Result<(), String> {
    let agent = build_agent();
    let req = if let (Some(t), Some(api)) = (token, api_url) {
        agent
            .get(api)
            .set("User-Agent", "rgt-updater/1.0")
            .set("Accept", "application/octet-stream")
            .set("Authorization", &format!("Bearer {}", t))
    } else {
        // FR-014: Omit Authorization header for browser_download_url
        // to avoid credential leakage on cross-origin redirects and AWS S3 rejections
        agent.get(url).set("User-Agent", "rgt-updater/1.0")
    };

    let response = req.call().map_err(|e| format!("Download failed: {}", e))?;

    let mut reader = response.into_reader().take(MAX_ASSET_BYTES + 1);
    let mut file = std::fs::File::create(dest).map_err(|e| format!("Cannot create file: {}", e))?;
    let copied =
        std::io::copy(&mut reader, &mut file).map_err(|e| format!("Write error: {}", e))?;
    if copied > MAX_ASSET_BYTES {
        let _ = std::fs::remove_file(dest);
        return Err("download exceeded maximum asset size (64 MiB)".to_string());
    }

    Ok(())
}

/// Verifies a downloaded asset against the release's `checksums.txt`.
///
/// Fetches the checksum file, finds the entry matching `asset_name`, and verifies
/// the SHA-256 of `asset_path`. Returns `Ok(true)` on match, `Ok(false)` on mismatch,
/// or `Err` on fetch/parse errors.
pub fn verify_asset_checksum(
    asset_path: &std::path::Path,
    asset_name: &str,
    checksum_url: &str,
    checksum_api_url: Option<&str>,
    token: Option<&str>,
) -> Result<bool, String> {
    let agent = build_agent();
    let req = if let (Some(t), Some(api)) = (token, checksum_api_url) {
        agent
            .get(api)
            .set("User-Agent", "rgt-updater/1.0")
            .set("Accept", "application/octet-stream")
            .set("Authorization", &format!("Bearer {}", t))
    } else {
        // FR-014: Omit Authorization header for browser_download_url
        agent.get(checksum_url).set("User-Agent", "rgt-updater/1.0")
    };

    let response = req
        .call()
        .map_err(|e| format!("Checksum fetch failed: {}", e))?;

    let mut reader = response.into_reader().take(MAX_JSON_BYTES + 1);
    let mut body = String::new();
    reader
        .read_to_string(&mut body)
        .map_err(|e| format!("Read error: {}", e))?;
    if body.len() > MAX_JSON_BYTES as usize {
        return Err("checksum response exceeded 16 MiB limit".to_string());
    }

    let entries = checksum::parse_checksums(&body);
    let entry = checksum::find_checksum(&entries, asset_name)
        .ok_or_else(|| format!("No checksum entry for {}", asset_name))?;

    checksum::verify_checksum(asset_path, &entry.hash)
        .map_err(|e| format!("Checksum verification error: {}", e))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_asset_matches_by_name() {
        let release = GitHubRelease {
            tag_name: "v0.1.0".to_string(),
            assets: vec![
                GitHubReleaseAsset {
                    name: "rgt-v0.1.0-aarch64-apple-darwin.tar.gz".to_string(),
                    browser_download_url: "https://example.com/dl1".to_string(),
                    url: None,
                },
                GitHubReleaseAsset {
                    name: "rgt-v0.1.0-x86_64-unknown-linux-gnu.tar.gz".to_string(),
                    browser_download_url: "https://example.com/dl2".to_string(),
                    url: None,
                },
            ],
        };

        let platform = Platform {
            os: crate::updater::platform::Os::Darwin,
            arch: crate::updater::platform::Arch::Aarch64,
            target_triple: "aarch64-apple-darwin".to_string(),
        };

        let asset = find_asset(&release, &platform).unwrap();
        assert_eq!(asset.browser_download_url, "https://example.com/dl1");
    }

    #[test]
    fn test_find_asset_returns_none_for_missing_platform() {
        let release = GitHubRelease {
            tag_name: "v0.1.0".to_string(),
            assets: vec![GitHubReleaseAsset {
                name: "rgt-v0.1.0-aarch64-apple-darwin.tar.gz".to_string(),
                browser_download_url: "https://example.com/dl1".to_string(),
                url: None,
            }],
        };

        let platform = Platform {
            os: crate::updater::platform::Os::Linux,
            arch: crate::updater::platform::Arch::X86_64,
            target_triple: "x86_64-unknown-linux-gnu".to_string(),
        };

        assert!(find_asset(&release, &platform).is_none());
    }

    #[test]
    fn test_find_checksum_asset() {
        let release = GitHubRelease {
            tag_name: "v0.1.0".to_string(),
            assets: vec![
                GitHubReleaseAsset {
                    name: "checksums.txt".to_string(),
                    browser_download_url: "https://example.com/checksums.txt".to_string(),
                    url: None,
                },
                GitHubReleaseAsset {
                    name: "rgt-v0.1.0-x86_64-linux.tar.gz".to_string(),
                    browser_download_url: "https://example.com/dl".to_string(),
                    url: None,
                },
            ],
        };

        let checksum = find_checksum_asset(&release).unwrap();
        assert_eq!(checksum.name, "checksums.txt");
    }
}
