#[cfg(test)]
mod tests {
    use rgt::cli::update::{execute_update_with_io, UpdateIo};
    use rgt::updater::github::{
        find_asset, find_checksum_asset, GitHubRelease, GitHubReleaseAsset,
    };
    use rgt::updater::platform::{Arch, Os, Platform};
    use std::path::Path;

    fn stub_release(tag: &str, checksums: bool) -> GitHubRelease {
        let platform = Platform::detect();
        let asset_name = platform.asset_name(tag);
        let mut assets = vec![GitHubReleaseAsset {
            name: asset_name,
            browser_download_url: "https://example.com/archive".to_string(),
            url: None,
        }];
        if checksums {
            assets.push(GitHubReleaseAsset {
                name: "checksums.txt".to_string(),
                browser_download_url: "https://example.com/checksums.txt".to_string(),
                url: None,
            });
        }
        GitHubRelease {
            tag_name: tag.to_string(),
            assets,
        }
    }

    /// A stub I/O seam so the refusal/gating contract is exercised
    /// deterministically with no network (Constitution IX).
    struct StubIo {
        release: GitHubRelease,
        download_ok: bool,
        checksum_result: Option<bool>,
        attestation_result: Result<(), String>,
        fetch_error: Option<String>,
    }

    impl StubIo {
        fn new(release: GitHubRelease) -> Self {
            StubIo {
                release,
                download_ok: true,
                checksum_result: Some(true),
                attestation_result: Ok(()),
                fetch_error: None,
            }
        }
    }

    impl UpdateIo for StubIo {
        fn fetch_release(&self, _tag: Option<&str>) -> Result<GitHubRelease, String> {
            match &self.fetch_error {
                Some(e) => Err(e.clone()),
                None => Ok(self.release.clone()),
            }
        }
        fn download_asset(
            &self,
            _asset: &GitHubReleaseAsset,
            dest: &Path,
            _token: Option<&str>,
        ) -> Result<(), String> {
            if self.download_ok {
                std::fs::write(dest, b"not-an-archive").map_err(|e| e.to_string())?;
                Ok(())
            } else {
                Err("Download failed".to_string())
            }
        }
        fn verify_checksum(
            &self,
            _asset_path: &Path,
            _asset_name: &str,
            _checksum: &GitHubReleaseAsset,
            _token: Option<&str>,
        ) -> Result<bool, String> {
            Ok(self.checksum_result.unwrap_or(true))
        }
        fn verify_attestation(
            &self,
            _archive_path: &Path,
            _token: Option<&str>,
        ) -> Result<(), String> {
            self.attestation_result.clone()
        }
    }

    fn run(
        version: Option<String>,
        skip_checksum: bool,
        skip_attestation: bool,
        io: &dyn UpdateIo,
    ) -> Result<(), rgt::cli::update::UpdateError> {
        execute_update_with_io(false, true, version, skip_checksum, skip_attestation, io)
    }

    // -----------------------------------------------------------------------
    // US1 (T008): refuse unverified installs
    // -----------------------------------------------------------------------

    #[test]
    fn missing_checksums_is_a_hard_refusal() {
        let io = StubIo::new(stub_release("v0.4.1", false));
        let err = run(Some("v0.4.1".to_string()), false, false, &io).unwrap_err();
        assert_eq!(err.code, 1);
        assert!(err.message.contains("checksums.txt"), "{}", err.message);
    }

    #[test]
    fn nonexistent_version_tag_is_invalid_input_exit_2() {
        let io = StubIo {
            fetch_error: Some("Release not found at https://api.github.com/repos/rafael-bianchi/rgt/releases/tags/v9.9.9".to_string()),
            ..StubIo::new(stub_release("v9.9.9", true))
        };
        let err = run(Some("v9.9.9".to_string()), false, false, &io).unwrap_err();
        assert_eq!(err.code, 2, "{}", err.message);
        assert!(err.message.contains("Release not found"), "{}", err.message);
    }

    #[test]
    fn skip_checksum_bypasses_the_checksum_refusal() {
        let io = StubIo::new(stub_release("v0.4.1", false));
        let err = run(Some("v0.4.1".to_string()), true, true, &io).unwrap_err();
        assert!(
            !err.message.contains("checksums.txt"),
            "must get past the checksum refusal: {}",
            err.message
        );
    }

    #[test]
    fn attestation_failure_is_a_hard_refusal_unless_skipped() {
        let io = StubIo {
            attestation_result: Err("bundle rejected".to_string()),
            ..StubIo::new(stub_release("v0.4.1", true))
        };
        let err = run(Some("v0.4.1".to_string()), false, false, &io).unwrap_err();
        assert_eq!(err.code, 1);
        assert!(
            err.message.contains("attestation verification failed"),
            "{}",
            err.message
        );

        let io2 = StubIo {
            attestation_result: Err("bundle rejected".to_string()),
            ..StubIo::new(stub_release("v0.4.1", true))
        };
        let err2 = run(Some("v0.4.1".to_string()), false, true, &io2).unwrap_err();
        assert!(
            !err2.message.contains("attestation verification failed"),
            "must get past the attestation refusal: {}",
            err2.message
        );
    }

    #[test]
    fn checksum_mismatch_aborts_before_install() {
        let io = StubIo {
            checksum_result: Some(false),
            ..StubIo::new(stub_release("v0.4.1", true))
        };
        let err = run(Some("v0.4.1".to_string()), false, false, &io).unwrap_err();
        assert_eq!(err.code, 1);
        assert!(
            err.message.contains("Checksum verification FAILED"),
            "{}",
            err.message
        );
        assert!(!err.message.contains("replace"), "replace must not run");
    }

    // -----------------------------------------------------------------------
    // US2 (T012): no silent downgrade, semver gating, --version handling
    // -----------------------------------------------------------------------

    #[test]
    fn older_or_equal_tag_is_up_to_date_not_an_update() {
        // current = CARGO_PKG_VERSION (0.4.0); an equal tag is up to date.
        let io = StubIo::new(stub_release("v0.4.0", true));
        let res = run(None, false, false, &io);
        assert!(
            res.is_ok(),
            "equal version must be up-to-date: {:?}",
            res.err()
        );

        // A genuinely older tag is never an update (the string-vs-semver
        // divergence itself is covered in tests/unit/test_update_semver.rs).
        let io2 = StubIo::new(stub_release("v0.3.9", true));
        let res2 = run(None, false, false, &io2);
        assert!(
            res2.is_ok(),
            "older tag must be up-to-date: {:?}",
            res2.err()
        );
    }

    #[test]
    fn genuine_newer_tag_proceeds_past_gating() {
        let io = StubIo {
            download_ok: false,
            ..StubIo::new(stub_release("v0.4.1", true))
        };
        let err = run(None, false, false, &io).unwrap_err();
        assert_eq!(err.message, "Download failed", "must proceed to download");
    }

    #[test]
    fn invalid_version_tag_is_invalid_input_exit_2() {
        let io = StubIo::new(stub_release("v1.x", true));
        let err = run(Some("v1.x".to_string()), false, false, &io).unwrap_err();
        assert_eq!(err.code, 2, "{}", err.message);
        assert!(
            err.message.contains("invalid semantic version"),
            "{}",
            err.message
        );
    }

    #[test]
    fn explicit_older_version_proceeds_with_warning_not_refusal() {
        let io = StubIo {
            download_ok: false,
            ..StubIo::new(stub_release("v0.3.0", true))
        };
        let err = run(Some("v0.3.0".to_string()), false, false, &io).unwrap_err();
        assert_eq!(
            err.message, "Download failed",
            "explicit older tag proceeds to download"
        );
    }

    // -----------------------------------------------------------------------
    // US3 (T015c): verification-before-replace ordering
    // -----------------------------------------------------------------------

    #[test]
    fn failed_verification_never_reaches_replace() {
        // Checksum mismatch: the pipeline must stop before replace.
        let io = StubIo {
            checksum_result: Some(false),
            ..StubIo::new(stub_release("v0.4.1", true))
        };
        let err = run(None, false, false, &io).unwrap_err();
        assert!(err.message.contains("Checksum verification FAILED"));
        assert!(!err.message.contains("replace"));

        // Attestation failure: same ordering guarantee.
        let io2 = StubIo {
            attestation_result: Err("rejected".to_string()),
            ..StubIo::new(stub_release("v0.4.1", true))
        };
        let err2 = run(None, false, false, &io2).unwrap_err();
        assert!(err2.message.contains("attestation verification failed"));
        assert!(!err2.message.contains("replace"));
    }

    // -----------------------------------------------------------------------
    // Existing asset/checksum matching tests (unchanged)
    // -----------------------------------------------------------------------

    #[test]
    fn test_release_asset_matching() {
        let release = GitHubRelease {
            tag_name: "v0.2.0".to_string(),
            assets: vec![
                GitHubReleaseAsset {
                    name: "rgt-v0.2.0-aarch64-apple-darwin.tar.gz".to_string(),
                    browser_download_url: "https://example.com/mac-arm.tar.gz".to_string(),
                    url: None,
                },
                GitHubReleaseAsset {
                    name: "rgt-v0.2.0-x86_64-unknown-linux-musl.tar.gz".to_string(),
                    browser_download_url: "https://example.com/linux-x64.tar.gz".to_string(),
                    url: None,
                },
                GitHubReleaseAsset {
                    name: "rgt-v0.2.0-x86_64-pc-windows-msvc.zip".to_string(),
                    browser_download_url: "https://example.com/win-x64.zip".to_string(),
                    url: None,
                },
                GitHubReleaseAsset {
                    name: "checksums.txt".to_string(),
                    browser_download_url: "https://example.com/checksums.txt".to_string(),
                    url: None,
                },
            ],
        };

        let mac_arm = Platform {
            os: Os::Darwin,
            arch: Arch::Aarch64,
            target_triple: "aarch64-apple-darwin".to_string(),
        };
        let asset = find_asset(&release, &mac_arm).unwrap();
        assert_eq!(
            asset.browser_download_url,
            "https://example.com/mac-arm.tar.gz"
        );

        let linux_x64 = Platform {
            os: Os::Linux,
            arch: Arch::X86_64,
            target_triple: "x86_64-unknown-linux-musl".to_string(),
        };
        let asset = find_asset(&release, &linux_x64).unwrap();
        assert_eq!(
            asset.browser_download_url,
            "https://example.com/linux-x64.tar.gz"
        );

        let win_x64 = Platform {
            os: Os::Windows,
            arch: Arch::X86_64,
            target_triple: "x86_64-pc-windows-msvc".to_string(),
        };
        let asset = find_asset(&release, &win_x64).unwrap();
        assert_eq!(
            asset.browser_download_url,
            "https://example.com/win-x64.zip"
        );

        let unsupported = Platform {
            os: Os::Darwin,
            arch: Arch::Unknown("armv7".to_string()),
            target_triple: "arm-unknown-linux-gnueabihf".to_string(),
        };
        assert!(find_asset(&release, &unsupported).is_none());
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
        assert_eq!(
            checksum.browser_download_url,
            "https://example.com/checksums.txt"
        );
    }

    #[test]
    fn test_no_checksum_returns_error() {
        let release = GitHubRelease {
            tag_name: "v0.1.0".to_string(),
            assets: vec![GitHubReleaseAsset {
                name: "rgt-v0.1.0-x86_64-linux.tar.gz".to_string(),
                browser_download_url: "https://example.com/dl".to_string(),
                url: None,
            }],
        };

        assert!(find_checksum_asset(&release).is_none());
    }
}
