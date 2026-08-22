use rgt::hooks::installer::detect_installed_agents;
use rgt::hooks::paths::copilot_cli_config_dir;
use std::path::Path;
use std::sync::Mutex;
use tempfile::tempdir;

static CWD_MUTEX: Mutex<()> = Mutex::new(());

fn set_cwd(dir: &Path) -> std::sync::MutexGuard<'static, ()> {
    let guard = CWD_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
    std::env::set_current_dir(dir).unwrap();
    guard
}

fn write(path: &Path) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(path, "# test\n").unwrap();
}

#[test]
fn no_triggers_yields_empty_without_error() {
    let dir = tempdir().unwrap();
    let _guard = set_cwd(dir.path());
    let config_dir = dir.path().join("config");
    let detected = detect_installed_agents(dir.path(), &config_dir);
    assert!(detected.is_empty());
}

#[test]
fn home_triggers_detected() {
    let dir = tempdir().unwrap();
    let _guard = set_cwd(dir.path());
    let config_dir = dir.path().join("config");

    write(&dir.path().join(".claude").join("settings.json"));
    write(&dir.path().join(".gemini").join("hooks.toml"));
    write(&dir.path().join(".hermes").join("plugins").join("x"));

    let detected = detect_installed_agents(dir.path(), &config_dir);
    assert!(detected.contains(&"claude-code".to_string()));
    assert!(detected.contains(&"gemini".to_string()));
    assert!(detected.contains(&"hermes".to_string()));
    assert!(!detected.contains(&"cursor".to_string()));
}

#[test]
fn project_triggers_detected() {
    let dir = tempdir().unwrap();
    let _guard = set_cwd(dir.path());
    let config_dir = dir.path().join("config");
    let cwd = dir.path();

    write(&cwd.join(".clinerules"));
    write(&cwd.join(".kilocode").join("rules").join("x"));
    write(&cwd.join(".agents").join("rules").join("x"));
    write(&cwd.join(".opencode").join("plugin").join("x"));

    let detected = detect_installed_agents(dir.path(), &config_dir);
    assert!(detected.contains(&"cline".to_string()));
    assert!(detected.contains(&"kilocode".to_string()));
    assert!(detected.contains(&"antigravity".to_string()));
    assert!(detected.contains(&"opencode".to_string()));
}

#[test]
fn copilot_detected_via_config_dir_settings() {
    let dir = tempdir().unwrap();
    let _guard = set_cwd(dir.path());
    let config_dir = dir.path().join("config");
    write(&config_dir.join("Code").join("User").join("settings.json"));

    let detected = detect_installed_agents(dir.path(), &config_dir);
    assert!(detected.contains(&"copilot".to_string()));
}

#[test]
fn copilot_detected_via_cli_config_dir() {
    let dir = tempdir().unwrap();
    let _guard = set_cwd(dir.path());
    let config_dir = dir.path().join("config");
    write(&copilot_cli_config_dir(dir.path(), &config_dir).join("config.json"));

    let detected = detect_installed_agents(dir.path(), &config_dir);
    assert!(detected.contains(&"copilot".to_string()));
}

#[test]
fn agent_config_dir_contains_windsurf_and_codex() {
    let dir = tempdir().unwrap();
    let _guard = set_cwd(dir.path());
    let config_dir = dir.path().join("config");
    let cwd = dir.path();

    write(&cwd.join("AGENTS.md"));
    write(&cwd.join(".windsurfrules"));

    let detected = detect_installed_agents(dir.path(), &config_dir);
    assert!(detected.contains(&"codex".to_string()));
    assert!(detected.contains(&"windsurf".to_string()));
}
