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

// ---------------------------------------------------------------------------
// 025: change-detection hardening (tier1/tier2/evaluate_file_change)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod detection_tests {
    use rgt::detection::{
        compute_blake3_hash, compute_blake3_hash_from_bytes, evaluate_file_change,
        get_metadata_snapshot, tier1_check_changed, DetectionResult,
    };
    use rgt::store::queries::upsert_source_document;
    use rgt::store::DbStore;

    // ---- T002: identity fields + hash-from-bytes ----

    #[test]
    fn metadata_snapshot_reports_size_and_identity() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("f.txt");
        std::fs::write(&p, "amount,42\n").unwrap();
        let m = get_metadata_snapshot(&p).unwrap();
        assert_eq!(m.file_size, 10);
        #[cfg(unix)]
        assert!(m.dev.is_some() && m.ino.is_some());
    }

    #[test]
    fn hash_from_bytes_matches_hash_of_same_file() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("f.txt");
        let bytes = b"revenue,120000\ncosts,60000\n";
        std::fs::write(&p, bytes).unwrap();
        assert_eq!(
            compute_blake3_hash_from_bytes(bytes),
            compute_blake3_hash(&p).unwrap()
        );
    }

    #[test]
    fn tier1_detects_identity_change_and_ignores_none() {
        let dir = tempfile::tempdir().unwrap();
        let a = dir.path().join("a");
        let b = dir.path().join("b");
        std::fs::write(&a, "same").unwrap();
        std::fs::write(&b, "same").unwrap();
        let ma = get_metadata_snapshot(&a).unwrap();
        let mb = get_metadata_snapshot(&b).unwrap();
        #[cfg(unix)]
        {
            // Different inodes => changed, even with identical mtime/size.
            assert!(tier1_check_changed(
                &mb,
                ma.mtime_nsec,
                ma.file_size,
                ma.dev,
                ma.ino
            ));
        }
        // None identity is never a change by itself.
        let none_meta = get_metadata_snapshot(&a).unwrap();
        let no_id = get_metadata_snapshot(&a).unwrap();
        assert!(!tier1_check_changed(
            &none_meta,
            no_id.mtime_nsec,
            no_id.file_size,
            None,
            None
        ));
    }

    // ---- T005 (US1): forced re-hash catches a same-size/same-mtime rewrite ----

    #[test]
    fn force_hash_catches_same_size_same_mtime_rewrite() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("f.txt");
        // Both versions are the same length (11 bytes).
        std::fs::write(&p, "amount,100\n").unwrap();
        let old_hash = compute_blake3_hash_from_bytes(b"amount,100\n");

        std::fs::write(&p, "amount,999\n").unwrap();
        let cur = get_metadata_snapshot(&p).unwrap();
        let cur_hash = compute_blake3_hash_from_bytes(b"amount,999\n");
        assert_ne!(old_hash, cur_hash);

        // Fast path with the CURRENT mtime/size but the OLD hash: Tier 1 matches
        // (false negative), so result is Unchanged.
        let fast = evaluate_file_change(
            &p,
            cur.mtime_nsec,
            cur.file_size,
            &old_hash,
            cur.dev,
            cur.ino,
            false,
        );
        assert!(matches!(fast, DetectionResult::Unchanged));

        // Forced re-hash catches it.
        let forced = evaluate_file_change(
            &p,
            cur.mtime_nsec,
            cur.file_size,
            &old_hash,
            cur.dev,
            cur.ino,
            true,
        );
        assert!(matches!(forced, DetectionResult::Changed { .. }));
    }

    // ---- T011 (US3): NotFound vs other I/O errors ----

    #[test]
    fn missing_file_is_file_not_found() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("nope.txt");
        let res = evaluate_file_change(&p, 0, 0, "h", None, None, false);
        assert!(matches!(res, DetectionResult::FileNotFound));
    }

    #[cfg(unix)]
    #[test]
    fn permission_error_is_check_failed_not_deleted() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("secret.txt");
        std::fs::write(&p, "amount,1\n").unwrap();
        std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o000)).unwrap();

        let res = evaluate_file_change(&p, 0, 0, "h", None, None, true);
        std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o644)).unwrap();
        assert!(matches!(res, DetectionResult::CheckFailed));
    }

    // ---- T013 (US4): symlink repoint via inode (Unix) ----

    #[cfg(unix)]
    #[test]
    fn symlink_repoint_to_different_inode_is_changed() {
        let dir = tempfile::tempdir().unwrap();
        let target_a = dir.path().join("a");
        let target_b = dir.path().join("b");
        let link = dir.path().join("link");
        std::fs::write(&target_a, "same").unwrap();
        std::fs::write(&target_b, "same").unwrap();
        std::os::unix::fs::symlink(&target_a, &link).unwrap();
        let ma = get_metadata_snapshot(&link).unwrap();

        std::fs::remove_file(&link).unwrap();
        std::os::unix::fs::symlink(&target_b, &link).unwrap();
        let mb = get_metadata_snapshot(&link).unwrap();

        // Same size and (coerced) mtime, different inode => changed.
        assert!(
            tier1_check_changed(&mb, ma.mtime_nsec, ma.file_size, ma.dev, ma.ino),
            "a repoint to a different inode must be detected"
        );
    }

    // ---- T015 (US5): casing variants dedupe on identity ----

    #[test]
    fn casing_variants_with_same_identity_dedupe() {
        let dir = tempfile::tempdir().unwrap();
        let db = DbStore::open_in_memory().unwrap();
        let p = dir.path().join("Data.CSV");
        std::fs::write(&p, "amount,42\n").unwrap();
        let m = get_metadata_snapshot(&p).unwrap();
        let hash = compute_blake3_hash_from_bytes(b"amount,42\n");

        upsert_source_document(
            db.conn(),
            "Data.CSV",
            m.mtime_nsec,
            m.file_size,
            &hash,
            m.dev,
            m.ino,
        )
        .unwrap();
        upsert_source_document(
            db.conn(),
            "data.csv",
            m.mtime_nsec,
            m.file_size,
            &hash,
            m.dev,
            m.ino,
        )
        .unwrap();

        // Both calls target the same identity; only one row remains.
        let count: i64 = db
            .conn()
            .query_row("SELECT COUNT(*) FROM source_documents", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 1, "case variants of the same file must dedupe");
    }
}
