use chrono::DateTime;
use regex::Regex;
use serde_json::{json, Map, Value};

const SCHEMA: &str = include_str!("../../docs/compatibility/evidence.schema.json");
const CONTRACT_SCHEMA: &str = include_str!(
    "../../specs/044-verify-harness-compatibility/contracts/compatibility-evidence.schema.json"
);
const MATRIX: &str = include_str!("../fixtures/hooks/synthetic-events.json");
const SUPPORT_MATRIX: &str = include_str!("../../docs/compatibility/support-matrix.md");
const OBSERVATIONS: &str = include_str!("../../docs/compatibility/evidence/observations.json");

fn good_success() -> Value {
    json!({
        "evidence_id": "synthetic-success-1",
        "observed_at": "2026-09-26T10:30:00Z",
        "surface_id": "claude-code",
        "read_path_id": "claude-read-post-content",
        "canonical_agent": "claude-code",
        "rgt_version": env!("CARGO_PKG_VERSION"),
        "client_version": "1.2.3",
        "platform": {"os": "macos", "architecture": "aarch64"},
        "method": "native-client",
        "registration_loaded": true,
        "event": {
            "phase": "post",
            "tool": "Read",
            "outcome": "success",
            "completion_evidence": "completed-result",
            "content_evidence": "full-snapshot-match"
        },
        "host_response": {"format": "none", "accepted": true, "tool_result_unchanged": true},
        "graph_result": {
            "values": [
                {"node_id": "node_raw_synthetic_42", "kind": "number", "value": "42", "source_line": 1},
                {"node_id": "node_raw_synthetic_date", "kind": "date", "value": "2026-09-26", "source_line": 2}
            ],
            "capture_agent": "claude-code"
        },
        "source_reference": "synthetic fixture test",
        "redacted": true
    })
}

fn good_noop() -> Value {
    json!({
        "evidence_id": "synthetic-noop-1",
        "observed_at": "2026-09-26T10:31:00Z",
        "surface_id": "claude-code",
        "read_path_id": "claude-read-post-content",
        "canonical_agent": "claude-code",
        "rgt_version": env!("CARGO_PKG_VERSION"),
        "client_version": "1.2.3",
        "platform": {"os": "macos", "architecture": "aarch64"},
        "method": "native-client",
        "registration_loaded": true,
        "event": {
            "phase": "post",
            "tool": "Read",
            "outcome": "no-op",
            "completion_evidence": "none",
            "content_evidence": "none"
        },
        "host_response": {"format": "none", "accepted": true, "tool_result_unchanged": true},
        "graph_result": {"values": [], "capture_agent": null},
        "source_reference": "synthetic fixture test",
        "redacted": true
    })
}

fn object<'a>(value: &'a Value, label: &str) -> Result<&'a Map<String, Value>, String> {
    value
        .as_object()
        .ok_or_else(|| format!("{label} must be an object"))
}

fn exact_keys(
    value: &Value,
    label: &str,
    required: &[&str],
    allowed: &[&str],
) -> Result<(), String> {
    let map = object(value, label)?;
    for key in required {
        if !map.contains_key(*key) {
            return Err(format!("{label} missing required property {key}"));
        }
    }
    for key in map.keys() {
        if !allowed.contains(&key.as_str()) {
            return Err(format!("{label} has unexpected property {key}"));
        }
    }
    Ok(())
}

fn string<'a>(value: &'a Value, label: &str) -> Result<&'a str, String> {
    let text = value
        .as_str()
        .ok_or_else(|| format!("{label} must be a string"))?;
    if text.is_empty() {
        return Err(format!("{label} must not be empty"));
    }
    Ok(text)
}

fn enum_value(value: &Value, label: &str, choices: &[&str]) -> Result<String, String> {
    let text = string(value, label)?;
    if !choices.contains(&text) {
        return Err(format!("{label} has unsupported value {text}"));
    }
    Ok(text.to_string())
}

fn validate_observation(value: &Value) -> Result<(), String> {
    const REQUIRED: &[&str] = &[
        "evidence_id",
        "observed_at",
        "surface_id",
        "read_path_id",
        "canonical_agent",
        "rgt_version",
        "client_version",
        "platform",
        "method",
        "registration_loaded",
        "event",
        "host_response",
        "graph_result",
        "source_reference",
        "redacted",
    ];
    const OPTIONAL: &[&str] = &["fixture_path", "notes"];
    let root = object(value, "observation")?;
    exact_keys(
        value,
        "observation",
        REQUIRED,
        &[REQUIRED, OPTIONAL].concat(),
    )?;
    for key in [
        "evidence_id",
        "canonical_agent",
        "rgt_version",
        "client_version",
        "source_reference",
    ] {
        string(&root[key], key)?;
    }
    let time = string(&root["observed_at"], "observed_at")?;
    DateTime::parse_from_rfc3339(time)
        .map_err(|_| "observed_at must be RFC3339 date-time".to_string())?;
    let slug = Regex::new(r"^[a-z0-9]+(?:-[a-z0-9]+)*$").unwrap();
    for key in ["surface_id", "read_path_id"] {
        if !slug.is_match(string(&root[key], key)?) {
            return Err(format!("{key} must match the lowercase slug pattern"));
        }
    }
    if !root["redacted"].as_bool().unwrap_or(false) {
        return Err("redacted must be true".into());
    }
    for key in OPTIONAL {
        if let Some(value) = root.get(*key) {
            if !value.is_string() {
                return Err(format!("{key} must be a string"));
            }
        }
    }
    enum_value(
        &root["method"],
        "method",
        &[
            "native-client",
            "fixture-replay",
            "vendor-documentation",
            "comparative-source",
        ],
    )?;
    let registration_loaded = root["registration_loaded"]
        .as_bool()
        .ok_or("registration_loaded must be boolean")?;

    let platform = &root["platform"];
    exact_keys(
        platform,
        "platform",
        &["os", "architecture"],
        &["os", "architecture"],
    )?;
    enum_value(
        &platform["os"],
        "platform.os",
        &["macos", "linux", "windows", "unknown"],
    )?;
    enum_value(
        &platform["architecture"],
        "platform.architecture",
        &["aarch64", "x86_64", "unknown"],
    )?;

    let event = &root["event"];
    exact_keys(
        event,
        "event",
        &[
            "phase",
            "tool",
            "outcome",
            "completion_evidence",
            "content_evidence",
        ],
        &[
            "phase",
            "tool",
            "outcome",
            "completion_evidence",
            "content_evidence",
        ],
    )?;
    let phase = enum_value(&event["phase"], "event.phase", &["pre", "post", "none"])?;
    let tool = string(&event["tool"], "event.tool")?;
    let outcome = enum_value(
        &event["outcome"],
        "event.outcome",
        &["success", "no-op", "failure", "canceled", "unknown"],
    )?;
    let completion = enum_value(
        &event["completion_evidence"],
        "event.completion_evidence",
        &["explicit-success", "completed-result", "none"],
    )?;
    let content = enum_value(
        &event["content_evidence"],
        "event.content_evidence",
        &[
            "full-snapshot-match",
            "path-only",
            "partial-or-filtered",
            "none",
        ],
    )?;

    let response = &root["host_response"];
    exact_keys(
        response,
        "host_response",
        &["format", "accepted", "tool_result_unchanged"],
        &["format", "accepted", "tool_result_unchanged"],
    )?;
    enum_value(
        &response["format"],
        "host_response.format",
        &["none", "json", "text", "unknown"],
    )?;
    let response_accepted = response["accepted"]
        .as_bool()
        .ok_or("host_response.accepted must be boolean")?;
    let result_unchanged = response["tool_result_unchanged"]
        .as_bool()
        .ok_or("host_response.tool_result_unchanged must be boolean")?;

    let graph = &root["graph_result"];
    exact_keys(
        graph,
        "graph_result",
        &["values", "capture_agent"],
        &["values", "capture_agent"],
    )?;
    let values = graph["values"]
        .as_array()
        .ok_or("graph_result.values must be an array")?;
    for (index, entry) in values.iter().enumerate() {
        exact_keys(
            entry,
            "graph_result.values[]",
            &["node_id", "kind", "value", "source_line"],
            &["node_id", "kind", "value", "source_line"],
        )?;
        string(&entry["node_id"], "graph_result.values[].node_id")?;
        enum_value(
            &entry["kind"],
            "graph_result.values[].kind",
            &["number", "date"],
        )?;
        string(&entry["value"], "graph_result.values[].value")?;
        let line = entry["source_line"]
            .as_u64()
            .ok_or("source_line must be an integer")?;
        if line == 0 {
            return Err(format!("source_line must be at least 1 at values[{index}]"));
        }
    }
    let capture_agent = &graph["capture_agent"];
    if !capture_agent.is_null()
        && string(capture_agent, "graph_result.capture_agent")?
            != root["canonical_agent"].as_str().unwrap()
    {
        return Err("graph_result.capture_agent must match canonical_agent".into());
    }

    let path = root["read_path_id"].as_str().unwrap();
    let canonical = root["canonical_agent"].as_str().unwrap();
    let surface_id = root["surface_id"].as_str().unwrap();
    let mapped_surface = rgt::hooks::registration::ClientSurface::by_id(surface_id)
        .is_some_and(|surface| surface.canonical_agent == canonical);
    let documented_tool = match (surface_id, path) {
        ("claude-code", "claude-read-post-content") => canonical == "claude-code" && tool == "Read",
        ("claude-code", "claude-bash-read-stdout") => canonical == "claude-code" && tool == "Bash",
        ("codex", "codex-post-tool-response") => {
            canonical == "codex" && ["read_file", "ReadFile", "Read"].contains(&tool)
        }
        ("opencode", "opencode-read-display-text") => canonical == "opencode" && tool == "read",
        ("windsurf", "windsurf-post-read-code-path-only") => {
            canonical == "windsurf" && tool == "post_read_code"
        }
        (_, "none") => true,
        _ => false,
    };
    if !mapped_surface {
        return Err(format!(
            "surface {surface_id} does not resolve to canonical agent {canonical}"
        ));
    }
    if !documented_tool {
        return Err(format!(
            "read path {path} does not belong to surface {surface_id} with tool {tool}"
        ));
    }
    if path == "none"
        && (!values.is_empty()
            || !capture_agent.is_null()
            || ["Read", "Bash", "read_file", "ReadFile", "post_read_code"].contains(&tool))
    {
        return Err("read_path_id none is reserved for guidance or non-read observations".into());
    }

    if ["no-op", "failure", "canceled"].contains(&outcome.as_str())
        && (!values.is_empty() || !capture_agent.is_null())
    {
        return Err("no-op/error observations cannot contain values or capture attribution".into());
    }

    if root["method"] == "native-client" && path != "none" {
        match outcome.as_str() {
            "success" => {
                if !registration_loaded
                    || phase != "post"
                    || completion != "completed-result"
                    || content != "full-snapshot-match"
                {
                    return Err("native success requires loaded registration, post event, completed result, and full snapshot match".into());
                }
                if !response_accepted
                    || !result_unchanged
                    || values.is_empty()
                    || capture_agent.as_str() != Some(canonical)
                {
                    return Err("native success requires accepted unchanged response, captured values, and canonical capture agent".into());
                }
                if root["client_version"] == "unknown"
                    || root["rgt_version"] == "unknown"
                    || platform["os"] == "unknown"
                    || platform["architecture"] == "unknown"
                {
                    return Err(
                        "native success requires known client/RGT versions and platform".into(),
                    );
                }
            }
            "no-op" | "failure" | "canceled" => {
                if !registration_loaded
                    || !response_accepted
                    || !result_unchanged
                    || !values.is_empty()
                    || !capture_agent.is_null()
                {
                    return Err("native no-op/error requires loaded registration, accepted unchanged response, and no graph values or capture attribution".into());
                }
            }
            _ => {}
        }
    }
    Ok(())
}

fn has_native_success_noop_pair(records: &[Value]) -> bool {
    records.iter().any(|success| {
        if validate_observation(success).is_err() {
            return false;
        }
        let Some(success_event) = success.get("event") else {
            return false;
        };
        if success.get("method").and_then(Value::as_str) != Some("native-client")
            || success_event.get("outcome").and_then(Value::as_str) != Some("success")
        {
            return false;
        }
        records.iter().any(|noop| {
            if validate_observation(noop).is_err() {
                return false;
            }
            let Some(noop_event) = noop.get("event") else {
                return false;
            };
            ["no-op", "failure", "canceled"].contains(
                &noop_event
                    .get("outcome")
                    .and_then(Value::as_str)
                    .unwrap_or(""),
            ) && noop.get("method") == success.get("method")
                && noop.get("surface_id") == success.get("surface_id")
                && noop.get("read_path_id") == success.get("read_path_id")
                && noop.get("canonical_agent") == success.get("canonical_agent")
                && noop.get("client_version") == success.get("client_version")
                && noop.get("rgt_version") == success.get("rgt_version")
                && noop.get("platform") == success.get("platform")
                && noop.get("evidence_id") != success.get("evidence_id")
        })
    })
}

struct VerifiedClaim {
    surface_id: String,
    canonical_agent: String,
    path: String,
    client_version: String,
    os: String,
    architecture: String,
}

fn verified_claims(support_matrix: &str) -> Result<Vec<VerifiedClaim>, String> {
    let mut claims = Vec::new();
    for line in support_matrix.lines().filter(|line| line.starts_with('|')) {
        let cells: Vec<_> = line.split('|').map(str::trim).collect();
        let Some(capture_cell) = cells.get(6) else {
            continue;
        };
        let capture_status = capture_cell.replace("**", "").to_ascii_lowercase();
        if !capture_status
            .trim_start()
            .starts_with("verified automatic value capture")
        {
            continue;
        }
        let canonical_agent = cells
            .get(1)
            .and_then(|cell| cell.split('`').nth(1))
            .ok_or("verified row must name a canonical agent")?;
        let surface_id = cells
            .get(2)
            .and_then(|cell| cell.split('`').nth(1))
            .ok_or("verified row must name a client surface ID")?;
        let surface = rgt::hooks::registration::ClientSurface::by_id(surface_id)
            .ok_or("verified row names an unknown client surface")?;
        if surface.canonical_agent != canonical_agent {
            return Err("verified row's canonical agent does not own its surface".into());
        }
        let version_platform = cells
            .get(7)
            .ok_or("verified row is missing its version/platform cell")?;
        let (client_version, platform) = version_platform
            .split_once(" / ")
            .ok_or("verified row must use client-version / os/architecture")?;
        let (os, architecture) = platform
            .split_once('/')
            .ok_or("verified row must use client-version / os/architecture")?;
        if client_version.is_empty() || os.is_empty() || architecture.is_empty() {
            return Err("verified row has an empty version/platform component".into());
        }
        let path_cell = cells
            .get(5)
            .ok_or("verified row is missing its read-path cell")?;
        let documented_paths: Vec<&str> = surface.read_paths.iter().map(|path| path.id).collect();
        let before = claims.len();
        let mut in_code = false;
        for token in path_cell.split('`') {
            if in_code && documented_paths.contains(&token) {
                claims.push(VerifiedClaim {
                    surface_id: surface_id.to_string(),
                    canonical_agent: canonical_agent.to_string(),
                    path: token.to_string(),
                    client_version: client_version.to_string(),
                    os: os.to_string(),
                    architecture: architecture.to_string(),
                });
            }
            in_code = !in_code;
        }
        if claims.len() == before {
            return Err("verified row must identify a read path".into());
        }
    }
    Ok(claims)
}

fn published_claims_have_native_pairs(support_matrix: &str, records: &[Value]) -> bool {
    let Ok(claims) = verified_claims(support_matrix) else {
        return false;
    };
    claims.iter().all(|claim| {
        let path_records: Vec<_> = records
            .iter()
            .filter(|record| {
                record["surface_id"].as_str() == Some(&claim.surface_id)
                    && record["canonical_agent"].as_str() == Some(&claim.canonical_agent)
                    && record["read_path_id"].as_str() == Some(&claim.path)
                    && record["client_version"].as_str() == Some(&claim.client_version)
                    && record["platform"]["os"].as_str() == Some(&claim.os)
                    && record["platform"]["architecture"].as_str() == Some(&claim.architecture)
            })
            .cloned()
            .collect();
        has_native_success_noop_pair(&path_records)
    })
}

#[test]
fn published_and_spec_evidence_schemas_stay_synchronized() {
    let tracked: Value = serde_json::from_str(SCHEMA).unwrap();
    let contract: Value = serde_json::from_str(CONTRACT_SCHEMA).unwrap();
    assert_eq!(tracked, contract);
    assert_eq!(tracked["$id"], "urn:rgt:compatibility-evidence:2");
    assert!(tracked["required"]
        .as_array()
        .unwrap()
        .iter()
        .any(|v| v == "rgt_version"));
}

#[test]
fn schema_and_semantic_validation_accept_a_complete_native_pair() {
    let records = vec![good_success(), good_noop()];
    for record in &records {
        validate_observation(record).unwrap();
    }
    assert!(has_native_success_noop_pair(&records));
}

#[test]
fn every_tracked_observation_is_validated_before_it_can_support_a_claim() {
    let records: Value = serde_json::from_str(OBSERVATIONS).unwrap();
    let records = records
        .as_array()
        .expect("the tracked observation ledger must be an array");
    for record in records {
        validate_observation(record)
            .unwrap_or_else(|error| panic!("invalid tracked observation {record}: {error}"));
    }
    assert!(published_claims_have_native_pairs(SUPPORT_MATRIX, records),
        "every read path published as verified automatic capture must have a valid native success/no-op pair");
}

#[test]
fn validator_rejects_schema_and_evidence_contract_violations() {
    let mut invalid = Vec::new();
    let mut record = good_success();
    record.as_object_mut().unwrap().remove("client_version");
    invalid.push(record);
    let mut record = good_success();
    record["method"] = json!("invented-method");
    invalid.push(record);
    let mut record = good_success();
    record["observed_at"] = json!("yesterday");
    invalid.push(record);
    let mut record = good_success();
    record["surface_id"] = json!("Bad Surface");
    invalid.push(record);
    let mut record = good_success();
    record["read_path_id"] = json!("unknown-read-path");
    invalid.push(record);
    let mut record = good_success();
    record["unexpected"] = json!(true);
    invalid.push(record);
    let mut record = good_success();
    record["redacted"] = json!(false);
    invalid.push(record);
    let mut record = good_success();
    record["rgt_version"] = json!("");
    invalid.push(record);
    let mut record = good_success();
    record["platform"]["extra"] = json!(true);
    invalid.push(record);
    let mut record = good_success();
    record["event"]["outcome"] = json!("maybe");
    invalid.push(record);
    let mut record = good_success();
    record["host_response"]["accepted"] = json!("yes");
    invalid.push(record);
    let mut record = good_success();
    record["graph_result"]["values"][0]["source_line"] = json!(0);
    invalid.push(record);
    let mut record = good_success();
    record["graph_result"]["values"][0]["extra"] = json!(1);
    invalid.push(record);
    let mut record = good_success();
    record["read_path_id"] = json!("none");
    invalid.push(record);
    let mut record = good_success();
    record["event"]["tool"] = json!("Write");
    invalid.push(record);
    let mut record = good_success();
    record["event"]["content_evidence"] = json!("path-only");
    invalid.push(record);
    let mut record = good_success();
    record["registration_loaded"] = json!(false);
    invalid.push(record);
    let mut record = good_success();
    record["graph_result"]["values"] = json!([]);
    invalid.push(record);
    let mut record = good_noop();
    record["graph_result"]["values"] = good_success()["graph_result"]["values"].clone();
    invalid.push(record);
    let mut record = good_noop();
    record["graph_result"]["capture_agent"] = json!("claude-code");
    invalid.push(record);
    let mut record = good_success();
    record["client_version"] = json!("unknown");
    invalid.push(record);
    let mut record = good_success();
    record["rgt_version"] = json!("unknown");
    invalid.push(record);
    let mut record = good_success();
    record["platform"]["os"] = json!("unknown");
    invalid.push(record);
    let mut record = good_success();
    record["surface_id"] = json!("codex");
    invalid.push(record);
    let mut record = good_success();
    record["surface_id"] = json!("fake-surface");
    invalid.push(record);

    for record in invalid {
        assert!(
            validate_observation(&record).is_err(),
            "unexpectedly accepted: {record}"
        );
    }
}

#[test]
fn automatic_support_pair_must_use_the_same_read_path_and_version_tuple() {
    let success = good_success();
    let mut mismatch = good_noop();
    mismatch["read_path_id"] = json!("claude-bash-read-stdout");
    mismatch["event"]["tool"] = json!("Bash");
    validate_observation(&mismatch).unwrap();
    assert!(!has_native_success_noop_pair(&[success.clone(), mismatch]));
    assert!(has_native_success_noop_pair(&[success, good_noop()]));

    let mut invalid_noop = good_noop();
    invalid_noop["graph_result"]["capture_agent"] = json!("claude-code");
    assert!(!has_native_success_noop_pair(&[
        good_success(),
        invalid_noop
    ]));
}

#[test]
fn tracked_ledger_allows_valid_pairs_and_requires_them_for_published_claims() {
    let verified_matrix = "| Canonical agent | Client surface | Local registration tier | Registration health evidence | Stable read-path ID and accepted result form | Exclusions / current capture tier | Client version / platform |\n|---|---|---|---|---|---|---|\n| `claude-code` | Claude Code (`claude-code`) | Native | Loaded | `claude-read-post-content` — `Read` | Verified automatic value capture | 1.2.3 / macos/aarch64 |";
    let valid_pair = vec![good_success(), good_noop()];
    assert!(has_native_success_noop_pair(&valid_pair));
    assert!(published_claims_have_native_pairs(
        verified_matrix,
        &valid_pair
    ));

    let unpaired = vec![good_success()];
    assert!(!published_claims_have_native_pairs(
        verified_matrix,
        &unpaired
    ));
    assert!(!published_claims_have_native_pairs(
        SUPPORT_MATRIX,
        &unpaired
    ));
    assert!(!published_claims_have_native_pairs(
        &verified_matrix.replace("1.2.3", "9.9.9"),
        &valid_pair
    ));
    assert!(!published_claims_have_native_pairs(
        &verified_matrix.replace("macos/aarch64", "linux/x86_64"),
        &valid_pair
    ));
    assert!(!published_claims_have_native_pairs(
        &verified_matrix.replace(
            "| `claude-code` | Claude Code (`claude-code`) |",
            "| `codex` | Codex CLI (`codex`) |",
        ),
        &valid_pair
    ));
}

#[test]
fn fixture_matrix_covers_each_surface_and_only_lists_documented_path_forms() {
    let matrix: Value = serde_json::from_str(MATRIX).unwrap();
    assert!(matrix["label"].as_str().unwrap().contains("synthetic-only"));
    let surfaces = matrix["read_paths"].as_array().unwrap();
    let matrix_surface_ids: std::collections::HashSet<_> = surfaces
        .iter()
        .map(|surface| surface["surface_id"].as_str().unwrap())
        .collect();
    assert_eq!(
        matrix_surface_ids.len(),
        rgt::hooks::registration::client_surfaces().len()
    );
    for surface in rgt::hooks::registration::client_surfaces() {
        assert!(matrix_surface_ids.contains(surface.surface_id));
    }
    assert_eq!(
        matrix["automatic_path_ids"],
        json!(["opencode-read-display-text"])
    );
    let mut seen = Vec::new();
    for entry in surfaces {
        let id = entry["surface_id"].as_str().unwrap();
        assert!(
            rgt::hooks::registration::ClientSurface::by_id(id).is_some(),
            "unknown surface {id}"
        );
        let path = entry["read_path_id"].as_str().unwrap();
        assert_eq!(
            entry["automatic_capture"].as_bool(),
            Some(path == "opencode-read-display-text")
        );
        if path != "none" {
            assert!(
                SUPPORT_MATRIX.contains(path),
                "{path} missing from support matrix"
            );
            assert!(SUPPORT_MATRIX.contains(entry["tool"].as_str().unwrap()));
            seen.push(path.to_string());
        }
        for case in entry["cases"].as_array().unwrap() {
            let payload = &case["payload"];
            let input = if let Some(raw) = payload.as_str() {
                raw.to_string()
            } else {
                serde_json::to_string(payload).unwrap()
            };
            let input = if case["bom"].as_bool() == Some(true) {
                format!("\u{feff}{input}")
            } else {
                input
            };
            let agent = entry["canonical_agent"].as_str().unwrap();
            let normalized = rgt::hooks::parser::normalize_agent_event(Some(agent), &input);
            let actual = normalized
                .as_ref()
                .is_some_and(|capture| capture.is_value_capture_eligible());
            assert_eq!(
                actual,
                case["eligible"].as_bool().unwrap(),
                "{}:{} capture={normalized:?}",
                id,
                case["id"]
            );
            let effective = actual && case["snapshot_matches"].as_bool().unwrap_or(true);
            if case["id"] == "changed-snapshot" {
                assert!(!effective, "snapshot mismatch must prevent value creation");
            }
        }
        if entry["automatic_capture"].as_bool() == Some(true) {
            let cases: Vec<&str> = entry["cases"]
                .as_array()
                .unwrap()
                .iter()
                .filter_map(|case| case["id"].as_str())
                .collect();
            for required in ["success", "empty", "malformed", "repeated", "no-op-error"] {
                assert!(cases.contains(&required), "{id}/{path} misses {required}");
            }
        }
    }
    assert!(seen.contains(&"claude-read-post-content".to_string()));
    assert!(seen.contains(&"claude-bash-read-stdout".to_string()));
    assert!(seen.contains(&"codex-post-tool-response".to_string()));
    assert!(seen.contains(&"windsurf-post-read-code-path-only".to_string()));
    assert!(seen.contains(&"opencode-read-display-text".to_string()));
}

#[test]
fn replay_candidate_read_paths_against_local_graph_and_source_snapshots() {
    use std::io::Write;
    use std::process::{Command, Stdio};
    use tempfile::tempdir;

    let matrix: Value = serde_json::from_str(MATRIX).unwrap();
    for entry in matrix["read_paths"].as_array().unwrap() {
        let path_id = entry["read_path_id"].as_str().unwrap();
        if path_id == "none" || path_id == "windsurf-post-read-code-path-only" {
            continue;
        }
        let agent = entry["canonical_agent"].as_str().unwrap();
        let source_text = entry["source_text"].as_str().unwrap();
        let dir = tempdir().unwrap();
        let source = dir.path().join("source.txt");
        std::fs::write(&source, source_text).unwrap();
        let init = Command::new(env!("CARGO_BIN_EXE_rgt"))
            .args(["init", "--agent", agent])
            .current_dir(dir.path())
            .output()
            .unwrap();
        assert!(
            init.status.success(),
            "init {agent}: {}",
            String::from_utf8_lossy(&init.stderr)
        );

        let mut expected_nodes = 0;
        for case in entry["cases"].as_array().unwrap() {
            if case["snapshot_matches"] == false {
                std::fs::write(&source, "amount 99\nwhen 2026-09-26\n").unwrap();
            } else {
                std::fs::write(&source, case["source_text"].as_str().unwrap_or(source_text))
                    .unwrap();
            }
            if case["id"] == "empty" && path_id == "opencode-read-display-text" {
                assert_eq!(std::fs::metadata(&source).unwrap().len(), 0);
                assert_eq!(case["snapshot_matches"], true);
            }
            let mut input = if let Some(raw) = case["payload"].as_str() {
                raw.to_string()
            } else {
                serde_json::to_string(&case["payload"]).unwrap()
            };
            input = input.replace("$SOURCE_PATH", &source.to_string_lossy());
            if case["bom"].as_bool() == Some(true) {
                input.insert(0, '\u{feff}');
            }
            let repeat = case["repeat"].as_u64().unwrap_or(1);
            for _ in 0..repeat {
                let mut child = Command::new(env!("CARGO_BIN_EXE_rgt"))
                    .args(["hook", "post", "--agent", agent])
                    .current_dir(dir.path())
                    .stdin(Stdio::piped())
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped())
                    .spawn()
                    .unwrap();
                child
                    .stdin
                    .take()
                    .unwrap()
                    .write_all(input.as_bytes())
                    .unwrap();
                let output = child.wait_with_output().unwrap();
                assert!(output.status.success(), "{agent}/{path_id}/{}", case["id"]);
                assert!(
                    output.stdout.is_empty(),
                    "unexpected hook output for {agent}/{path_id}"
                );
            }
            if case["eligible"].as_bool() == Some(true)
                && case["snapshot_matches"].as_bool().unwrap_or(true)
            {
                expected_nodes = 2;
            }
            let db = rgt::store::DbStore::open_in_project(dir.path()).unwrap();
            let nodes = rgt::store::queries::list_all_nodes(db.conn()).unwrap();
            assert_eq!(
                nodes.len(),
                expected_nodes,
                "{agent}/{path_id}/{}",
                case["id"]
            );
            let links = rgt::store::queries::list_capture_associations(db.conn()).unwrap();
            assert_eq!(
                links.len(),
                expected_nodes,
                "association count for {agent}/{path_id}"
            );
            assert!(links.iter().all(|(_, observer)| observer == agent));
        }
    }
}
