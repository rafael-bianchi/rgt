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
/// Written to `~/.config/opencode/plugins/` (global) or `.opencode/plugins/`
/// (project). Only captures post-tool events. `rgt_path` is the absolute path
/// to the CLI binary, injected at `rgt init` time (FR-003).
pub fn opencode_plugin(rgt_path: &str) -> String {
    let exe = js_string_literal(rgt_path);
    format!(
        r#"import {{ spawnSync }} from "node:child_process";

// RGT-managed integration. rgt init may safely replace this file.
export const RgtPlugin = async ({{ directory }}) => ({{
  "tool.execute.after": async (input, output) => {{
    try {{
      spawnSync("{exe}", ["hook", "post", "--agent", "opencode", "--rgt-managed"], {{
        cwd: directory,
        input: JSON.stringify({{
          event: "tool.execute.after",
          tool: input.tool,
          args: input.args,
          result: output,
        }}),
        encoding: "utf-8",
        timeout: 800,
      }});
    }} catch {{
      // fail-open: never block or alter the agent's tool call
    }}
  }},
}});
"#
    )
}

/// Pi TypeScript extension (`rgt.ts`).
///
/// Written to `.pi/extensions/` (project) or `~/.pi/agent/extensions/`
/// (`--global`). Only captures post-tool events. `rgt_path` is the absolute
/// path to the CLI binary, injected at `rgt init` time (FR-003).
pub fn pi_extension(rgt_path: &str) -> String {
    let exe = js_string_literal(rgt_path);
    format!(
        r#"// RGT-managed integration. rgt init may safely replace this file.
import {{ spawnSync }} from "node:child_process";

export const extension = {{
  name: "rgt",
  async setup() {{}},
}};

export async function onToolCall(input) {{
  try {{
    spawnSync("{exe}", ["hook", "post", "--agent", "pi", "--rgt-managed"], {{
      input: JSON.stringify(input),
      encoding: "utf-8",
      timeout: 800,
    }});
  }} catch {{
    // fail-open: never block or alter the agent's tool call
  }}
}}
"#
    )
}

/// Hermes Python plugin (`plugin.py`).
///
/// Written to `~/.hermes/plugins/rgt/` (global) or `.hermes/plugins/rgt/`
/// (project) and enabled via `plugins.enabled` in the Hermes config. Only
/// captures post-tool events. `rgt_path` is the absolute path to the CLI
/// binary, injected at `rgt init` time (FR-003); it replaces the PATH lookup.
pub fn hermes_plugin(rgt_path: &str) -> String {
    let exe = py_string_literal(rgt_path);
    format!(
        r#"# RGT-managed integration. rgt init may safely replace this file.
import json
import subprocess


def pre_tool_call(args):
    _capture(args)


def post_tool_call(args):
    _capture(args)


def _capture(args):
    try:
        subprocess.run(
            [{exe}, "hook", "post", "--agent", "hermes", "--rgt-managed"],
            input=json.dumps(args).encode("utf-8"),
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
            timeout=0.8,
        )
    except Exception:
        # fail-open: never block or alter the agent's tool call
        pass
"#
    )
}

/// Escapes `s` for embedding inside a double-quoted JavaScript string literal.
fn js_string_literal(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

/// Escapes `s` for embedding inside a double-quoted Python string literal.
fn py_string_literal(s: &str) -> String {
    format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
}

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

/// Instructions for reading values when automatic capture is unavailable.
/// Support varies by client and read path; an installed hook or text file is
/// not evidence that a successful result reached RGT.
pub const NON_TEXT_SOURCE_INSTRUCTION: &str = "\n### Recording Values When Automatic Capture Is Unavailable\n\
    RGT automatically captures values only when a supported client delivers a successful completed read with full source content, and the bytes and source lines match the current file. See the compatibility support matrix for each client's verified read paths. A configured hook or plain-text file alone does not prove automatic capture.\n\
    For PDF, Excel, images, scanned documents, or any read path without verified full-content capture, report the extracted values explicitly:\n\
    - Extract the numeric values and dates from what you just read.\n\
    - Pipe them to RGT in plain text form: `echo \"Amount: 1234.56\" | rgt record --stdin`\n\
    - (Alternative: write your extraction to an intermediate `.txt`/`.md` file, then run `rgt record <file>` if automatic capture is not verified for your client.)\n\
    Use `rgt record <file>` whenever the support matrix does not list your successful read path as verified automatic capture.\n";

/// Wraps a rules block so it ends with the non-text-source instruction followed
/// by the end marker (the instruction sits inside RGT's delimited block).
pub fn with_instruction(rules: &str) -> String {
    format!(
        "{}{}{}",
        rules, NON_TEXT_SOURCE_INSTRUCTION, RGT_END_MARKER_LINE
    )
}
