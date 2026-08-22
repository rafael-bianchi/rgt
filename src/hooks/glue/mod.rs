//! Embedded thin glue artifacts for plugin-based and rules-file agents.
//!
//! These constants are written verbatim by the installer to each agent's
//! config path at `rgt init` time (constitution I Agent-Side Glue Exemption).
//! They are thin delegates only: they parse the agent's native event JSON and
//! invoke the `rgt` CLI as a subprocess — zero business logic (constitution
//! VIII). Every glue artifact fails open: if the `rgt` binary or the agent's
//! runtime is absent, execution is a no-op that never blocks the agent.

/// OpenCode TypeScript plugin (`rgt.ts`).
///
/// Written to `~/.config/opencode/plugins/` (global) or `.opencode/plugin/`
/// (project). Only captures post-tool events.
pub const OPENCODE_TS_PLUGIN: &str = r#"import { plugin } from "@opencode-ai/plugin";

export const rgt = plugin("rgt", {
  tool: {
    execute: {
      after: async (args) => {
        try {
          const { spawnSync } = await import("node:child_process");
          spawnSync("rgt", ["hook", "post", "--agent", "opencode"], {
            input: JSON.stringify(args),
            encoding: "utf-8",
          });
        } catch {
          // fail-open: never block or alter the agent's tool call
        }
      },
    },
  },
});
"#;

/// Pi TypeScript extension (`rgt.ts`).
///
/// Written to `.pi/extensions/` (project) or `~/.pi/agent/extensions/`
/// (`--global`). Only captures post-tool events.
pub const PI_TS_EXTENSION: &str = r#"export const extension = {
  name: "rgt",
  async setup() {},
};

export async function onToolCall(input) {
  try {
    const { spawnSync } = await import("node:child_process");
    spawnSync("rgt", ["hook", "post", "--agent", "pi"], {
      input: JSON.stringify(input),
      encoding: "utf-8",
    });
  } catch {
    // fail-open: never block or alter the agent's tool call
  }
}
"#;

/// Hermes Python plugin (`plugin.py`).
///
/// Written to `~/.hermes/plugins/rgt/` (global) or `.hermes/plugins/rgt/`
/// (project) and enabled via `plugins.enabled` in the Hermes config. Only
/// captures post-tool events.
pub const HERMES_PYTHON_PLUGIN: &str = r#"import json
import shutil
import subprocess


def pre_tool_call(args):
    _capture(args)


def post_tool_call(args):
    _capture(args)


def _capture(args):
    try:
        rgt = shutil.which("rgt")
        if rgt is None:
            return
        subprocess.run(
            [rgt, "hook", "post", "--agent", "hermes"],
            input=json.dumps(args).encode("utf-8"),
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
            timeout=5,
        )
    except Exception:
        # fail-open: never block or alter the agent's tool call
        pass
"#;

/// Cline / Roo Code rules-file section, appended to `.clinerules`.
pub const CLINE_RULES: &str = "\n## RGT Integration\n\
    RGT tracks numeric and date provenance for AI coding agents.\n\
    \n\
    ### Recording Values\n\
    After reading a data file containing numbers or dates, record its values:\n\
    - `rgt record <file>` — extracts and tracks numeric/date values from file content\n\
    - Run `rgt status` to check what's tracked\n\
    \n\
    ### Recording Derivations\n\
    After computing a derived value from tracked root nodes:\n\
    - `rgt derive --parents <id1>,<id2> --operation EXPRESSION --expression \"a - b\" --result <val>`\n\
    - The derivation is verified before recording; wrong results are rejected (exit 1)\n\
    \n\
    ### Inspecting the Graph\n\
    - `rgt status` — see all tracked nodes and staleness state\n\
    - `rgt query <node_id>` — trace provenance lineage for a value\n\
    - `rgt graph` — export the dependency graph as text or mermaid\n\
    \n\
    Run `rgt --help` for all available commands.\n";

/// Antigravity rules file, written to `.agents/rules/antigravity-rgt-rules.md`.
pub const ANTIGRAVITY_RULES: &str = "# RGT Integration\n\
    RGT tracks numeric and date provenance for AI coding agents.\n\
    \n\
    ## Recording Values\n\
    After reading a data file containing numbers or dates, record its values:\n\
    - `rgt record <file>` — extracts and tracks numeric/date values from file content\n\
    - Run `rgt status` to check what's tracked\n\
    \n\
    ## Recording Derivations\n\
    After computing a derived value from tracked root nodes:\n\
    - `rgt derive --parents <id1>,<id2> --operation EXPRESSION --expression \"a - b\" --result <val>`\n\
    - The derivation is verified before recording; wrong results are rejected (exit 1)\n\
    \n\
    ## Inspecting the Graph\n\
    - `rgt status` — see all tracked nodes and staleness state\n\
    - `rgt query <node_id>` — trace provenance lineage for a value\n\
    - `rgt graph` — export the dependency graph as text or mermaid\n\
    \n\
    Run `rgt --help` for all available commands.\n";

/// Kilo rules file, written to `.kilocode/rules/rgt-rules.md`.
pub const KILO_RULES: &str = "# RGT Integration\n\
    RGT tracks numeric and date provenance for AI coding agents.\n\
    \n\
    ## Recording Values\n\
    After reading a data file containing numbers or dates, record its values:\n\
    - `rgt record <file>` — extracts and tracks numeric/date values from file content\n\
    - Run `rgt status` to check what's tracked\n\
    \n\
    ## Recording Derivations\n\
    After computing a derived value from tracked root nodes:\n\
    - `rgt derive --parents <id1>,<id2> --operation EXPRESSION --expression \"a - b\" --result <val>`\n\
    - The derivation is verified before recording; wrong results are rejected (exit 1)\n\
    \n\
    ## Inspecting the Graph\n\
    - `rgt status` — see all tracked nodes and staleness state\n\
    - `rgt query <node_id>` — trace provenance lineage for a value\n\
    - `rgt graph` — export the dependency graph as text or mermaid\n\
    \n\
    Run `rgt --help` for all available commands.\n";

/// Mistral Vibe system prompt, written to `~/.vibe/prompts/rgt.md`.
pub const VIBE_PROMPT: &str = "# RGT Integration\n\
    RGT tracks numeric and date provenance for AI coding agents.\n\
    After reading data files, record extracted values with `rgt record <file>`.\n\
    After computing derived values, record them with `rgt derive --parents <ids> --operation EXPRESSION --expression \"a - b\" --result <val>`.\n\
    Derivations are verified before recording; wrong results are rejected.\n\
    Use `rgt status` to check provenance graph state.\n\
    Run `rgt --help` for all available commands.\n";

/// Copilot CLI instructions file, written to `~/.config/github-copilot/AGENTS.md`.
///
/// Copilot CLI (like Codex) consumes project `AGENTS.md` instructions; the
/// file written here carries the RGT usage instructions for the CLI agent.
pub const COPILOT_CLI_RULES: &str = "\n## RGT Integration\n\
    RGT tracks numeric and date provenance for AI coding agents.\n\
    \n\
    ### Recording Values\n\
    After reading a data file containing numbers or dates, record its values:\n\
    - `rgt record <file>` — extracts and tracks numeric/date values from file content\n\
    - Run `rgt status` to check what's tracked\n\
    \n\
    ### Recording Derivations\n\
    After computing a derived value from tracked root nodes:\n\
    - `rgt derive --parents <id1>,<id2> --operation EXPRESSION --expression \"a - b\" --result <val>`\n\
    - The derivation is verified before recording; wrong results are rejected (exit 1)\n\
    \n\
    ### Inspecting the Graph\n\
    - `rgt status` — see all tracked nodes and staleness state\n\
    - `rgt query <node_id>` — trace provenance lineage for a value\n\
    - `rgt graph` — export the dependency graph as text or mermaid\n\
    \n\
    Run `rgt --help` for all available commands.\n";

/// Marker used to identify RGT-managed sections/files for idempotent rewrites.
pub const RGT_MARKER: &str = "## RGT Integration";
