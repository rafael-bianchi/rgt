use std::env;

#[derive(Debug, Clone, PartialEq)]
pub enum Os {
    Darwin,
    Linux,
    Windows,
    Unknown(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Arch {
    X86_64,
    Aarch64,
    Unknown(String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Platform {
    pub os: Os,
    pub arch: Arch,
    pub target_triple: String,
}

impl Platform {
    pub fn detect() -> Self {
        let os = Self::detect_os();
        let arch = Self::detect_arch();
        let target_triple = Self::build_target_triple(&os, &arch);
        Platform {
            os,
            arch,
            target_triple,
        }
    }

    fn detect_os() -> Os {
        match env::consts::OS {
            "macos" => Os::Darwin,
            "linux" => Os::Linux,
            "windows" => Os::Windows,
            other => Os::Unknown(other.to_string()),
        }
    }

    fn detect_arch() -> Arch {
        match env::consts::ARCH {
            "x86_64" | "x86" => Arch::X86_64,
            "aarch64" | "arm64" => Arch::Aarch64,
            other => Arch::Unknown(other.to_string()),
        }
    }

    fn build_target_triple(os: &Os, arch: &Arch) -> String {
        match (os, arch) {
            (Os::Darwin, Arch::X86_64) => "x86_64-apple-darwin".to_string(),
            (Os::Darwin, Arch::Aarch64) => "aarch64-apple-darwin".to_string(),
            (Os::Linux, Arch::X86_64) => "x86_64-unknown-linux-musl".to_string(),
            (Os::Linux, Arch::Aarch64) => "aarch64-unknown-linux-gnu".to_string(),
            (Os::Windows, Arch::X86_64) => "x86_64-pc-windows-msvc".to_string(),
            _ => format!(
                "{}-{}",
                match os {
                    Os::Darwin => "apple-darwin",
                    Os::Linux => "unknown-linux-musl",
                    Os::Windows => "pc-windows-msvc",
                    Os::Unknown(s) => s.as_str(),
                },
                match arch {
                    Arch::X86_64 => "x86_64",
                    Arch::Aarch64 => "aarch64",
                    Arch::Unknown(s) => s.as_str(),
                }
            ),
        }
    }

    pub fn asset_name(&self, version: &str) -> String {
        let ext = match self.os {
            Os::Windows => "zip",
            _ => "tar.gz",
        };
        format!(
            "rgt-v{}-{}.{}",
            version.trim_start_matches('v'),
            self.target_triple,
            ext
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_returns_valid_triple() {
        let platform = Platform::detect();
        assert!(!platform.target_triple.is_empty());
    }

    #[test]
    fn test_build_target_triple_darwin_x86_64() {
        let triple = Platform::build_target_triple(&Os::Darwin, &Arch::X86_64);
        assert_eq!(triple, "x86_64-apple-darwin");
    }

    #[test]
    fn test_build_target_triple_darwin_aarch64() {
        let triple = Platform::build_target_triple(&Os::Darwin, &Arch::Aarch64);
        assert_eq!(triple, "aarch64-apple-darwin");
    }

    #[test]
    fn test_build_target_triple_linux_x86_64() {
        let triple = Platform::build_target_triple(&Os::Linux, &Arch::X86_64);
        assert_eq!(triple, "x86_64-unknown-linux-musl");
    }

    #[test]
    fn test_build_target_triple_linux_aarch64() {
        let triple = Platform::build_target_triple(&Os::Linux, &Arch::Aarch64);
        assert_eq!(triple, "aarch64-unknown-linux-gnu");
    }

    #[test]
    fn test_build_target_triple_windows_x86_64() {
        let triple = Platform::build_target_triple(&Os::Windows, &Arch::X86_64);
        assert_eq!(triple, "x86_64-pc-windows-msvc");
    }

    #[test]
    fn test_asset_name_darwin() {
        let platform = Platform {
            os: Os::Darwin,
            arch: Arch::Aarch64,
            target_triple: "aarch64-apple-darwin".to_string(),
        };
        assert_eq!(
            platform.asset_name("0.1.0"),
            "rgt-v0.1.0-aarch64-apple-darwin.tar.gz"
        );
    }

    #[test]
    fn test_asset_name_windows() {
        let platform = Platform {
            os: Os::Windows,
            arch: Arch::X86_64,
            target_triple: "x86_64-pc-windows-msvc".to_string(),
        };
        assert_eq!(
            platform.asset_name("0.1.0"),
            "rgt-v0.1.0-x86_64-pc-windows-msvc.zip"
        );
    }

    #[test]
    fn test_asset_name_strips_leading_v() {
        let platform = Platform {
            os: Os::Linux,
            arch: Arch::X86_64,
            target_triple: "x86_64-unknown-linux-musl".to_string(),
        };
        assert_eq!(
            platform.asset_name("v0.1.0"),
            "rgt-v0.1.0-x86_64-unknown-linux-musl.tar.gz"
        );
    }
}
