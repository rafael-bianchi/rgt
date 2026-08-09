use crate::updater::{atomic::AtomicReplace, github, platform::Platform};
use std::env;
use std::path::PathBuf;

/// Executes `rgt update`: checks GitHub Releases for a newer version and performs
/// an atomic in-place binary upgrade (or just checks with `--check`).
pub fn execute_update(check_only: bool, yes: bool, version: Option<String>) -> Result<(), String> {
    let current_version = env!("CARGO_PKG_VERSION");
    let token = env::var("GITHUB_TOKEN").ok();

    let release = if let Some(ref tag) = version {
        github::fetch_release_by_tag(tag, token.as_deref())?
    } else {
        github::fetch_latest_release(token.as_deref())?
    };

    let latest_tag = release.tag_name.clone();
    let latest_version = latest_tag.trim_start_matches('v');

    if check_only {
        if latest_version != current_version {
            println!(
                "A new version of rgt is available: {} (current: v{})",
                latest_tag, current_version
            );
            return Ok(());
        } else {
            println!("rgt is up to date (v{})", current_version);
            return Ok(());
        }
    }

    if latest_version == current_version {
        println!(
            "rgt is already at the latest version (v{})",
            current_version
        );
        return Ok(());
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
            .map_err(|e| format!("Input error: {}", e))?;
        let input = input.trim().to_lowercase();
        if input != "y" && input != "yes" {
            println!("Update cancelled.");
            return Ok(());
        }
    }

    let platform = Platform::detect();
    let asset = github::find_asset(&release, &platform)
        .ok_or_else(|| format!("No binary asset found for {}", platform.target_triple))?;

    let tmp_dir = tempfile::TempDir::new().map_err(|e| format!("Temp dir error: {}", e))?;
    let archive_path = tmp_dir.path().join(&asset.name);

    println!("Downloading {}...", asset.name);
    github::download_asset(&asset.browser_download_url, &archive_path)?;

    if let Some(checksum_asset) = github::find_checksum_asset(&release) {
        println!("Verifying SHA-256 checksum...");
        let valid = github::verify_asset_checksum(
            &archive_path,
            &asset.name,
            &checksum_asset.browser_download_url,
        )?;
        if !valid {
            return Err("Checksum verification FAILED. Aborting update.".to_string());
        }
    }

    let extract_dir = tmp_dir.path().join("extracted");
    std::fs::create_dir_all(&extract_dir).map_err(|e| format!("Create dir error: {}", e))?;

    extract_archive(&archive_path, &extract_dir)?;

    let extracted_binary = find_binary(&extract_dir)?;
    let current_exe =
        env::current_exe().map_err(|e| format!("Cannot locate current binary: {}", e))?;

    println!("Replacing {}...", current_exe.display());
    let ar = AtomicReplace::new(&current_exe);
    ar.replace(&extracted_binary)
        .map_err(|e| format!("Atomic replace error: {}", e))?;

    println!("Successfully updated to {}", latest_tag);
    Ok(())
}

fn extract_archive(archive_path: &std::path::Path, dest: &std::path::Path) -> Result<(), String> {
    let file = std::fs::File::open(archive_path).map_err(|e| format!("Open error: {}", e))?;

    let archive_name = archive_path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy();

    if archive_name.ends_with(".tar.gz") {
        let decoder = flate2::read::GzDecoder::new(file);
        let mut archive = tar::Archive::new(decoder);
        archive
            .unpack(dest)
            .map_err(|e| format!("Tar extract error: {}", e))?;
    } else if archive_name.ends_with(".zip") {
        let mut archive =
            zip::ZipArchive::new(file).map_err(|e| format!("Zip open error: {}", e))?;
        archive
            .extract(dest)
            .map_err(|e| format!("Zip extract error: {}", e))?;
    } else {
        return Err(format!("Unknown archive format: {}", archive_name));
    }

    Ok(())
}

fn find_binary(dir: &std::path::Path) -> Result<PathBuf, String> {
    for entry in walkdir::WalkDir::new(dir) {
        let entry = entry.map_err(|e| format!("Walk error: {}", e))?;
        if entry.file_type().is_file() {
            let name = entry.file_name().to_string_lossy();
            if name == "rgt" || name == "rgt.exe" {
                return Ok(entry.path().to_path_buf());
            }
        }
    }
    Err("rgt binary not found in extracted archive".to_string())
}
