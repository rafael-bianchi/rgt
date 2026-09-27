use crate::hooks::registration::{GuidanceHealth, GuidanceInspection, RegistrationKind};
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

/// Runs store and aggregate local-hook presence checks for tests and returns
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

    match open_store() {
        Ok(_db) => {}
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
            message: "No local RGT registration was identified — run `rgt init` to configure a supported integration.".into(),
        });
    } else {
        diagnostics.push(Diagnostic {
            name: "hooks",
            status: DiagnosticStatus::Healthy,
            message: "At least one local RGT registration was identified; this does not verify client loading or value capture.".into(),
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

/// A local guidance finding. The file may be usable even when the host has not
/// loaded it, and it is independent of the native hook registration status.
pub fn guidance_diagnostic(inspection: &GuidanceInspection) -> Diagnostic {
    let (mut status, detail) = match inspection.health {
        GuidanceHealth::Usable => (
            DiagnosticStatus::Healthy,
            "RGT-owned recording instructions are complete",
        ),
        GuidanceHealth::UsableUnmanaged => (
            DiagnosticStatus::Warning,
            "recording instructions are readable, but the RGT closing marker is absent; review ownership before rewriting",
        ),
        GuidanceHealth::Missing => (
            DiagnosticStatus::Warning,
            "RGT recording instructions are missing; rerun `rgt init` for this client",
        ),
        GuidanceHealth::Incomplete => (
            DiagnosticStatus::Error,
            "RGT-owned recording instructions are incomplete; repair the RGT block before rerunning `rgt init`",
        ),
        GuidanceHealth::Unowned => (
            DiagnosticStatus::Warning,
            "the file contains no RGT-owned recording instructions; preserve its contents and inspect ownership before initializing",
        ),
    };
    let instruction_only = crate::hooks::registration::ClientSurface::by_id(inspection.surface_id)
        .is_some_and(|surface| surface.registration_kind == RegistrationKind::Instruction);
    if instruction_only && status == DiagnosticStatus::Healthy {
        status = DiagnosticStatus::Warning;
    }
    let tier = if instruction_only {
        " Instruction-only recording; no automatic read capture."
    } else {
        ""
    };
    let state = match inspection.health {
        GuidanceHealth::Usable => "Managed",
        GuidanceHealth::UsableUnmanaged => "Hand-maintained",
        GuidanceHealth::Missing => "Missing",
        GuidanceHealth::Incomplete => "Incomplete",
        GuidanceHealth::Unowned => "Unowned",
    };
    Diagnostic {
        name: "guidance",
        status,
        message: format!(
            "{}: {} guidance at {}. {}.{} Local inspection does not prove client loading or read capture.",
            inspection.surface_id,
            state,
            inspection.artifact_path.display(),
            detail,
            tier
        ),
    }
}

/// Executes `rgt doctor` for the current project (production entry point).
pub fn execute_doctor() -> Result<i32, String> {
    let home = dirs::home_dir().unwrap_or_default();
    let config_dir = dirs::config_dir().unwrap_or_else(|| home.join(".config"));
    let surface_registrations =
        crate::hooks::registration::inspect_local_registrations(&home, &config_dir);
    let guidance_inspections =
        crate::hooks::registration::inspect_local_guidance(&home, &config_dir);
    let hooks_present = surface_registrations.iter().any(|registration| {
        matches!(
            registration.health,
            crate::hooks::registration::RegistrationHealth::Active
                | crate::hooks::registration::RegistrationHealth::GuidanceOnly
        )
    });
    let (mut diagnostics, mut code) = run_doctor(
        || std::path::Path::new(".rgt").join("store.db").exists(),
        || DbStore::open_in_project(".").map_err(|e| e.to_string()),
        || hooks_present,
    );

    for registration in surface_registrations {
        if registration.registration_kind == RegistrationKind::Instruction {
            continue; // Its distinct guidance finding appears below.
        }
        let status = match registration.health {
            crate::hooks::registration::RegistrationHealth::Active => DiagnosticStatus::Healthy,
            crate::hooks::registration::RegistrationHealth::GuidanceOnly
            | crate::hooks::registration::RegistrationHealth::Missing
            | crate::hooks::registration::RegistrationHealth::TrustPending
            | crate::hooks::registration::RegistrationHealth::InactiveWorkspace => {
                DiagnosticStatus::Warning
            }
            crate::hooks::registration::RegistrationHealth::Malformed
            | crate::hooks::registration::RegistrationHealth::Obsolete
            | crate::hooks::registration::RegistrationHealth::Ambiguous => DiagnosticStatus::Error,
        };
        if status == DiagnosticStatus::Error {
            code = 1;
        }
        let caveat = match registration.surface_id {
            "codex" => " Codex may still require project trust approval in `/hooks`.",
            "windsurf" => {
                " Hooks are inactive in Restricted Mode; system and user hooks may take precedence."
            }
            _ => " Local presence does not prove the client loaded or invoked it.",
        };
        let hermes_config_detail = if registration.surface_id == "hermes"
            && matches!(
                registration.health,
                crate::hooks::registration::RegistrationHealth::Obsolete
                    | crate::hooks::registration::RegistrationHealth::Malformed
            ) {
            let (path, init_command) = match registration.scope {
                crate::hooks::registration::RegistrationScope::Project => (
                    std::path::PathBuf::from(".hermes/config.toml"),
                    "rgt init --agent hermes",
                ),
                crate::hooks::registration::RegistrationScope::User => (
                    home.join(".hermes/config.toml"),
                    "rgt init -g --agent hermes",
                ),
            };
            format!(
                " Check `plugins.enabled` for `rgt` in {}; repair malformed TOML or run `{init_command}`.",
                path.display()
            )
        } else {
            String::new()
        };
        diagnostics.push(Diagnostic {
            name: "surface",
            status,
            message: format!(
                "{}: {:?} registration at {}; value-capture tier {:?}.{}{}",
                registration.surface_id,
                registration.health,
                registration.artifact_path.display(),
                registration.capture_tier,
                caveat,
                hermes_config_detail
            ),
        });
    }

    for inspection in guidance_inspections {
        let diagnostic = guidance_diagnostic(&inspection);
        if diagnostic.status == DiagnosticStatus::Error {
            code = 1;
        }
        diagnostics.push(diagnostic);
    }

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
    fn initialized_store_with_hooks_is_local_health_only() {
        let (diags, code) = run_doctor(store_exists, in_memory_store, hooks);
        assert_eq!(code, 0);
        let hook = diags.iter().find(|d| d.name == "hooks").unwrap();
        assert_eq!(hook.status, DiagnosticStatus::Healthy);
        assert!(hook.message.contains("does not verify"));
    }

    #[test]
    fn initialized_without_hooks_is_warning_exit_0() {
        let (diags, code) = run_doctor(store_exists, in_memory_store, no_hooks);
        assert_eq!(code, 0);
        assert!(has_status(&diags, "hooks", DiagnosticStatus::Warning));
    }
}
