use crate::hooks::installer::{detect_and_configure_hooks, resolve_agent_name, valid_agent_names};
use crate::store::DbStore;

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

/// Executes `rgt init`: initializes the `.rgt/store.db` database and configures
/// AI agent hooks (global, forced, or per-agent via `--agent`).
pub fn execute_init(global: bool, force: bool, agent: Option<&str>) -> Result<(), String> {
    let _db = DbStore::open_in_project(".").map_err(|e| format!("Database init failed: {}", e))?;
    println!("✓ Initialized RGT database at .rgt/store.db");

    let configured = detect_and_configure_hooks(global, force, agent)
        .map_err(|e| format!("Hook configuration failed: {}", e))?;

    if configured.is_empty() {
        println!(
            "! No new AI coding tool hook configs written (use --force to overwrite existing)."
        );
    } else {
        println!("✓ Auto-configured AI coding agent hooks:");
        for item in configured {
            println!("  - {}", item);
        }
    }

    println!();
    println!("Verify derived calculations: rgt verify --parents <ids> --operation EXPRESSION --expression \"a + b\" --result <val>");
    println!("  Variables: parent[0]=a, parent[1]=b, parent[2]=c, ...");
    println!("  Supported operations: EXPRESSION, DATE_DIFF");
    println!("  Exit codes: 0=match, 1=mismatch (retry with correct result), 2=invalid input");

    Ok(())
}
