mod cli;
mod detection;
mod graph;
mod hooks;
mod query;
mod store;
mod types;
mod updater;
mod verify;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "rgt")]
#[command(about = "Rust Graph Tracker: Numeric and date provenance tracking with expression evaluation and derivation verification for LLM coding agents", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize RGT project tracking and configure AI agent hooks
    Init {
        /// Detect and configure global hooks for Claude Code, Cursor (full hook), Windsurf
        /// (rules-file) and Codex CLI (rules-file)
        #[arg(short = 'g', long)]
        global: bool,

        /// Overwrite existing hook configurations
        #[arg(long)]
        force: bool,

        /// Target a specific agent: claude-code, cursor (full hook), windsurf, codex (rules-file). Detects all if omitted.
        #[arg(long)]
        agent: Option<String>,
    },
    /// Inspect provenance graph status, active nodes, and stale values
    Status {
        /// Show only stale nodes
        #[arg(long)]
        stale_only: bool,

        /// Output status summary in JSON format
        #[arg(long)]
        json: bool,
    },
    /// Query provenance lineage or value status by node ID
    Query {
        /// Target node ID to query
        node_id: String,

        /// Output lineage as JSON
        #[arg(long)]
        json: bool,
    },
    /// Export or visualize the dependency graph structure
    Graph {
        /// Export format: text, mermaid, dot
        #[arg(short, long, default_value = "text")]
        format: String,
    },
    /// Process passive execution hooks (PreToolUse / PostToolUse)
    Hook {
        /// Hook event type: pre or post
        event: String,
    },
    /// Check for and install binary updates from GitHub Releases
    Update {
        /// Only check if an update is available (no download)
        #[arg(long)]
        check: bool,

        /// Skip confirmation prompt
        #[arg(short = 'y', long)]
        yes: bool,

        /// Target a specific release version tag
        #[arg(long)]
        version: Option<String>,
    },
    /// Verify that a derived value matches its parent nodes
    Verify {
        /// Comma-separated parent node IDs in variable order (parent[0]=a, parent[1]=b, ...)
        #[arg(long)]
        parents: String,

        /// Operation type: EXPRESSION or DATE_DIFF
        #[arg(long)]
        operation: String,

        /// Formula string for EXPRESSION (e.g., "(a + b) * c / 100")
        #[arg(long)]
        expression: Option<String>,

        /// Expected numeric result to verify
        #[arg(long)]
        result: f64,
    },
    /// Record numeric and date values from a file into the provenance graph
    Record {
        /// Path to the data file to record values from
        file: String,

        /// Read file content from stdin instead of disk (path still required for node IDs)
        #[arg(long)]
        stdin: bool,
    },
    /// Verify and record a derived value from parent nodes
    Derive {
        /// Comma-separated parent node IDs in variable order (parent[0]=a, parent[1]=b, ...)
        #[arg(long)]
        parents: String,

        /// Operation type: EXPRESSION or DATE_DIFF
        #[arg(long)]
        operation: String,

        /// Formula string for EXPRESSION (e.g., "(a + b) * c / 100")
        #[arg(long)]
        expression: Option<String>,

        /// Expected numeric result to verify and record
        #[arg(long)]
        result: f64,
    },
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Init {
            global,
            force,
            agent,
        } => cli::execute_init(global, force, agent.as_deref()),
        Commands::Status { stale_only, json } => cli::execute_status(stale_only, json),
        Commands::Query { node_id, json } => cli::execute_query(&node_id, json),
        Commands::Graph { format } => cli::execute_graph(&format),
        Commands::Hook { event } => {
            if let Err(e) = hooks::handle_passive_hook_event(&event) {
                eprintln!("Hook warning: {}", e);
            }
            Ok(())
        }
        Commands::Update {
            check,
            yes,
            version,
        } => cli::execute_update(check, yes, version),
        Commands::Verify {
            parents,
            operation,
            expression,
            result,
        } => {
            rgt::verify::run_verify_cli(&parents, &operation, expression.as_deref(), result);
            Ok(())
        }
        Commands::Record { file, stdin } => cli::execute_record(&file, stdin),
        Commands::Derive {
            parents,
            operation,
            expression,
            result,
        } => cli::execute_derive(&parents, &operation, expression.as_deref(), result),
    };

    if let Err(err_msg) = result {
        eprintln!("Error: {}", err_msg);
        std::process::exit(1);
    }
}
