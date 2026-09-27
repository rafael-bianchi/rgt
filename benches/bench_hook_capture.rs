//! Manual, local latency and fail-open runner for passive hook capture.
//!
//! Run with: `cargo bench --bench bench_hook_capture -- /absolute/path/to/rgt`
//! This is intentionally a report-only runner; wall-clock results are not a CI gate.

use base64::Engine;
use rgt::hooks::glue;
use rgt::hooks::installer::direct_hook_command_for_platform;
use rgt::store::queries::{list_all_nodes, list_capture_associations};
use rgt::store::DbStore;
use rusqlite::{Connection, OpenFlags};
use serde_json::json;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

fn main() {
    let binary = std::env::args().nth(1).unwrap_or_else(|| {
        eprintln!("usage: bench_hook_capture <absolute-path-to-rgt>");
        std::process::exit(2);
    });
    let binary = std::fs::canonicalize(binary).unwrap_or_else(|error| {
        eprintln!("could not resolve RGT executable: {error}");
        std::process::exit(2);
    });
    let mut failures = 0;

    for size in [64 * 1024, 1024 * 1024] {
        let content = text_payload(size);
        let (dir, source) = project_with_source(&content);
        let event = completed_read(&source, &content);
        failures += report_direct(
            &binary,
            dir.path(),
            &format!("direct-success-{size}"),
            &event,
            1,
        );
        // Pi is not currently a qualifying capture dialect. Keep its plugin
        // timing case in a fresh project and assert that it adds no graph data.
        let (plugin_dir, plugin_source) = project_with_source(&content);
        failures += report_plugin(
            &binary,
            plugin_dir.path(),
            &format!("plugin-resultless-text-{size}"),
            &completed_read(&plugin_source, &content),
            0,
        );
        let opencode_content = format!("{} 42", "x".repeat(size - 3));
        let (opencode_dir, opencode_source) = project_with_source(&opencode_content);
        failures += report_opencode_plugin(
            &binary,
            opencode_dir.path(),
            &opencode_source,
            &opencode_content,
            size,
        );
        for (kind, event) in [
            (
                "no-op",
                json!({
                    "event":"PostToolUse", "tool_name":"read_file",
                    "tool_input":{"file_path":"/no/source"}, "padding":"x".repeat(size)
                }),
            ),
            ("malformed-shape", json!({"unexpected":"x".repeat(size)})),
        ] {
            let case_dir = tempfile::tempdir().expect("plugin no-op temp project");
            initialize_store(case_dir.path());
            failures += report_plugin(
                &binary,
                case_dir.path(),
                &format!("plugin-{kind}-{size}"),
                &event,
                0,
            );
        }
        if size == 1024 * 1024 {
            failures += report_pdf_envelope(&binary);
        }

        let (dir, source) = project_with_source(&content);
        let repeated = completed_read(&source, &content);
        let start = Instant::now();
        let first = run_direct(&binary, dir.path(), &repeated.to_string());
        let second = run_direct(&binary, dir.path(), &repeated.to_string());
        let count = graph_counts_or_exit(dir.path(), &format!("direct-repeated-{size}")).nodes;
        let elapsed = start.elapsed();
        let ok = elapsed < Duration::from_secs(1)
            && first.success
            && second.success
            && first.stdout.is_empty()
            && second.stdout.is_empty()
            && count == 1;
        println!(
            "direct-repeated-{size}: elapsed={:?} exit={}/{} neutral={} roots={count} {}",
            elapsed,
            first.code,
            second.code,
            first.stdout.is_empty() && second.stdout.is_empty(),
            if ok { "PASS" } else { "FAIL" }
        );
        failures += usize::from(!ok);

        for (kind, payload) in [
            (
                "no-op",
                json!({
                    "event":"PostToolUse","tool_name":"read_file",
                    "tool_input":{"file_path":"/no/source"},"padding":"x".repeat(size)
                })
                .to_string(),
            ),
            (
                "malformed",
                format!("{{{}", "x".repeat(size.saturating_sub(1))),
            ),
        ] {
            let case_dir = tempfile::tempdir().expect("temp project");
            initialize_store(case_dir.path());
            let start = Instant::now();
            let output = run_direct(&binary, case_dir.path(), &payload);
            let roots =
                graph_counts_or_exit(case_dir.path(), &format!("direct-{kind}-{size}")).nodes;
            let elapsed = start.elapsed();
            let ok = elapsed < Duration::from_secs(1)
                && output.success
                && output.stdout.is_empty()
                && roots == 0;
            println!(
                "direct-{kind}-{size}: input={}B elapsed={:?} exit={} neutral={} roots={roots} {}",
                payload.len(),
                elapsed,
                output.code,
                output.stdout.is_empty(),
                if ok { "PASS" } else { "FAIL" }
            );
            failures += usize::from(!ok);
        }
    }

    let oversized_dir = tempfile::tempdir().expect("temp project");
    initialize_store(oversized_dir.path());
    let oversized = format!("{{\"event\":\"PostToolUse\",\"tool_name\":\"read_file\",\"tool_input\":{{\"file_path\":\"/oversized\"}},\"tool_response\":{{\"content\":\"{}\"}}}}", "x".repeat(8 * 1024 * 1024 + 1));
    let start = Instant::now();
    let output = run_direct(&binary, oversized_dir.path(), &oversized);
    let count = graph_counts_or_exit(oversized_dir.path(), "direct-oversized").nodes;
    let elapsed = start.elapsed();
    let ok = elapsed < Duration::from_secs(1)
        && output.success
        && output.stdout.is_empty()
        && count == 0;
    println!(
        "direct-oversized: bytes={} elapsed={:?} exit={} neutral={} roots={count} {}",
        oversized.len(),
        elapsed,
        output.code,
        output.stdout.is_empty(),
        if ok { "PASS" } else { "FAIL" }
    );
    failures += usize::from(!ok);

    let plugin_oversized = json!({
        "event":"PostToolUse", "tool_name":"read_file",
        "tool_response":{"content":"x".repeat(8 * 1024 * 1024 + 1)}
    });
    failures += report_plugin(
        &binary,
        oversized_dir.path(),
        "plugin-oversized",
        &plugin_oversized,
        0,
    );

    let plugin_pdf = padded_pdf(768 * 1024);
    let plugin_pdf_base64 = base64::engine::general_purpose::STANDARD.encode(&plugin_pdf);
    let plugin_binary_dir = tempfile::tempdir().expect("plugin binary temp project");
    let plugin_pdf_path = plugin_binary_dir.path().join("source.pdf");
    std::fs::write(&plugin_pdf_path, &plugin_pdf).expect("write plugin PDF snapshot");
    let plugin_pdf_path = plugin_pdf_path.to_string_lossy().into_owned();
    initialize_store(plugin_binary_dir.path());
    let plugin_binary = json!({
        "event":"PostToolUse", "tool_name":"Read",
        "tool_input":{"file_path":plugin_pdf_path.clone()},
        "tool_response":{"type":"pdf","file":{"filePath":plugin_pdf_path.clone(),"base64":plugin_pdf_base64.clone(),"originalSize":plugin_pdf.len()}}
    });
    failures += report_plugin(
        &binary,
        plugin_binary_dir.path(),
        "plugin-binary-envelope-success-1MiB",
        &plugin_binary,
        0,
    );

    let plugin_binary_noop_dir = tempfile::tempdir().expect("plugin binary no-op project");
    initialize_store(plugin_binary_noop_dir.path());
    let plugin_binary_noop = json!({
        "event":"PostToolUse", "tool_name":"list_files",
        "tool_response":{"type":"pdf","file":{"base64":plugin_pdf_base64.clone()}}
    });
    failures += report_plugin(
        &binary,
        plugin_binary_noop_dir.path(),
        "plugin-binary-envelope-noop-1MiB",
        &plugin_binary_noop,
        0,
    );

    let plugin_binary_malformed_dir = tempfile::tempdir().expect("plugin malformed binary project");
    initialize_store(plugin_binary_malformed_dir.path());
    let plugin_binary_malformed = json!({
        "event":"PostToolUse", "tool_name":"Read",
        "tool_input":{"file_path":"/missing.pdf"},
        "tool_response":{"type":"pdf","file":{"base64":"!".repeat(1024 * 1024)}}
    });
    failures += report_plugin(
        &binary,
        plugin_binary_malformed_dir.path(),
        "plugin-binary-envelope-malformed-1MiB",
        &plugin_binary_malformed,
        0,
    );

    let plugin_binary_repeated_dir = tempfile::tempdir().expect("plugin repeated binary project");
    let repeated_pdf_path = plugin_binary_repeated_dir.path().join("source.pdf");
    std::fs::write(&repeated_pdf_path, &plugin_pdf).expect("write repeated plugin PDF snapshot");
    initialize_store(plugin_binary_repeated_dir.path());
    let plugin_binary_repeated = json!({
        "event":"PostToolUse", "tool_name":"Read",
        "tool_input":{"file_path":repeated_pdf_path},
        "tool_response":{"type":"pdf","file":{"filePath":repeated_pdf_path.to_string_lossy().into_owned(),"base64":plugin_pdf_base64.clone(),"originalSize":plugin_pdf.len()}}
    });
    failures += report_plugin(
        &binary,
        plugin_binary_repeated_dir.path(),
        "plugin-binary-envelope-repeated-1MiB",
        &plugin_binary_repeated,
        0,
    );

    let slow_dir = tempfile::tempdir().expect("temp project");
    initialize_store(slow_dir.path());
    let (elapsed, success, neutral, count) = run_slow_stdin(&binary, slow_dir.path());
    let ok = success && neutral && count == 0 && elapsed < Duration::from_secs(1);
    println!("direct-stalled-hook-no-eof: elapsed={elapsed:?} exit-success={success} neutral={neutral} roots={count} {}",
        if ok {"PASS"} else {"FAIL"});
    failures += usize::from(!ok);

    failures += report_plugin_stalled_child();
    failures += report_direct_stalled_process();
    if failures > 0 {
        std::process::exit(1);
    }
}

struct HookOutput {
    success: bool,
    code: String,
    stdout: Vec<u8>,
}

fn run_direct(binary: &Path, dir: &Path, input: &str) -> HookOutput {
    run_direct_as(binary, dir, input, "codex")
}

fn run_direct_as(binary: &Path, dir: &Path, input: &str, agent: &str) -> HookOutput {
    let mut child = direct_command_process(binary, dir, agent)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("launch direct hook");
    child
        .stdin
        .take()
        .unwrap()
        .write_all(input.as_bytes())
        .unwrap();
    let output = child.wait_with_output().expect("wait for direct hook");
    HookOutput {
        success: output.status.success(),
        code: output
            .status
            .code()
            .map_or_else(|| "signal".into(), |code| code.to_string()),
        stdout: output.stdout,
    }
}

fn direct_command_process(binary: &Path, dir: &Path, agent: &str) -> Command {
    #[cfg(unix)]
    let mut command = {
        let hook = direct_hook_command_for_platform(binary, "post", agent, false);
        let mut command = Command::new("sh");
        command.args(["-c", &hook]);
        command
    };
    #[cfg(windows)]
    let mut command = {
        let hook = direct_hook_command_for_platform(binary, "post", agent, true);
        let mut parts = hook.split_whitespace();
        let executable = parts.next().expect("generated PowerShell hook executable");
        let mut command = Command::new(executable);
        command.args(parts);
        command
    };
    #[cfg(not(any(unix, windows)))]
    let mut command = {
        let mut command = Command::new(binary);
        command.args(["hook", "post", "--agent", agent, "--rgt-managed"]);
        command
    };
    command.current_dir(dir);
    command
}

fn report_pdf_envelope(binary: &Path) -> usize {
    let pdf = padded_pdf(768 * 1024);
    let dir = tempfile::tempdir().expect("PDF temp project");
    let path = dir.path().join("sample.pdf");
    std::fs::write(&path, &pdf).expect("write exact PDF snapshot");
    initialize_store(dir.path());
    let encoded = base64::engine::general_purpose::STANDARD.encode(&pdf);
    let event = json!({
        "event":"PostToolUse",
        "tool_name":"Read",
        "tool_input":{"file_path":path},
        "tool_response":{"type":"pdf","file":{"filePath":path,"base64":encoded,"originalSize":pdf.len()}}
    });
    let input = event.to_string();
    let start = Instant::now();
    let output = run_direct_as(binary, dir.path(), &input, "claude-code");
    let elapsed = start.elapsed();
    let roots = graph_counts_or_exit(dir.path(), "direct-pdf-binary-envelope-1MiB").nodes;
    let response = String::from_utf8_lossy(&output.stdout);
    let ok = elapsed < Duration::from_secs(1)
        && output.success
        && roots == 1
        && response.contains("additionalContext");
    println!(
        "direct-pdf-binary-envelope-1MiB: input={}B pdf={}B elapsed={:?} exit={} roots={roots} {}",
        input.len(),
        pdf.len(),
        elapsed,
        output.code,
        if ok { "PASS" } else { "FAIL" }
    );
    let mut failures = usize::from(!ok);

    let start = Instant::now();
    let repeated = run_direct_as(binary, dir.path(), &input, "claude-code");
    let elapsed = start.elapsed();
    let roots = graph_counts_or_exit(dir.path(), "direct-pdf-binary-envelope-repeated").nodes;
    let repeated_response = String::from_utf8_lossy(&repeated.stdout);
    let repeated_ok = elapsed < Duration::from_secs(1)
        && repeated.success
        && roots == 1
        && repeated_response.contains("additionalContext");
    println!(
        "direct-pdf-binary-envelope-repeated: elapsed={elapsed:?} exit={} roots={roots} {}",
        repeated.code,
        if repeated_ok { "PASS" } else { "FAIL" }
    );
    failures += usize::from(!repeated_ok);

    let noop_dir = tempfile::tempdir().expect("PDF no-op temp project");
    initialize_store(noop_dir.path());
    let noop_event = json!({
        "event":"PostToolUse", "tool_name":"Read",
        "tool_input":{"file_path":"/no/pdf"},
        "tool_response":{"type":"pdf","file":{"filePath":"/no/pdf","base64":"!".repeat(1024 * 1024)}}
    })
    .to_string();
    let start = Instant::now();
    let noop = run_direct_as(binary, noop_dir.path(), &noop_event, "claude-code");
    let elapsed = start.elapsed();
    let roots = graph_counts_or_exit(noop_dir.path(), "direct-pdf-binary-envelope-noop-1MiB").nodes;
    let noop_ok =
        elapsed < Duration::from_secs(1) && noop.success && noop.stdout.is_empty() && roots == 0;
    println!(
        "direct-pdf-binary-envelope-noop-1MiB: input={}B elapsed={elapsed:?} exit={} roots={roots} {}",
        noop_event.len(),
        noop.code,
        if noop_ok { "PASS" } else { "FAIL" }
    );
    failures += usize::from(!noop_ok);

    let malformed_dir = tempfile::tempdir().expect("malformed PDF temp project");
    initialize_store(malformed_dir.path());
    let malformed = format!(
        "{{\"event\":\"PostToolUse\",\"tool_name\":\"Read\",\"tool_response\":{{\"type\":\"pdf\",\"file\":{{\"base64\":\"{}\"}}",
        "A".repeat(1024 * 1024)
    );
    let start = Instant::now();
    let malformed_output = run_direct_as(binary, malformed_dir.path(), &malformed, "claude-code");
    let elapsed = start.elapsed();
    let roots = graph_counts_or_exit(
        malformed_dir.path(),
        "direct-pdf-binary-envelope-malformed-1MiB",
    )
    .nodes;
    let malformed_ok = elapsed < Duration::from_secs(1)
        && malformed_output.success
        && malformed_output.stdout.is_empty()
        && roots == 0;
    println!(
        "direct-pdf-binary-envelope-malformed-1MiB: input={}B elapsed={elapsed:?} exit={} roots={roots} {}",
        malformed.len(),
        malformed_output.code,
        if malformed_ok { "PASS" } else { "FAIL" }
    );
    failures += usize::from(!malformed_ok);
    failures
}

fn padded_pdf(padding_bytes: usize) -> Vec<u8> {
    let text = "BT /F1 12 Tf 72 720 Td (Amount 42) Tj ET";
    let stream = format!("{}{}", " ".repeat(padding_bytes), text);
    let objects = [
        "<< /Type /Catalog /Pages 2 0 R >>".to_string(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_string(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R /Resources << /Font << /F1 5 0 R >> >> >>".to_string(),
        format!("<< /Length {} >>\nstream\n{}\nendstream", stream.len(), stream),
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_string(),
    ];
    let mut pdf = b"%PDF-1.4\n".to_vec();
    let mut offsets = Vec::new();
    for (index, object) in objects.iter().enumerate() {
        offsets.push(pdf.len());
        pdf.extend_from_slice(format!("{} 0 obj\n{}\nendobj\n", index + 1, object).as_bytes());
    }
    let xref = pdf.len();
    pdf.extend_from_slice(format!("xref\n0 {}\n", objects.len() + 1).as_bytes());
    pdf.extend_from_slice(b"0000000000 65535 f \n");
    for offset in offsets {
        pdf.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    pdf.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
            objects.len() + 1
        )
        .as_bytes(),
    );
    pdf
}

fn report_direct(
    binary: &Path,
    dir: &Path,
    label: &str,
    event: &serde_json::Value,
    expected_roots: usize,
) -> usize {
    let start = Instant::now();
    let output = run_direct(binary, dir, &event.to_string());
    let elapsed = start.elapsed();
    let count = graph_counts_or_exit(dir, label).nodes;
    let ok = elapsed < Duration::from_secs(1)
        && output.success
        && output.stdout.is_empty()
        && count == expected_roots;
    println!(
        "{label}: input={}B elapsed={:?} exit={} neutral={} roots={count} {}",
        event.to_string().len(),
        elapsed,
        output.code,
        output.stdout.is_empty(),
        if ok { "PASS" } else { "FAIL" }
    );
    usize::from(!ok)
}

fn report_plugin(
    binary: &Path,
    dir: &Path,
    label: &str,
    event: &serde_json::Value,
    expected_roots: usize,
) -> usize {
    if !command_exists("node") {
        println!("{label}: INCOMPLETE (Node.js is unavailable; plugin coverage was not run)");
        return 1;
    }
    let plugin_path = dir.join("rgt-plugin-bench.mjs");
    std::fs::write(&plugin_path, glue::pi_extension(&binary.to_string_lossy()))
        .expect("write generated Pi plugin wrapper");
    let script = r#"const fs=require('node:fs'); const {pathToFileURL}=require('node:url');
(async()=>{const input=JSON.parse(fs.readFileSync(0,'utf8')); const mod=await import(pathToFileURL(process.argv[1]).href);
const before=JSON.stringify(input);
const firstStart=process.hrtime.bigint(); const firstResult=await mod.onToolCall(input); const firstMs=Number(process.hrtime.bigint()-firstStart)/1e6;
const unchangedAfterFirst=before===JSON.stringify(input);
const secondStart=process.hrtime.bigint(); const secondResult=await mod.onToolCall(input); const secondMs=Number(process.hrtime.bigint()-secondStart)/1e6;
process.stdout.write(JSON.stringify({first_ms:firstMs,second_ms:secondMs,neutral:firstResult===undefined&&secondResult===undefined,unchanged:unchangedAfterFirst&&before===JSON.stringify(input)}));})().catch(()=>process.exit(1));"#;
    let input = event.to_string();
    let start = Instant::now();
    let mut child = Command::new("node")
        .args(["-e", script, plugin_path.to_str().unwrap()])
        .current_dir(dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("launch plugin process wrapper");
    child
        .stdin
        .take()
        .unwrap()
        .write_all(input.as_bytes())
        .unwrap();
    let output = child
        .wait_with_output()
        .expect("wait for plugin process wrapper");
    let elapsed = start.elapsed();
    let graph = graph_counts_or_exit(dir, label);
    let count = graph.nodes;
    let timings: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap_or_default();
    let first_ms = timings["first_ms"].as_f64().unwrap_or(f64::INFINITY);
    let second_ms = timings["second_ms"].as_f64().unwrap_or(f64::INFINITY);
    let neutral = timings["neutral"].as_bool() == Some(true);
    let unchanged = timings["unchanged"].as_bool() == Some(true);
    let associations = graph.associations;
    let ok = elapsed < Duration::from_secs(1)
        && first_ms < 1_000.0
        && second_ms < 1_000.0
        && neutral
        && unchanged
        && output.status.success()
        && count == expected_roots
        && associations == expected_roots;
    println!(
        "{label}: input={}B cold-first={first_ms:.1}ms repeated={second_ms:.1}ms launch-to-response={:?} exit={} neutral={neutral} unchanged={unchanged} roots={count} associations={associations} {}",
        input.len(),
        elapsed,
        output.status.code().unwrap_or(-1),
        if ok { "PASS" } else { "FAIL" }
    );
    usize::from(!ok)
}

fn report_opencode_plugin(
    binary: &Path,
    dir: &Path,
    source: &Path,
    content: &str,
    size: usize,
) -> usize {
    let label = format!("opencode-plugin-success-{size}");
    if !command_exists("node") {
        println!("{label}: INCOMPLETE (Node.js is unavailable)");
        return 1;
    }
    let plugin = dir.join("rgt-opencode-bench.mjs");
    std::fs::write(&plugin, glue::opencode_plugin(&binary.to_string_lossy()))
        .expect("write generated OpenCode plugin");
    let event = json!({
        "input": {"tool":"read", "args":{"filePath":source}},
        "output": {
            "output":"completed read",
            "metadata":{"truncated":false,"display":{
                "type":"file","path":source,"text":content,
                "lineStart":1,"lineEnd":1,"totalLines":1,"truncated":false
            }}
        }
    });
    let script = r#"const fs=require('node:fs'); const {pathToFileURL}=require('node:url');
(async()=>{const event=JSON.parse(fs.readFileSync(0,'utf8')); const mod=await import(pathToFileURL(process.argv[1]).href);
const plugin=await mod.RgtPlugin({directory:process.cwd()}); const before=JSON.stringify(event);
const firstStart=process.hrtime.bigint(); const firstResult=await plugin['tool.execute.after'](event.input,event.output); const firstMs=Number(process.hrtime.bigint()-firstStart)/1e6;
const secondStart=process.hrtime.bigint(); const secondResult=await plugin['tool.execute.after'](event.input,event.output); const secondMs=Number(process.hrtime.bigint()-secondStart)/1e6;
process.stdout.write(JSON.stringify({first_ms:firstMs,second_ms:secondMs,neutral:firstResult===undefined&&secondResult===undefined,unchanged:before===JSON.stringify(event)}));})().catch(()=>process.exit(1));"#;
    let input = event.to_string();
    let start = Instant::now();
    let mut child = Command::new("node")
        .args(["-e", script, plugin.to_str().unwrap()])
        .current_dir(dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("launch OpenCode plugin wrapper");
    child
        .stdin
        .take()
        .unwrap()
        .write_all(input.as_bytes())
        .unwrap();
    let output = child
        .wait_with_output()
        .expect("wait for OpenCode plugin wrapper");
    let elapsed = start.elapsed();
    let timings: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap_or_default();
    let first_ms = timings["first_ms"].as_f64().unwrap_or(f64::INFINITY);
    let second_ms = timings["second_ms"].as_f64().unwrap_or(f64::INFINITY);
    let neutral = timings["neutral"].as_bool() == Some(true);
    let unchanged = timings["unchanged"].as_bool() == Some(true);
    let graph_ok = opencode_graph_evidence(dir);
    let ok = output.status.success()
        && elapsed < Duration::from_secs(1)
        && first_ms < 1_000.0
        && second_ms < 1_000.0
        && neutral
        && unchanged
        && graph_ok;
    println!(
        "{label}: input={}B cold-first={first_ms:.1}ms repeated={second_ms:.1}ms launch-to-response={elapsed:?} exit={} neutral={neutral} unchanged={unchanged} value=42 line=1 capture-agent=opencode graph-ok={graph_ok} {}",
        input.len(), output.status.code().unwrap_or(-1), if ok { "PASS" } else { "FAIL" }
    );
    usize::from(!ok)
}

fn opencode_graph_evidence(dir: &Path) -> bool {
    let db_path = dir.join(".rgt/store.db");
    let Ok(conn) = Connection::open_with_flags(db_path, OpenFlags::SQLITE_OPEN_READ_ONLY) else {
        return false;
    };
    let Ok(nodes) = list_all_nodes(&conn) else {
        return false;
    };
    let Ok(associations) = list_capture_associations(&conn) else {
        return false;
    };
    nodes.len() == 1
        && nodes[0].value.to_string_repr() == "42"
        && nodes[0].line_number == Some(1)
        && associations == vec![(nodes[0].id.clone(), "opencode".to_string())]
}

fn report_plugin_stalled_child() -> usize {
    if !command_exists("node") {
        println!("plugin-stalled-child: INCOMPLETE (Node.js is unavailable; stalled plugin coverage was not run)");
        return 1;
    }
    #[cfg(not(unix))]
    {
        println!("plugin-stalled-child: INCOMPLETE (the local executable stub requires a POSIX host; stalled plugin coverage was not run)");
        1
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().expect("stalled plugin temp project");
        let stalled = dir.path().join("rgt-stalled");
        std::fs::write(
            &stalled,
            "#!/usr/bin/env node\nconst fs=require('node:fs'); const id=process.env.RGT_BENCH_STALL_ID; if (!['first','second'].includes(id)) process.exit(2); fs.writeFileSync(`plugin-stalled-${id}.started`,'started'); fs.writeFileSync(`plugin-stalled-${id}.pid`,String(process.pid)); setTimeout(()=>{},2000);\n",
        )
            .expect("write stalled child stub");
        let mut permissions = std::fs::metadata(&stalled).unwrap().permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(&stalled, permissions).unwrap();
        let plugin = dir.path().join("rgt-plugin-stalled.mjs");
        std::fs::write(&plugin, glue::pi_extension(&stalled.to_string_lossy()))
            .expect("write generated Pi plugin wrapper");
        initialize_store(dir.path());
        let script = r#"const {pathToFileURL}=require('node:url');
(async()=>{const mod=await import(pathToFileURL(process.argv[1]).href); const input={event:'PostToolUse',tool_name:'read_file'};
const before=JSON.stringify(input);
process.env.RGT_BENCH_STALL_ID='first';
const firstStart=process.hrtime.bigint(); const firstResult=await mod.onToolCall(input); const firstMs=Number(process.hrtime.bigint()-firstStart)/1e6;
process.env.RGT_BENCH_STALL_ID='second';
const secondStart=process.hrtime.bigint(); const secondResult=await mod.onToolCall(input); const secondMs=Number(process.hrtime.bigint()-secondStart)/1e6;
process.stdout.write(JSON.stringify({first_ms:firstMs,second_ms:secondMs,neutral:firstResult===undefined&&secondResult===undefined,unchanged:before===JSON.stringify(input)}));})().catch(()=>process.exit(1));"#;
        let start = Instant::now();
        let output = Command::new("node")
            .args(["-e", script, plugin.to_str().unwrap()])
            .current_dir(dir.path())
            .output()
            .expect("launch generated plugin timeout check");
        let elapsed = start.elapsed();
        let timings: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap_or_default();
        let first_ms = timings["first_ms"].as_f64().unwrap_or(f64::INFINITY);
        let second_ms = timings["second_ms"].as_f64().unwrap_or(f64::INFINITY);
        let neutral = timings["neutral"].as_bool() == Some(true);
        let unchanged = timings["unchanged"].as_bool() == Some(true);
        let graph = graph_counts_or_exit(dir.path(), "plugin-stalled-child");
        let roots = graph.nodes;
        let associations = graph.associations;
        let first_child = stalled_child_evidence(dir.path(), "first");
        let second_child = stalled_child_evidence(dir.path(), "second");
        let children = [first_child, second_child];
        let both_children_verified = stalled_children_verified(&children, 2);
        let missing_second_rejected = !stalled_children_verified(&children[..1], 2);
        let ok = output.status.success()
            && first_ms < 1_000.0
            && second_ms < 1_000.0
            && neutral
            && unchanged
            && roots == 0
            && associations == 0
            && both_children_verified
            && missing_second_rejected;
        println!(
            "plugin-stalled-child: cold-first={first_ms:.1}ms repeated={second_ms:.1}ms total={elapsed:?} timeout=800ms neutral={neutral} unchanged={unchanged} roots={roots} associations={associations} first-started={} first-terminated={} second-started={} second-terminated={} both-pids-recorded={} missing-second-rejected={missing_second_rejected} {}",
            children[0].started,
            children[0].terminated,
            children[1].started,
            children[1].terminated,
            children[0].pid.is_some() && children[1].pid.is_some(),
            if ok { "PASS" } else { "FAIL" }
        );
        usize::from(!ok)
    }
}

#[cfg(unix)]
struct StalledChildEvidence {
    started: bool,
    pid: Option<u32>,
    terminated: bool,
}

#[cfg(unix)]
fn stalled_child_evidence(dir: &Path, id: &str) -> StalledChildEvidence {
    let started = std::fs::read_to_string(dir.join(format!("plugin-stalled-{id}.started")))
        .is_ok_and(|value| value == "started");
    let pid = std::fs::read_to_string(dir.join(format!("plugin-stalled-{id}.pid")))
        .ok()
        .and_then(|value| value.trim().parse::<u32>().ok());
    let terminated = pid.is_some_and(|pid| {
        Command::new("kill")
            .args(["-0", &pid.to_string()])
            .stderr(Stdio::null())
            .status()
            .is_ok_and(|status| !status.success())
    });
    StalledChildEvidence {
        started,
        pid,
        terminated,
    }
}

#[cfg(unix)]
fn stalled_children_verified(children: &[StalledChildEvidence], expected: usize) -> bool {
    children.len() == expected
        && children
            .iter()
            .all(|child| child.started && child.terminated && child.pid.is_some())
}

fn report_direct_stalled_process() -> usize {
    #[cfg(not(unix))]
    {
        println!(
            "direct-stalled-child: INCOMPLETE (manual executable stub requires a POSIX host; stalled direct coverage was not run)"
        );
        1
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().expect("stalled direct hook temp project");
        initialize_store(dir.path());
        let stalled = dir.path().join("rgt-stalled");
        std::fs::write(
            &stalled,
            "#!/bin/sh\n: > direct-stalled-started\nprintf '%s\\n' \"$$\" > direct-stalled-pid\nexec sleep 2\n",
        )
        .expect("write stalled direct-hook executable");
        let mut permissions = std::fs::metadata(&stalled).unwrap().permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(&stalled, permissions).unwrap();
        let hook = direct_hook_command_for_platform(&stalled, "post", "codex", false);
        let start = Instant::now();
        let mut child = Command::new("sh")
            .args(["-c", &hook])
            .current_dir(dir.path())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("launch stalled direct hook command");
        child
            .stdin
            .take()
            .unwrap()
            .write_all(b"{\"event\":\"PostToolUse\"}")
            .expect("write direct hook event");
        let output = child
            .wait_with_output()
            .expect("wait for bounded direct hook wrapper");
        let elapsed = start.elapsed();
        let roots = graph_counts_or_exit(dir.path(), "direct-stalled-child").nodes;
        let started = dir.path().join("direct-stalled-started").exists();
        let pid =
            std::fs::read_to_string(dir.path().join("direct-stalled-pid")).unwrap_or_default();
        let terminated = pid.trim().parse::<u32>().is_ok_and(|pid| {
            Command::new("kill")
                .args(["-0", &pid.to_string()])
                .stderr(Stdio::null())
                .status()
                .is_ok_and(|status| !status.success())
        });
        let ok = elapsed < Duration::from_secs(1)
            && output.status.success()
            && output.stdout.is_empty()
            && roots == 0
            && started
            && terminated;
        println!(
            "direct-stalled-child: elapsed={elapsed:?} exit={} neutral={} roots={roots} started={started} terminated={terminated} {}",
            output.status.code().unwrap_or(-1),
            output.stdout.is_empty(),
            if ok { "PASS" } else { "FAIL" }
        );
        usize::from(!ok)
    }
}

fn run_slow_stdin(binary: &Path, dir: &Path) -> (Duration, bool, bool, usize) {
    let mut child = direct_command_process(binary, dir, "codex")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("launch slow-input hook");
    let mut stdin = child.stdin.take().unwrap();
    stdin.write_all(b"{").unwrap();
    let start = Instant::now();
    loop {
        if child.try_wait().expect("poll slow-input hook").is_some() {
            break;
        }
        if start.elapsed() > Duration::from_secs(1) {
            let _ = child.kill();
            break;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    drop(stdin);
    let output = child
        .wait_with_output()
        .expect("collect slow-input hook output");
    (
        start.elapsed(),
        output.status.success(),
        output.stdout.is_empty(),
        graph_counts_or_exit(dir, "direct-stalled-hook-no-eof").nodes,
    )
}

fn completed_read(path: &Path, content: &str) -> serde_json::Value {
    json!({"event":"PostToolUse","tool_name":"read_file","tool_input":{"file_path":path},"tool_response":{"content":content}})
}

fn text_payload(size: usize) -> String {
    let mut content = "x".repeat(size.saturating_sub(4));
    content.push_str(" 42\n");
    content
}

fn project_with_source(content: &str) -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().expect("temp project");
    let source = dir.path().join("source.txt");
    std::fs::write(&source, content).expect("write source snapshot");
    initialize_store(dir.path());
    (dir, source)
}

fn initialize_store(dir: &Path) {
    DbStore::open_in_project(dir).expect("initialize local store");
}

struct GraphCounts {
    nodes: usize,
    associations: usize,
}

fn graph_counts(dir: &Path) -> Result<GraphCounts, String> {
    let db_path = dir.join(".rgt").join("store.db");
    if !db_path.is_file() {
        return Err(format!(
            "existing project store is missing at {}",
            db_path.display()
        ));
    }
    let conn = Connection::open_with_flags(&db_path, OpenFlags::SQLITE_OPEN_READ_ONLY).map_err(
        |error| {
            format!(
                "could not open existing store {}: {error}",
                db_path.display()
            )
        },
    )?;
    let nodes = list_all_nodes(&conn)
        .map_err(|error| {
            format!(
                "could not query tracked nodes in {}: {error}",
                db_path.display()
            )
        })?
        .len();
    let associations = list_capture_associations(&conn)
        .map_err(|error| {
            format!(
                "could not query capture associations in {}: {error}",
                db_path.display()
            )
        })?
        .len();
    Ok(GraphCounts {
        nodes,
        associations,
    })
}

fn graph_counts_or_exit(dir: &Path, label: &str) -> GraphCounts {
    graph_counts(dir).unwrap_or_else(|error| {
        eprintln!("{label}: FAIL graph read: {error}");
        std::process::exit(1);
    })
}

fn command_exists(command: &str) -> bool {
    Command::new(command)
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}
