use crate::store::queries::list_all_nodes;
use crate::store::DbStore;

/// Severity tier of a `rgt doctor` diagnostic (FR-001).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticStatus {
    /// Everything checks out.
    Healthy,
    /// Advisory — may warrant attention but is not a hard failure.
    Warning,
    /// Unambiguous problem (e.g. not initialized).
    Error,
}

/// A single `rgt doctor` health check result (FR-001/FR-002).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub name: &'static str,
    pub status: DiagnosticStatus,
    pub message: String,
}

/// Runs `rgt doctor` against the current project directory and returns the
/// diagnostics plus the process exit code (0 = healthy or warnings, 1 = error).
///
/// The store checks are injectable for deterministic tests; the production
/// closures probe `.rgt/store.db` existence and open it.
pub fn run_doctor(
    store_exists: impl FnOnce() -> bool,
    open_store: impl FnOnce() -> Result<DbStore, String>,
    hooks_present: impl FnOnce() -> bool,
) -> (Vec<Diagnostic>, i32) {
    let mut diagnostics = Vec::new();

    // `open_in_project` auto-creates the store, so detect "not initialized" by
    // probing for the store file before attempting to open it.
    if !store_exists() {
        diagnostics.push(Diagnostic {
            name: "store",
            status: DiagnosticStatus::Error,
            message:
                "RGT is not initialized in this project — run `rgt init` to create .rgt/store.db."
                    .into(),
        });
        return (diagnostics, 1);
    }

    let node_count = match open_store() {
        Ok(db) => match list_all_nodes(db.conn()) {
            Ok(nodes) => nodes.len(),
            Err(_) => 0,
        },
        Err(_) => {
            diagnostics.push(Diagnostic {
                name: "store",
                status: DiagnosticStatus::Error,
                message: "The RGT store exists but could not be opened — it may be corrupted."
                    .into(),
            });
            return (diagnostics, 1);
        }
    };

    diagnostics.push(Diagnostic {
        name: "store",
        status: DiagnosticStatus::Healthy,
        message: "Store initialized.".into(),
    });

    let hooks = hooks_present();
    if !hooks {
        diagnostics.push(Diagnostic {
            name: "hooks",
            status: DiagnosticStatus::Warning,
            message: "No RGT hooks are installed — `rgt init` configures agent hooks for capture."
                .into(),
        });
    } else if node_count == 0 {
        // FR-002: the silent no-op signature. Advisory only: a fresh
        // install with no captures yet is not a hard failure.
        diagnostics.push(Diagnostic {
            name: "captures",
            status: DiagnosticStatus::Warning,
            message: "Hooks are installed but no values have been recorded. If you expected captures, verify `rgt` is resolvable on the PATH of the shell your AI tool uses (hook subprocesses fail open, so a broken PATH records nothing silently).".into(),
        });
    } else {
        diagnostics.push(Diagnostic {
            name: "captures",
            status: DiagnosticStatus::Healthy,
            message: "Values are being captured.".into(),
        });
    }

    let exit_code = if diagnostics
        .iter()
        .any(|d| d.status == DiagnosticStatus::Error)
    {
        1
    } else {
        0
    };
    (diagnostics, exit_code)
}

/// Executes `rgt doctor` for the current project (production entry point).
pub fn execute_doctor() -> Result<i32, String> {
    let (diagnostics, code) = run_doctor(
        || std::path::Path::new(".rgt").join("store.db").exists(),
        || DbStore::open_in_project(".").map_err(|e| e.to_string()),
        || {
            let home = dirs::home_dir();
            let config_dir = dirs::config_dir();
            match (home, config_dir) {
                (Some(h), Some(c)) => {
                    !crate::hooks::installer::list_hook_artifacts(&h, &c).is_empty()
                }
                _ => false,
            }
        },
    );

    println!("=== RGT Doctor ===");
    for d in &diagnostics {
        let mark = match d.status {
            DiagnosticStatus::Healthy => "✓",
            DiagnosticStatus::Warning => "⚠",
            DiagnosticStatus::Error => "✗",
        };
        println!("{} [{}] {}", mark, d.name, d.message);
    }
    println!();
    if code == 0 {
        println!("✓ Environment healthy (or only advisory warnings present).");
    } else {
        println!("✗ Problems found — see above.");
    }
    Ok(code)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::DbStore;

    fn no_store() -> bool {
        false
    }

    fn store_exists() -> bool {
        true
    }

    fn in_memory_store() -> Result<DbStore, String> {
        DbStore::open_in_memory().map_err(|e| e.to_string())
    }

    fn no_hooks() -> bool {
        false
    }

    fn hooks() -> bool {
        true
    }

    fn has_status(diags: &[Diagnostic], name: &str, status: DiagnosticStatus) -> bool {
        diags.iter().any(|d| d.name == name && d.status == status)
    }

    #[test]
    fn not_initialized_is_an_error_exit_1() {
        let (diags, code) = run_doctor(no_store, in_memory_store, no_hooks);
        assert_eq!(code, 1);
        assert!(has_status(&diags, "store", DiagnosticStatus::Error));
    }

    #[test]
    fn initialized_with_values_is_healthy_exit_0() {
        let db = in_memory_store().unwrap();
        // Record a value so the graph is non-empty.
        let now = chrono::Utc::now();
        let node = crate::types::TrackedNode {
            id: "doctor-test-node".into(),
            node_type: crate::types::NodeType::Root,
            value_kind: crate::types::ValueKind::Number,
            value: crate::types::ValueData::Number(42.0),
            source_doc_id: None,
            line_number: Some(1),
            is_stale: false,
            stale_reason: None,
            created_at: now,
            updated_at: now,
        };
        crate::store::queries::insert_tracked_node(db.conn(), &node).unwrap();
        let (diags, code) = run_doctor(store_exists, || Ok(db), hooks);
        assert_eq!(code, 0);
        assert!(has_status(&diags, "captures", DiagnosticStatus::Healthy));
    }

    #[test]
    fn initialized_empty_graph_with_hooks_is_warning_exit_0() {
        let (diags, code) = run_doctor(store_exists, in_memory_store, hooks);
        assert_eq!(code, 0);
        assert!(has_status(&diags, "captures", DiagnosticStatus::Warning));
    }

    #[test]
    fn initialized_without_hooks_is_warning_exit_0() {
        let (diags, code) = run_doctor(store_exists, in_memory_store, no_hooks);
        assert_eq!(code, 0);
        assert!(has_status(&diags, "hooks", DiagnosticStatus::Warning));
    }
}
