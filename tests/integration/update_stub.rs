use rgt::cli::update::UpdateIo;
use rgt::updater::github::{GitHubRelease, GitHubReleaseAsset};
use std::cell::Cell;
use std::path::Path;

/// Check-only update tests must never reach the download/install path.
pub struct CheckOnlyUpdateIo {
    release: GitHubRelease,
    fetches: Cell<usize>,
}

impl CheckOnlyUpdateIo {
    pub fn new(tag_name: String) -> Self {
        Self {
            release: GitHubRelease {
                tag_name,
                assets: Vec::new(),
            },
            fetches: Cell::new(0),
        }
    }

    pub fn fetches(&self) -> usize {
        self.fetches.get()
    }
}

impl UpdateIo for CheckOnlyUpdateIo {
    fn fetch_release(&self, tag: Option<&str>) -> Result<GitHubRelease, String> {
        assert!(
            tag.is_none(),
            "check-only test should fetch the latest release"
        );
        self.fetches.set(self.fetches.get() + 1);
        Ok(self.release.clone())
    }

    fn download_asset(
        &self,
        _asset: &GitHubReleaseAsset,
        _dest: &Path,
        _token: Option<&str>,
    ) -> Result<(), String> {
        panic!("check-only update must not download assets")
    }

    fn verify_checksum(
        &self,
        _asset_path: &Path,
        _asset_name: &str,
        _checksum: &GitHubReleaseAsset,
        _token: Option<&str>,
    ) -> Result<bool, String> {
        panic!("check-only update must not verify an asset")
    }

    fn verify_attestation(&self, _archive_path: &Path, _token: Option<&str>) -> Result<(), String> {
        panic!("check-only update must not verify an attestation")
    }
}
