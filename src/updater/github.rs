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
    pub size: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GitHubRelease {
    pub tag_name: String,
    pub name: Option<String>,
    pub draft: bool,
    pub prerelease: bool,
    pub published_at: Option<String>,
    pub assets: Vec<GitHubReleaseAsset>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct UpdateCheckResult {
    pub current_version: String,
    pub latest_version: String,
    pub is_update_available: bool,
    pub download_url: Option<String>,
    pub checksum_url: Option<String>,
}

fn build_agent() -> ureq::Agent {
    ureq::AgentBuilder::new()
        .timeout_read(Duration::from_secs(30))
        .timeout_write(Duration::from_secs(30))
        .build()
}

pub fn fetch_latest_release(token: Option<&str>) -> Result<GitHubRelease, String> {
    fetch_release_from_url(GITHUB_API_RELEASES_LATEST, token)
}

pub fn fetch_release_by_tag(tag: &str, token: Option<&str>) -> Result<GitHubRelease, String> {
    let url = format!("{}/{}", GITHUB_API_RELEASES_TAG, tag);
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
        } else if err_str.contains("status 404") {
            format!("Release not found at {}", url)
        } else {
            format!("Failed to fetch release: {}", err_str)
        }
    })?;

    let body = response
        .into_string()
        .map_err(|e| format!("Read error: {}", e))?;

    serde_json::from_str::<GitHubRelease>(&body)
        .map_err(|e| format!("Failed to parse release JSON: {}", e))
}

pub fn find_asset<'a>(
    release: &'a GitHubRelease,
    platform: &Platform,
) -> Option<&'a GitHubReleaseAsset> {
    let expected_name = platform.asset_name(&release.tag_name);
    release.assets.iter().find(|a| a.name == expected_name)
}

pub fn find_checksum_asset(release: &GitHubRelease) -> Option<&GitHubReleaseAsset> {
    release
        .assets
        .iter()
        .find(|a| a.name == "checksums.txt")
}

pub fn check_update(
    current_version: &str,
    token: Option<&str>,
) -> Result<UpdateCheckResult, String> {
    let release = fetch_latest_release(token)?;
    let platform = Platform::detect();

    let latest = release.tag_name.clone();
    let current = current_version.to_string();
    let is_update_available = latest != current;

    Ok(UpdateCheckResult {
        current_version: current,
        latest_version: latest,
        is_update_available,
        download_url: find_asset(&release, &platform)
            .map(|a| a.browser_download_url.clone()),
        checksum_url: find_checksum_asset(&release)
            .map(|a| a.browser_download_url.clone()),
    })
}

pub fn download_asset(url: &str, dest: &std::path::Path) -> Result<(), String> {
    let agent = build_agent();
    let response = agent
        .get(url)
        .set("User-Agent", "rgt-updater/1.0")
        .call()
        .map_err(|e| format!("Download failed: {}", e))?;

    let mut reader = response.into_reader();
    let mut file =
        std::fs::File::create(dest).map_err(|e| format!("Cannot create file: {}", e))?;
    std::io::copy(&mut reader, &mut file).map_err(|e| format!("Write error: {}", e))?;

    Ok(())
}

pub fn verify_asset_checksum(
    asset_path: &std::path::Path,
    asset_name: &str,
    checksum_url: &str,
) -> Result<bool, String> {
    let agent = build_agent();
    let response = agent
        .get(checksum_url)
        .set("User-Agent", "rgt-updater/1.0")
        .call()
        .map_err(|e| format!("Checksum fetch failed: {}", e))?;

    let body = response
        .into_string()
        .map_err(|e| format!("Read error: {}", e))?;

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
            name: Some("Release v0.1.0".to_string()),
            draft: false,
            prerelease: false,
            published_at: Some("2026-01-01T00:00:00Z".to_string()),
            assets: vec![
                GitHubReleaseAsset {
                    name: "rgt-v0.1.0-aarch64-apple-darwin.tar.gz".to_string(),
                    browser_download_url: "https://example.com/dl1".to_string(),
                    size: 5000000,
                },
                GitHubReleaseAsset {
                    name: "rgt-v0.1.0-x86_64-unknown-linux-gnu.tar.gz".to_string(),
                    browser_download_url: "https://example.com/dl2".to_string(),
                    size: 6000000,
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
            name: None,
            draft: false,
            prerelease: false,
            published_at: None,
            assets: vec![GitHubReleaseAsset {
                name: "rgt-v0.1.0-aarch64-apple-darwin.tar.gz".to_string(),
                browser_download_url: "https://example.com/dl1".to_string(),
                size: 5000000,
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
            name: None,
            draft: false,
            prerelease: false,
            published_at: None,
            assets: vec![
                GitHubReleaseAsset {
                    name: "checksums.txt".to_string(),
                    browser_download_url: "https://example.com/checksums.txt".to_string(),
                    size: 1024,
                },
                GitHubReleaseAsset {
                    name: "rgt-v0.1.0-x86_64-linux.tar.gz".to_string(),
                    browser_download_url: "https://example.com/dl".to_string(),
                    size: 5000000,
                },
            ],
        };

        let checksum = find_checksum_asset(&release).unwrap();
        assert_eq!(checksum.name, "checksums.txt");
    }
}
