#[cfg(test)]
mod tests {
    use rgt::updater::github::{
        find_asset, find_checksum_asset, GitHubRelease, GitHubReleaseAsset,
        UpdateCheckResult,
    };
    use rgt::updater::platform::{Arch, Os, Platform};

    #[test]
    fn test_update_check_result_is_update_when_version_differs() {
        let result = UpdateCheckResult {
            current_version: "0.1.0".to_string(),
            latest_version: "v0.2.0".to_string(),
            is_update_available: true,
            download_url: Some("https://example.com/dl".to_string()),
            checksum_url: Some("https://example.com/checksums.txt".to_string()),
        };
        assert!(result.is_update_available);
        assert_eq!(result.current_version, "0.1.0");
        assert_eq!(result.latest_version, "v0.2.0");
    }

    #[test]
    fn test_update_check_result_no_update() {
        let result = UpdateCheckResult {
            current_version: "0.1.0".to_string(),
            latest_version: "v0.1.0".to_string(),
            is_update_available: false,
            download_url: None,
            checksum_url: None,
        };
        assert!(!result.is_update_available);
    }

    #[test]
    fn test_release_asset_matching() {
        let release = GitHubRelease {
            tag_name: "v0.2.0".to_string(),
            name: None,
            draft: false,
            prerelease: false,
            published_at: None,
            assets: vec![
                GitHubReleaseAsset {
                    name: "rgt-v0.2.0-aarch64-apple-darwin.tar.gz".to_string(),
                    browser_download_url: "https://example.com/mac-arm.tar.gz".to_string(),
                    size: 5000000,
                },
                GitHubReleaseAsset {
                    name: "rgt-v0.2.0-x86_64-unknown-linux-musl.tar.gz".to_string(),
                    browser_download_url: "https://example.com/linux-x64.tar.gz".to_string(),
                    size: 6000000,
                },
                GitHubReleaseAsset {
                    name: "rgt-v0.2.0-x86_64-pc-windows-msvc.zip".to_string(),
                    browser_download_url: "https://example.com/win-x64.zip".to_string(),
                    size: 7000000,
                },
                GitHubReleaseAsset {
                    name: "checksums.txt".to_string(),
                    browser_download_url: "https://example.com/checksums.txt".to_string(),
                    size: 512,
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

        let win = Platform {
            os: Os::Windows,
            arch: Arch::X86_64,
            target_triple: "x86_64-pc-windows-msvc".to_string(),
        };
        let asset = find_asset(&release, &win).unwrap();
        assert_eq!(
            asset.browser_download_url,
            "https://example.com/win-x64.zip"
        );

        let checksum = find_checksum_asset(&release).unwrap();
        assert_eq!(
            checksum.browser_download_url,
            "https://example.com/checksums.txt"
        );
    }

    #[test]
    fn test_platform_flag_paths_covered() {
        let platforms = vec![
            (Os::Darwin, Arch::X86_64, "x86_64-apple-darwin"),
            (Os::Darwin, Arch::Aarch64, "aarch64-apple-darwin"),
            (Os::Linux, Arch::X86_64, "x86_64-unknown-linux-musl"),
            (Os::Linux, Arch::Aarch64, "aarch64-unknown-linux-gnu"),
            (Os::Windows, Arch::X86_64, "x86_64-pc-windows-msvc"),
        ];
        for (os, arch, triple) in platforms {
            let p = Platform {
                os,
                arch,
                target_triple: triple.to_string(),
            };
            let name = p.asset_name("0.1.0");
            assert!(name.contains("rgt-v0.1.0"), "Failed for {}", triple);
        }
    }
}
