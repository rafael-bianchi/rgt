use chrono::{TimeZone, Utc};
use rgt::export::turtle::{load_snapshot, render_turtle_with_limit, MAX_TURTLE_BYTES};
use rgt::store::DbStore;
use rgt::types::{TrackedNode, ValueData};
use rusqlite::{params, Connection};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::thread;
use std::time::{Duration, Instant};

const RUNS: usize = 10;
const WATCHDOG: Duration = Duration::from_secs(5);

#[test]
#[ignore = "manual release benchmark; run from the hosted performance workflow"]
fn release_latency_boundaries() {
    let binary = std::env::var_os("RGT_BENCH_BIN")
        .map(PathBuf::from)
        .expect("set RGT_BENCH_BIN to the release executable");
    let scratch = tempfile::tempdir().unwrap();

    for count in [10_000usize, 10_001] {
        let project = scratch.path().join(format!("values-{count}"));
        fs::create_dir_all(&project).unwrap();
        build_value_fixture(&project, count);
        benchmark_fixture(
            &binary,
            &project,
            &format!("values-{count}"),
            count == 10_000,
            None,
        );
    }

    for count in [30_000usize, 30_001] {
        let project = scratch.path().join(format!("edges-{count}"));
        fs::create_dir_all(&project).unwrap();
        build_edge_fixture(&project, count == 30_001);
        benchmark_fixture(
            &binary,
            &project,
            &format!("edges-{count}"),
            count == 30_000,
            None,
        );
    }

    let representative = scratch.path().join("representative");
    fs::create_dir_all(&representative).unwrap();
    build_representative_fixture(&representative);
    let representative_bytes = rendered_size(&representative);
    assert!(representative_bytes <= MAX_TURTLE_BYTES);
    benchmark_fixture(
        &binary,
        &representative,
        "representative-10000-values-8000-edges-10000-captures",
        true,
        Some(representative_bytes),
    );

    for target in [MAX_TURTLE_BYTES, MAX_TURTLE_BYTES + 1] {
        let project = scratch.path().join(format!("bytes-{target}"));
        fs::create_dir_all(&project).unwrap();
        build_representative_fixture(&project);
        pad_representative_expressions_to(&project, target);
        assert_eq!(rendered_size(&project), target);
        benchmark_fixture(
            &binary,
            &project,
            &format!("rendered-bytes-{target}"),
            target == MAX_TURTLE_BYTES,
            Some(target),
        );
    }
}

#[test]
#[ignore = "manual real-pipe deadline probe runs on Linux, macOS, and Windows"]
fn blocked_stdout_pipe() {
    let binary = std::env::var_os("RGT_BENCH_BIN")
        .map(PathBuf::from)
        .expect("set RGT_BENCH_BIN to the release executable");
    let project = tempfile::tempdir().unwrap();
    build_blocked_pipe_fixture(project.path());
    let rendered = rendered_size(project.path());
    assert!(
        rendered >= 2 * 1024 * 1024,
        "fixture has only {rendered} bytes"
    );

    let mut child = Command::new(binary)
        .args(["graph", "--format", "ttl"])
        .current_dir(project.path())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn release rgt");
    let started = Instant::now();
    let mut unread_stdout = child.stdout.take().expect("stdout pipe");
    let status = wait_with_watchdog(&mut child, WATCHDOG);
    let elapsed = started.elapsed();
    let stderr = read_stderr(&mut child);
    let mut partial = Vec::new();
    unread_stdout.read_to_end(&mut partial).unwrap();
    println!(
        "probe=blocked_stdout_pipe bytes={rendered} elapsed_ms={} exit={:?} partial_bytes={} stderr={}",
        elapsed.as_millis(),
        status.code(),
        partial.len(),
        String::from_utf8_lossy(&stderr).trim()
    );
    assert!(!status.success(), "blocked pipe must fail nonzero");
    assert!(
        elapsed < Duration::from_secs(1),
        "blocked pipe took {elapsed:?}"
    );
    assert!(
        String::from_utf8_lossy(&stderr).contains("while writing stdout"),
        "expected final-write timeout, got: {}",
        String::from_utf8_lossy(&stderr)
    );
    // A Windows anonymous-pipe WriteFile can remain pending without making a
    // partial buffer visible to the reader. The deadline, nonzero exit, and
    // final-write error above are the portable contract.
    #[cfg(not(windows))]
    assert!(
        !partial.is_empty(),
        "the pipe must have received a partial write before stalling"
    );
}

fn benchmark_fixture(
    binary: &Path,
    project: &Path,
    label: &str,
    should_succeed: bool,
    expected_bytes: Option<usize>,
) {
    for run in 0..=RUNS {
        let (elapsed, status, output_bytes, stderr) = run_to_file(binary, project);
        assert!(
            elapsed < Duration::from_secs(1),
            "{label} run {run} exceeded one second: {elapsed:?}; stderr={}",
            String::from_utf8_lossy(&stderr)
        );
        assert_eq!(
            status.success(),
            should_succeed,
            "{label} run {run}: {}",
            String::from_utf8_lossy(&stderr)
        );
        if should_succeed {
            if let Some(expected) = expected_bytes {
                assert_eq!(output_bytes, expected as u64, "{label} output size");
            }
        } else {
            assert_eq!(output_bytes, 0, "over-limit errors must not write stdout");
        }
        println!(
            "case={label} run={} phase={} elapsed_ms={} rendered_bytes={} exit={:?} stderr={}",
            run,
            if run == 0 { "warmup" } else { "measured" },
            elapsed.as_millis(),
            output_bytes,
            status.code(),
            String::from_utf8_lossy(&stderr).trim()
        );
    }
}

fn run_to_file(binary: &Path, project: &Path) -> (Duration, ExitStatus, u64, Vec<u8>) {
    let output = tempfile::NamedTempFile::new().unwrap();
    let file = output.reopen().unwrap();
    let started = Instant::now();
    let mut child = Command::new(binary)
        .args(["graph", "-f", "ttl"])
        .current_dir(project)
        .stdout(Stdio::from(file))
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn release rgt");
    let status = wait_with_watchdog(&mut child, WATCHDOG);
    let elapsed = started.elapsed();
    let stderr = read_stderr(&mut child);
    let output_bytes = output.as_file().metadata().unwrap().len();
    (elapsed, status, output_bytes, stderr)
}

fn wait_with_watchdog(child: &mut Child, timeout: Duration) -> ExitStatus {
    let deadline = Instant::now() + timeout;
    loop {
        if let Some(status) = child.try_wait().expect("poll benchmark process") {
            return status;
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            panic!("benchmark subprocess exceeded watchdog of {timeout:?}");
        }
        thread::sleep(Duration::from_millis(5));
    }
}

fn read_stderr(child: &mut Child) -> Vec<u8> {
    let mut stderr = Vec::new();
    if let Some(mut stream) = child.stderr.take() {
        stream.read_to_end(&mut stderr).unwrap();
    }
    stderr
}

fn open_project(project: &Path) -> DbStore {
    DbStore::open_in_project(project).expect("open benchmark store")
}

struct FixtureNode<'a> {
    id: &'a str,
    node_type: &'a str,
    kind: &'a str,
    number: Option<f64>,
    date: Option<&'a str>,
    duration: Option<i64>,
    source: Option<i64>,
}

fn insert_node(conn: &Connection, node: FixtureNode<'_>) {
    conn.execute(
        "INSERT INTO tracked_nodes (id,node_type,value_kind,number_val,date_val,duration_secs,
             source_doc_id,line_number,is_stale,created_at,updated_at)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,0,'2026-01-01T00:00:00Z','2026-01-01T00:00:00Z')",
        params![
            node.id,
            node.node_type,
            node.kind,
            node.number,
            node.date,
            node.duration,
            node.source,
            node.source.map(|_| 1i64)
        ],
    )
    .unwrap();
}

fn insert_source(conn: &Connection, project: &Path, relative: &str) -> i64 {
    let path = project.join(relative);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(&path, "fixture\n").unwrap();
    conn.execute(
        "INSERT INTO source_documents (file_path,mtime_nsec,file_size,blake3_hash,last_checked_at)
         VALUES (?1,0,8,'fixture-hash','2026-01-01T00:00:00Z')",
        [relative],
    )
    .unwrap();
    conn.last_insert_rowid()
}

fn build_value_fixture(project: &Path, count: usize) {
    let db = open_project(project);
    let conn = db.conn();
    conn.execute_batch("BEGIN IMMEDIATE").unwrap();
    let source = insert_source(conn, project, "shared.csv");
    for index in 0..count {
        insert_node(
            conn,
            FixtureNode {
                id: &format!("value-{index:05}"),
                node_type: "ROOT",
                kind: "NUMBER",
                number: Some(index as f64 + 0.5),
                date: None,
                duration: None,
                source: Some(source),
            },
        );
    }
    conn.execute_batch("COMMIT").unwrap();
}

fn build_edge_fixture(project: &Path, one_over: bool) {
    let db = open_project(project);
    let conn = db.conn();
    conn.execute_batch("BEGIN IMMEDIATE").unwrap();
    for index in 0..3_000 {
        insert_node(
            conn,
            FixtureNode {
                id: &format!("root-{index:04}"),
                node_type: "ROOT",
                kind: "NUMBER",
                number: Some(index as f64),
                date: None,
                duration: None,
                source: None,
            },
        );
    }
    for child_index in 0..7_000 {
        let parent_count = if child_index < 2_000 || (one_over && child_index == 2_000) {
            5
        } else {
            4
        };
        let parents = (0..parent_count)
            .map(|offset| format!("root-{:04}", (child_index * 7 + offset * 13) % 3_000))
            .collect::<Vec<_>>();
        let value = ValueData::Number(10_000.0 + child_index as f64);
        let child = TrackedNode::generate_derived_id(&parents, "EXPRESSION", &value);
        insert_node(
            conn,
            FixtureNode {
                id: &child,
                node_type: "DERIVED",
                kind: "NUMBER",
                number: Some(10_000.0 + child_index as f64),
                date: None,
                duration: None,
                source: None,
            },
        );
        let expression = match parent_count {
            4 => "a + b + c + d",
            5 => "a + b + c + d + e",
            _ => unreachable!(),
        };
        for parent in parents {
            conn.execute(
                "INSERT INTO derivation_edges (parent_node_id,child_node_id,operation_type,expression)
                 VALUES (?1,?2,'EXPRESSION',?3)",
                params![parent, child, expression],
            )
            .unwrap();
        }
    }
    conn.execute_batch("COMMIT").unwrap();
}

fn build_representative_fixture(project: &Path) {
    let db = open_project(project);
    let conn = db.conn();
    conn.execute_batch("BEGIN IMMEDIATE").unwrap();
    let mut number_roots = Vec::new();
    let mut date_roots = Vec::new();
    let mut values = Vec::with_capacity(10_000);
    for index in 0..4_000 {
        let id = format!("number-root-{index:04}");
        let source = insert_source(conn, project, &format!("sources/n-{index:04}.csv"));
        insert_node(
            conn,
            FixtureNode {
                id: &id,
                node_type: "ROOT",
                kind: "NUMBER",
                number: Some(index as f64 + 0.25),
                date: None,
                duration: None,
                source: Some(source),
            },
        );
        number_roots.push(id.clone());
        values.push(id);
    }
    for index in 0..2_000 {
        let id = format!("date-root-{index:04}");
        let source = insert_source(conn, project, &format!("sources/d-{index:04}.csv"));
        let lexical = Utc
            .timestamp_opt(1_700_000_000 + index as i64, 0)
            .single()
            .unwrap()
            .to_rfc3339();
        insert_node(
            conn,
            FixtureNode {
                id: &id,
                node_type: "ROOT",
                kind: "DATE",
                number: None,
                date: Some(&lexical),
                duration: None,
                source: Some(source),
            },
        );
        date_roots.push(id.clone());
        values.push(id);
    }
    for index in 0..2_000 {
        let parents = vec![
            number_roots[index * 2].clone(),
            number_roots[index * 2 + 1].clone(),
        ];
        let value = ValueData::Number(20_000.0 + index as f64);
        let child = TrackedNode::generate_derived_id(&parents, "EXPRESSION", &value);
        insert_node(
            conn,
            FixtureNode {
                id: &child,
                node_type: "DERIVED",
                kind: "NUMBER",
                number: Some(20_000.0 + index as f64),
                date: None,
                duration: None,
                source: None,
            },
        );
        for parent in &parents {
            conn.execute(
                "INSERT INTO derivation_edges (parent_node_id,child_node_id,operation_type,expression)
                 VALUES (?1,?2,'EXPRESSION','a + b')",
                params![parent, child],
            )
            .unwrap();
        }
        values.push(child);
    }
    for index in 0..2_000 {
        let pair = (index % 1_000) * 2;
        let parents = vec![date_roots[pair].clone(), date_roots[pair + 1].clone()];
        let seconds = index as i64 + 1;
        let value = ValueData::Duration(chrono::Duration::seconds(seconds));
        let child = TrackedNode::generate_derived_id(&parents, "DATE_DIFF", &value);
        insert_node(
            conn,
            FixtureNode {
                id: &child,
                node_type: "DERIVED",
                kind: "DURATION",
                number: None,
                date: None,
                duration: Some(seconds),
                source: None,
            },
        );
        for parent in &parents {
            conn.execute(
                "INSERT INTO derivation_edges (parent_node_id,child_node_id,operation_type,expression)
                 VALUES (?1,?2,'DATE_DIFF',NULL)",
                params![parent, child],
            )
            .unwrap();
        }
        values.push(child);
    }
    assert_eq!(values.len(), 10_000);
    for (index, node) in values.iter().enumerate() {
        let agent = if index % 2 == 0 {
            "claude-code"
        } else {
            "cursor"
        };
        conn.execute(
            "INSERT INTO capture_associations (node_id,agent_name) VALUES (?1,?2)",
            params![node, agent],
        )
        .unwrap();
    }
    conn.execute_batch("COMMIT").unwrap();
}

fn pad_representative_expressions_to(project: &Path, target_bytes: usize) {
    let baseline = rendered_size(project);
    assert!(
        baseline < target_bytes,
        "fixture base size is already over byte boundary"
    );
    let padding_total = target_bytes - baseline;
    let db = open_project(project);
    let conn = db.conn();
    conn.execute_batch("BEGIN IMMEDIATE").unwrap();
    let child_ids = {
        let mut statement = conn
            .prepare(
                "SELECT child_node_id FROM derivation_edges WHERE operation_type='EXPRESSION'
                 GROUP BY child_node_id ORDER BY child_node_id",
            )
            .unwrap();
        statement
            .query_map([], |row| row.get::<_, String>(0))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
    };
    assert_eq!(child_ids.len(), 2_000);
    let base = padding_total / child_ids.len();
    let remainder = padding_total % child_ids.len();
    let mut update = conn
        .prepare("UPDATE derivation_edges SET expression=?1 WHERE child_node_id=?2")
        .unwrap();
    for (index, child) in child_ids.iter().enumerate() {
        let padding = base + usize::from(index < remainder);
        update
            .execute(params![format!("a + b{}", " ".repeat(padding)), child])
            .unwrap();
    }
    drop(update);
    conn.execute_batch("COMMIT").unwrap();
}

fn rendered_size(project: &Path) -> usize {
    let db = open_project(project);
    let snapshot = load_snapshot(db.conn(), Instant::now() + Duration::from_secs(60)).unwrap();
    render_turtle_with_limit(
        &snapshot,
        project,
        false,
        usize::MAX,
        Instant::now() + Duration::from_secs(60),
    )
    .unwrap()
    .turtle
    .len()
}

fn build_blocked_pipe_fixture(project: &Path) {
    let db = open_project(project);
    let node_id = "x".repeat(500_000);
    insert_node(
        db.conn(),
        FixtureNode {
            id: &node_id,
            node_type: "ROOT",
            kind: "NUMBER",
            number: Some(1.0),
            date: None,
            duration: None,
            source: None,
        },
    );
}
