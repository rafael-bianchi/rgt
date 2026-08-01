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

    Ok(())
}
