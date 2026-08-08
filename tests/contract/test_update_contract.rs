#[cfg(test)]
mod tests {
    use rgt::updater::github::{
        find_asset, find_checksum_asset, GitHubRelease, GitHubReleaseAsset,
    };
    use rgt::updater::platform::{Arch, Os, Platform};

    #[test]
    fn test_release_asset_matching() {
        let release = GitHubRelease {
            tag_name: "v0.2.0".to_string(),
            assets: vec![
                GitHubReleaseAsset {
                    name: "rgt-v0.2.0-aarch64-apple-darwin.tar.gz".to_string(),
                    browser_download_url: "https://example.com/mac-arm.tar.gz".to_string(),
                },
                GitHubReleaseAsset {
                    name: "rgt-v0.2.0-x86_64-unknown-linux-musl.tar.gz".to_string(),
                    browser_download_url: "https://example.com/linux-x64.tar.gz".to_string(),
                },
                GitHubReleaseAsset {
                    name: "rgt-v0.2.0-x86_64-pc-windows-msvc.zip".to_string(),
                    browser_download_url: "https://example.com/win-x64.zip".to_string(),
                },
                GitHubReleaseAsset {
                    name: "checksums.txt".to_string(),
                    browser_download_url: "https://example.com/checksums.txt".to_string(),
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
                },
                GitHubReleaseAsset {
                    name: "rgt-v0.1.0-x86_64-linux.tar.gz".to_string(),
                    browser_download_url: "https://example.com/dl".to_string(),
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
            }],
        };

        assert!(find_checksum_asset(&release).is_none());
    }
}
