//! Embedded thin glue artifacts for plugin-based and rules-file agents.
//!
//! These constants are written verbatim by the installer to each agent's
//! config path at `rgt init` time (constitution I Agent-Side Glue Exemption).
//! They are thin delegates only: they parse the agent's native event JSON and
//! invoke the `rgt` CLI as a subprocess: zero business logic (constitution
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
    - `rgt record <file>`: extracts and tracks numeric/date values from file content\n\
    - Run `rgt status` to check what's tracked\n\
    \n\
    ### Recording Derivations\n\
    After computing a derived value from tracked root nodes:\n\
    - `rgt derive --parents <id1>,<id2> --operation EXPRESSION --expression \"a - b\" --result <val>`\n\
    - The derivation is verified before recording; wrong results are rejected (exit 1)\n\
    \n\
    ### Inspecting the Graph\n\
    - `rgt status`: see all tracked nodes and staleness state\n\
    - `rgt query <node_id>`: trace provenance lineage for a value\n\
    - `rgt graph`: export the dependency graph as text or mermaid\n\
    \n\
    Run `rgt --help` for all available commands.\n";

/// Antigravity rules file, written to `.agents/rules/antigravity-rgt-rules.md`.
pub const ANTIGRAVITY_RULES: &str = "# RGT Integration\n\
    RGT tracks numeric and date provenance for AI coding agents.\n\
    \n\
    ## Recording Values\n\
    After reading a data file containing numbers or dates, record its values:\n\
    - `rgt record <file>`: extracts and tracks numeric/date values from file content\n\
    - Run `rgt status` to check what's tracked\n\
    \n\
    ## Recording Derivations\n\
    After computing a derived value from tracked root nodes:\n\
    - `rgt derive --parents <id1>,<id2> --operation EXPRESSION --expression \"a - b\" --result <val>`\n\
    - The derivation is verified before recording; wrong results are rejected (exit 1)\n\
    \n\
    ## Inspecting the Graph\n\
    - `rgt status`: see all tracked nodes and staleness state\n\
    - `rgt query <node_id>`: trace provenance lineage for a value\n\
    - `rgt graph`: export the dependency graph as text or mermaid\n\
    \n\
    Run `rgt --help` for all available commands.\n";

/// Kilo rules file, written to `.kilocode/rules/rgt-rules.md`.
pub const KILO_RULES: &str = "# RGT Integration\n\
    RGT tracks numeric and date provenance for AI coding agents.\n\
    \n\
    ## Recording Values\n\
    After reading a data file containing numbers or dates, record its values:\n\
    - `rgt record <file>`: extracts and tracks numeric/date values from file content\n\
    - Run `rgt status` to check what's tracked\n\
    \n\
    ## Recording Derivations\n\
    After computing a derived value from tracked root nodes:\n\
    - `rgt derive --parents <id1>,<id2> --operation EXPRESSION --expression \"a - b\" --result <val>`\n\
    - The derivation is verified before recording; wrong results are rejected (exit 1)\n\
    \n\
    ## Inspecting the Graph\n\
    - `rgt status`: see all tracked nodes and staleness state\n\
    - `rgt query <node_id>`: trace provenance lineage for a value\n\
    - `rgt graph`: export the dependency graph as text or mermaid\n\
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
    - `rgt record <file>`: extracts and tracks numeric/date values from file content\n\
    - Run `rgt status` to check what's tracked\n\
    \n\
    ### Recording Derivations\n\
    After computing a derived value from tracked root nodes:\n\
    - `rgt derive --parents <id1>,<id2> --operation EXPRESSION --expression \"a - b\" --result <val>`\n\
    - The derivation is verified before recording; wrong results are rejected (exit 1)\n\
    \n\
    ### Inspecting the Graph\n\
    - `rgt status`: see all tracked nodes and staleness state\n\
    - `rgt query <node_id>`: trace provenance lineage for a value\n\
    - `rgt graph`: export the dependency graph as text or mermaid\n\
    \n\
    Run `rgt --help` for all available commands.\n";

/// Marker used to identify RGT-managed sections/files for idempotent rewrites.
pub const RGT_MARKER: &str = "## RGT Integration";

/// End marker delimiting RGT-owned blocks in text/rules files. `rgt init --force`
/// replaces only the bytes between `RGT_MARKER` and this marker, so user content
/// below the block is preserved (findings §2.4).
pub const RGT_END_MARKER: &str = "<!-- /RGT Integration -->";

/// `RGT_END_MARKER` on its own line, for appending to block constants at write
/// time.
pub const RGT_END_MARKER_LINE: &str = "\n<!-- /RGT Integration -->\n";

/// Instruction telling agents how to record values from non-text sources.
///
/// The passive hook cannot see inside PDF/Excel/image files (no local text
/// extraction exists — findings §4), so the agent must report those values
/// explicitly. Scoped narrowly to non-text formats; plain-text sources are
/// already auto-captured. Spliced into every rules/instructions artifact RGT
/// writes (findings §6).
pub const NON_TEXT_SOURCE_INSTRUCTION: &str = "\n### Recording Values from Non-Text Sources (PDF, Excel, images, scanned documents)\n\
    RGT's passive hook can only capture plain-text file content — it cannot see inside PDF, Excel (.xlsx), image, or other binary files, even when you read them directly. This is a platform limitation (PDF/image interpretation happens inside the model itself, never as a local text file), not something RGT's hook can intercept.\n\
    \n\
    After reading a PDF, Excel, image, or other non-text source, you must report its values explicitly:\n\
    - Extract the numeric values and dates from what you just read.\n\
    - Pipe them to RGT in plain text form: `echo \"Amount: 1234.56\" | rgt record --stdin`\n\
    - (Alternative: write your extraction to an intermediate `.txt`/`.md` file — that file's own read/write will be captured automatically, since it is plain text.)\n\
    \n\
    Do **not** do this for plain-text, CSV, JSON, or Markdown files — those are already captured automatically; no action needed.\n";

/// Wraps a rules block so it ends with the non-text-source instruction followed
/// by the end marker (the instruction sits inside RGT's delimited block).
pub fn with_instruction(rules: &str) -> String {
    format!(
        "{}{}{}",
        rules, NON_TEXT_SOURCE_INSTRUCTION, RGT_END_MARKER_LINE
    )
}
