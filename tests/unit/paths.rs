use rgt::hooks::paths::{copilot_cli_config_dir_for, copilot_user_settings, vscode_user_settings};
use std::path::PathBuf;

#[test]
fn copilot_settings_macos() {
    let config_dir = PathBuf::from("/Users/test/Library/Application Support");
    let path = copilot_user_settings(&config_dir);
    assert_eq!(
        path,
        PathBuf::from("/Users/test/Library/Application Support/Code/User/settings.json")
    );
}

#[test]
fn copilot_settings_linux() {
    let config_dir = PathBuf::from("/home/test/.config");
    let path = copilot_user_settings(&config_dir);
    assert_eq!(
        path,
        PathBuf::from("/home/test/.config/Code/User/settings.json")
    );
}

#[test]
fn copilot_settings_windows_appdata() {
    // dirs::config_dir() on Windows resolves to %APPDATA% (AppData/Roaming).
    let config_dir = PathBuf::from(r"C:\Users\test\AppData\Roaming");
    let path = copilot_user_settings(&config_dir);
    let expected = PathBuf::from(r"C:\Users\test\AppData\Roaming")
        .join("Code")
        .join("User")
        .join("settings.json");
    assert_eq!(path, expected);
}

#[test]
fn vscode_settings_relative() {
    let path = vscode_user_settings(&PathBuf::from(".config"));
    assert_eq!(path, PathBuf::from(".config/Code/User/settings.json"));
}

#[test]
fn copilot_cli_config_dir_macos() {
    // macOS does NOT use dirs::config_dir() (~/Library/Application Support);
    // Copilot CLI uses the XDG-style ~/.config/github-copilot.
    let home = PathBuf::from("/Users/test");
    let config_dir = PathBuf::from("/Users/test/Library/Application Support");
    let path = copilot_cli_config_dir_for(&home, &config_dir, "macos");
    assert_eq!(path, PathBuf::from("/Users/test/.config/github-copilot"));
}

#[test]
fn copilot_cli_config_dir_linux() {
    let home = PathBuf::from("/home/test");
    let config_dir = PathBuf::from("/home/test/.config");
    let path = copilot_cli_config_dir_for(&home, &config_dir, "linux");
    assert_eq!(path, PathBuf::from("/home/test/.config/github-copilot"));
}

#[test]
fn copilot_cli_config_dir_windows() {
    let home = PathBuf::from(r"C:\Users\test");
    let config_dir = PathBuf::from(r"C:\Users\test\AppData\Roaming");
    let path = copilot_cli_config_dir_for(&home, &config_dir, "windows");
    let expected = PathBuf::from(r"C:\Users\test\AppData\Roaming").join("github-copilot");
    assert_eq!(path, expected);
}
