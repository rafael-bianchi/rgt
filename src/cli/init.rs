use crate::hooks::installer::detect_and_configure_hooks;
use crate::store::DbStore;

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
