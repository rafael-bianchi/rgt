use crate::hooks::installer::{
    detect_and_configure_hooks, resolve_agent_name, valid_agent_names, AgentOutcome, InstallReport,
};
use crate::hooks::parser::NumberFormat;
use crate::store::queries::set_setting;

/// Resolves an `--agent` spelling (canonical or alias) to its canonical name.
/// Unknown names produce an error message listing every valid `--agent` value
/// (FR-001). The CLI maps this error to exit code 2 (invalid input).
pub fn resolve_agent_arg(agent: Option<&str>) -> Result<Option<String>, String> {
    match agent {
        None => Ok(None),
        Some(name) => match resolve_agent_name(name) {
            Some(canonical) => Ok(Some(canonical)),
            None => Err(format!(
                "Unknown agent '{}'. Valid --agent values: {}",
                name,
                valid_agent_names().join(", ")
            )),
        },
    }
}

/// True when at least one hook was newly configured, so `rgt init` should tell
/// the user to restart their AI tool for capture to take effect (FR-005). A
/// pure no-op re-run (all `Skipped`) does not nag.
pub fn needs_restart_reminder(report: &InstallReport) -> bool {
    report
        .outcomes
        .iter()
        .any(|o| matches!(o, AgentOutcome::Configured { .. }))
}

/// Executes `rgt init`: initializes the `.rgt/store.db` database and configures
/// AI agent hooks (global, forced, or per-agent via `--agent`).
///
/// Per-agent outcomes are printed (configured / skipped / failed with reason).
/// If any agent fails to configure, the command returns an error so the CLI
/// exits with code 1 (expected error, per the constitution); healthy agents are
/// still configured (FR-008).
pub fn execute_init(
    global: bool,
    force: bool,
    agent: Option<&str>,
    number_format: Option<NumberFormat>,
) -> Result<(), String> {
    let db = crate::store::DbStore::open_in_project(".")
        .map_err(|e| format!("Database init failed: {}", e))?;
    if let Some(fmt) = number_format {
        set_setting(db.conn(), "number_format", fmt.as_str())
            .map_err(|e| format!("Failed to persist --number-format: {}", e))?;
    }
    println!("✓ Initialized RGT database at .rgt/store.db");

    let report = detect_and_configure_hooks(global, force, agent)
        .map_err(|e| format!("Hook configuration failed: {}", e))?;

    let mut failed = 0usize;
    let mut configured = 0usize;
    // FR-005: compute before `report.outcomes` is consumed by the loop below.
    let remind_restart = needs_restart_reminder(&report);
    if !report.is_empty() {
        println!("✓ Auto-configured AI coding agent hooks:");
        for outcome in report.outcomes {
            match outcome {
                AgentOutcome::Configured { agent, artifact } => {
                    println!("  - {} -> {}", agent, artifact.display());
                    configured += 1;
                }
                AgentOutcome::Skipped { agent } => {
                    println!(
                        "  - {} already configured (use --force to overwrite)",
                        agent
                    );
                }
                AgentOutcome::Failed { agent, reason } => {
                    eprintln!("✗ {}: {}", agent, reason);
                    failed += 1;
                }
            }
        }
    } else {
        println!("! No supported AI coding tool detected or no new hook configs written.");
    }

    if configured == 0 && failed == 0 {
        println!(
            "! No new AI coding tool hook configs written (use --force to overwrite existing)."
        );
    }

    // FR-005: hooks are read at session start — remind the user to restart.
    if remind_restart {
        println!();
        println!(
            "Restart your AI coding tool now for the newly installed RGT hooks to take effect."
        );
    }

    println!();
    println!("Verify derived calculations: rgt verify --parents <ids> --operation EXPRESSION --expression \"a + b\" --result <val>");
    println!("  Variables: parent[0]=a, parent[1]=b, parent[2]=c, ...");
    println!("  Supported operations: EXPRESSION, DATE_DIFF");
    println!("  Exit codes: 0=match, 1=mismatch (retry with correct result), 2=invalid input");

    if failed > 0 {
        return Err(format!(
            "{} agent(s) failed to configure — see errors above (backup copies, where written, are kept as *.rgt.bak).",
            failed
        ));
    }
    Ok(())
}
