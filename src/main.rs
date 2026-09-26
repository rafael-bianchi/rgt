mod cli;
mod detection;
mod export;
mod extract;
mod graph;
mod hooks;
mod query;
mod store;
mod types;
mod updater;
mod verify;

use crate::hooks::parser::NumberFormat;
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
        /// Detect and configure global hooks. Agents: Claude Code, Cursor,
        /// Copilot, Gemini, Mistral Vibe (full hook); OpenCode, Pi, Hermes
        /// (plugin); Windsurf, Codex CLI, Cline/Roo Code, Antigravity, Kilo
        /// (rules-file).
        #[arg(short = 'g', long)]
        global: bool,

        /// Overwrite existing hook configurations
        #[arg(long)]
        force: bool,

        /// Target a specific agent: claude-code, cursor, codex, windsurf,
        /// copilot, gemini, vibe, opencode, pi, hermes, cline, antigravity,
        /// kilocode (or aliases claude, roo-code, kilo). Detects all if omitted.
        #[arg(long)]
        agent: Option<String>,

        /// Number parsing locale: us, eu, or auto. Persisted for the passive hook path.
        #[arg(long, value_parser = ["us", "eu", "auto"])]
        number_format: Option<String>,
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
    #[command(
        long_about = "Query provenance lineage or value status by node ID.\n\nDuration display units are fixed elapsed-time units: seconds, minutes, hours, days, and weeks. Stored Durations remain exact whole seconds; calendar months and years are unsupported. Selecting --unit changes only this response and is not stored as node metadata. Legacy EXPRESSION calculations with Date or Duration parents still return a Number and emit a warning: Dates are Unix timestamp seconds and Durations are elapsed seconds."
    )]
    Query {
        /// Target node ID to query
        node_id: String,

        /// Output lineage as JSON
        #[arg(long)]
        json: bool,

        /// Response-only Duration display: seconds, minutes, hours, days, or weeks; not stored
        #[arg(long, value_parser = ["seconds", "minutes", "hours", "days", "weeks"])]
        unit: Option<String>,
    },
    /// Export or visualize the dependency graph structure
    Graph {
        /// Export format: text, mermaid, dot, ttl
        #[arg(short, long, default_value = "text")]
        format: String,

        /// Include available absolute source paths in Turtle output (privacy-sensitive)
        #[arg(long)]
        include_absolute_paths: bool,
    },
    /// Process passive execution hooks (PreToolUse / PostToolUse)
    Hook {
        /// Hook event type: pre or post
        event: String,

        /// Target agent stdin dialect: claude-code, cursor, codex, windsurf,
        /// copilot, gemini, vibe, opencode, pi, hermes, cline, antigravity,
        /// kilocode (or aliases claude, roo-code, kilo). Omitted = Claude Code format.
        #[arg(long)]
        agent: Option<String>,
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

        /// Skip SHA-256 checksum verification (INSECURE — never the default)
        #[arg(long)]
        skip_checksum: bool,

        /// Skip GitHub Artifact Attestation verification (INSECURE — never the default)
        #[arg(long)]
        skip_attestation: bool,
    },
    /// Diagnose hook installation and capture health (0 = healthy/warnings, 1 = error)
    Doctor,
    /// Remove obsolete (stale, dependent-free) nodes from the provenance graph
    Gc {
        /// Reclaim physical disk space with VACUUM after collection
        #[arg(long)]
        vacuum: bool,
    },
    /// Verify that a derived value matches its parent nodes
    #[command(
        long_about = "Verify a derived value against its parent nodes.\n\nTemporal claims use exact whole-second Duration values and the fixed elapsed-time units seconds, minutes, hours, days, and weeks. Calendar months and years are unsupported. --result-unit qualifies a claim only for verification and is not stored as node metadata. Legacy EXPRESSION calculations with Date or Duration parents still return a Number and emit a warning: Dates are Unix timestamp seconds and Durations are elapsed seconds. Use DATE_DIFF or typed Duration operations when a Duration result is intended."
    )]
    Verify {
        /// Comma-separated parent node IDs in variable order (parent[0]=a, parent[1]=b, ...)
        #[arg(long)]
        parents: String,

        /// Operation type: EXPRESSION, DATE_DIFF, DURATION_SUM, or DURATION_AVG
        #[arg(long)]
        operation: String,

        /// Formula string for EXPRESSION (e.g., "(a + b) * c / 100")
        #[arg(long)]
        expression: Option<String>,

        /// Expected result (seconds by default for temporal operations)
        #[arg(long, allow_hyphen_values = true)]
        result: String,

        /// Claim unit: seconds, minutes, hours, days, or weeks; exact whole seconds only
        #[arg(long)]
        result_unit: Option<String>,
    },
    /// Record numeric and date values from a file into the provenance graph
    Record {
        /// Path to the data file to record values from
        file: String,

        /// Read file content from stdin instead of disk (path still required for node IDs)
        #[arg(long)]
        stdin: bool,

        /// Number parsing locale: us, eu, or auto (defaults to persisted setting, then auto)
        #[arg(long, value_parser = ["us", "eu", "auto"])]
        number_format: Option<String>,
    },
    /// Verify and record a derived value from parent nodes
    #[command(
        long_about = "Verify and record a derived value from parent nodes.\n\nTemporal operations store exact whole-second Duration values. Claims default to seconds; supported fixed elapsed-time units are seconds, minutes, hours, days, and weeks. Calendar months and years are unsupported. --result-unit and --unit qualify a claim or display only and are not stored as node metadata. Legacy EXPRESSION calculations with Date or Duration parents still return a Number and emit a warning: Dates are Unix timestamp seconds and Durations are elapsed seconds. Use DATE_DIFF or typed Duration operations when a Duration result is intended."
    )]
    Derive {
        /// Comma-separated parent node IDs in variable order (parent[0]=a, parent[1]=b, ...)
        #[arg(long)]
        parents: String,

        /// Operation type: EXPRESSION, DATE_DIFF, DURATION_SUM, or DURATION_AVG
        #[arg(long)]
        operation: String,

        /// Formula string for EXPRESSION (e.g., "(a + b) * c / 100")
        #[arg(long)]
        expression: Option<String>,

        /// Optional result claim (required for EXPRESSION)
        #[arg(long, allow_hyphen_values = true)]
        result: Option<String>,

        /// Claim unit: seconds, minutes, hours, days, or weeks; exact whole seconds only
        #[arg(long)]
        result_unit: Option<String>,

        /// Display unit: seconds, minutes, hours, days, or weeks; response only, not stored
        #[arg(long)]
        unit: Option<String>,
    },
}

/// Maps a validated `--number-format` literal to a `NumberFormat` (clap's
/// `value_parser` already restricts the input to us/eu/auto, so this is
/// infallible). `None` means the caller falls back to setting/auto.
fn parse_number_format(s: Option<&str>) -> Option<NumberFormat> {
    s.and_then(|raw| raw.parse().ok())
}

fn main() {
    // Reserve time for process-launch and scheduling overhead so a blocked
    // stdout write remains observable as a sub-second CLI failure.
    let command_deadline = std::time::Instant::now() + std::time::Duration::from_millis(400);
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Init {
            global,
            force,
            agent,
            number_format,
        } => match cli::resolve_agent_arg(agent.as_deref()) {
            Ok(resolved) => {
                let fmt = parse_number_format(number_format.as_deref());
                cli::execute_init(global, force, resolved.as_deref(), fmt)
            }
            Err(msg) => {
                eprintln!("Error: {}", msg);
                std::process::exit(2);
            }
        },
        Commands::Status { stale_only, json } => cli::execute_status(stale_only, json),
        Commands::Query {
            node_id,
            json,
            unit,
        } => {
            let query_result = if let Some(unit) = unit.as_deref() {
                cli::execute_query_with_unit(&node_id, json, Some(unit))
            } else {
                cli::execute_query(&node_id, json)
            };
            match query_result {
                Ok(()) => Ok(()),
                Err(crate::query::QueryError::InvalidInput(message)) => {
                    eprintln!("Error: {message}");
                    std::process::exit(2);
                }
                Err(crate::query::QueryError::Error(message)) => Err(message),
            }
        }
        Commands::Graph {
            format,
            include_absolute_paths,
        } => {
            if include_absolute_paths && !format.eq_ignore_ascii_case("ttl") {
                eprintln!("error: --include-absolute-paths is only valid with --format ttl");
                std::process::exit(2);
            }
            cli::execute_graph_with_options(&format, include_absolute_paths, command_deadline)
        }
        Commands::Hook { event, agent } => {
            if let Err(e) = hooks::handle_passive_hook_event(&event, agent.as_deref()) {
                eprintln!("Hook warning: {}", e);
            }
            Ok(())
        }
        Commands::Update {
            check,
            yes,
            version,
            skip_checksum,
            skip_attestation,
        } => match cli::execute_update(check, yes, version, skip_checksum, skip_attestation) {
            Ok(()) => Ok(()),
            Err(e) => {
                eprintln!("Error: {}", e.message);
                std::process::exit(e.code);
            }
        },
        Commands::Doctor => match cli::execute_doctor() {
            Ok(code) => std::process::exit(code),
            Err(e) => {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        },
        Commands::Gc { vacuum } => cli::execute_gc(vacuum),
        Commands::Verify {
            parents,
            operation,
            expression,
            result,
            result_unit,
        } => {
            crate::verify::run_verify_cli(
                &parents,
                &operation,
                expression.as_deref(),
                &result,
                result_unit.as_deref(),
            );
            Ok(())
        }
        Commands::Record {
            file,
            stdin,
            number_format,
        } => cli::execute_record(&file, stdin, parse_number_format(number_format.as_deref())),
        Commands::Derive {
            parents,
            operation,
            expression,
            result,
            result_unit,
            unit,
        } => cli::execute_derive(
            &parents,
            &operation,
            expression.as_deref(),
            result.as_deref(),
            result_unit.as_deref(),
            unit.as_deref(),
        ),
    };

    if let Err(err_msg) = result {
        eprintln!("Error: {}", err_msg);
        std::process::exit(1);
    }
}
