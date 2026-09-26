use chrono::{Duration, TimeZone, Utc};
use rgt::store::queries::insert_tracked_node;
use rgt::store::DbStore;
use rgt::types::{NodeType, TrackedNode, ValueData};
use std::fs;
use std::path::Path;
use std::process::{Command, Output};
use std::time::Instant;
use tempfile::TempDir;

const NODE_COUNT: usize = 10_000;
const SAMPLES: usize = 20;

struct CommandCase {
    label: String,
    args: Vec<String>,
    succeeds: bool,
}

fn base_fixture() -> TempDir {
    let directory = tempfile::tempdir().unwrap();
    let mut db = DbStore::open_in_project(directory.path()).unwrap();
    let tx = db.conn_mut().unchecked_transaction().unwrap();
    let now = Utc::now();
    let insert = |id: String, value: ValueData| {
        insert_tracked_node(
            &tx,
            &TrackedNode {
                id,
                node_type: NodeType::Root,
                value_kind: value.kind(),
                value,
                source_doc_id: None,
                line_number: None,
                is_stale: false,
                stale_reason: None,
                created_at: now,
                updated_at: now,
            },
        )
        .unwrap();
    };

    insert(
        "date-start".into(),
        ValueData::Date(Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap()),
    );
    insert(
        "date-end".into(),
        ValueData::Date(Utc.with_ymd_and_hms(2026, 2, 15, 0, 0, 0).unwrap()),
    );
    for index in 0..26 {
        insert(
            format!("duration-{index:02}"),
            ValueData::Duration(Duration::seconds(((index + 1) * 2) as i64)),
        );
    }
    insert(
        "duration-one-second".into(),
        ValueData::Duration(Duration::seconds(1)),
    );
    insert("plain-number".into(), ValueData::Number(1.0));

    // 2 Date + 26 aggregate parents + one query Duration + one Number = 30.
    for index in 0..(NODE_COUNT - 30) {
        insert(
            format!("filler-{index:05}"),
            ValueData::Number(index as f64),
        );
    }
    tx.commit().unwrap();
    drop(db);
    assert_eq!(
        rgt::store::queries::list_all_nodes(
            DbStore::open_in_project(directory.path()).unwrap().conn()
        )
        .unwrap()
        .len(),
        NODE_COUNT
    );
    directory
}

fn copy_fixture(base: &Path) -> TempDir {
    let copy = tempfile::tempdir().unwrap();
    let database_dir = copy.path().join(".rgt");
    fs::create_dir_all(&database_dir).unwrap();
    fs::copy(
        base.join(".rgt").join("store.db"),
        database_dir.join("store.db"),
    )
    .unwrap();
    copy
}

fn invoke(project: &TempDir, args: &[String]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_rgt"))
        .args(args)
        .current_dir(project.path())
        .output()
        .expect("run local rgt binary")
}

fn cases() -> Vec<CommandCase> {
    let duration_parents = (0..26)
        .map(|index| format!("duration-{index:02}"))
        .collect::<Vec<_>>()
        .join(",");
    let mut cases = Vec::new();
    for (operation, parents, result, result_unit) in [
        ("DATE_DIFF", "date-start,date-end".to_string(), "45", "days"),
        ("DURATION_SUM", duration_parents.clone(), "702", "seconds"),
        ("DURATION_AVG", duration_parents, "27", "seconds"),
    ] {
        let correct_derive = vec![
            "derive",
            "--parents",
            &parents,
            "--operation",
            operation,
            "--unit",
            "days",
        ];
        cases.push(CommandCase {
            label: format!("{operation} derive valid"),
            args: correct_derive.into_iter().map(str::to_owned).collect(),
            succeeds: true,
        });
        let incorrect_derive = vec![
            "derive",
            "--parents",
            &parents,
            "--operation",
            operation,
            "--result",
            "0",
        ];
        cases.push(CommandCase {
            label: format!("{operation} derive invalid"),
            args: incorrect_derive.into_iter().map(str::to_owned).collect(),
            succeeds: false,
        });
        let correct_verify = vec![
            "verify",
            "--parents",
            &parents,
            "--operation",
            operation,
            "--result",
            result,
            "--result-unit",
            result_unit,
        ];
        cases.push(CommandCase {
            label: format!("{operation} verify valid"),
            args: correct_verify.into_iter().map(str::to_owned).collect(),
            succeeds: true,
        });
        let incorrect_verify = vec![
            "verify",
            "--parents",
            &parents,
            "--operation",
            operation,
            "--result",
            "0",
        ];
        cases.push(CommandCase {
            label: format!("{operation} verify invalid"),
            args: incorrect_verify.into_iter().map(str::to_owned).collect(),
            succeeds: false,
        });
    }
    cases.push(CommandCase {
        label: "query --unit valid".into(),
        args: ["query", "duration-one-second", "--unit", "minutes"]
            .into_iter()
            .map(str::to_owned)
            .collect(),
        succeeds: true,
    });
    cases.push(CommandCase {
        label: "query --unit invalid non-Duration".into(),
        args: ["query", "plain-number", "--unit", "minutes"]
            .into_iter()
            .map(str::to_owned)
            .collect(),
        succeeds: false,
    });
    cases
}

fn cpu_model() -> String {
    #[cfg(target_os = "macos")]
    {
        if let Ok(output) = Command::new("sysctl")
            .args(["-n", "machdep.cpu.brand_string"])
            .output()
        {
            let model = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !model.is_empty() {
                return model;
            }
        }
    }
    #[cfg(target_os = "linux")]
    {
        if let Ok(text) = fs::read_to_string("/proc/cpuinfo") {
            if let Some(model) = text.lines().find(|line| line.starts_with("model name")) {
                return model
                    .split_once(':')
                    .map_or(model, |(_, value)| value)
                    .trim()
                    .to_string();
            }
        }
    }
    format!("{} ({})", std::env::consts::ARCH, std::env::consts::OS)
}

#[test]
#[ignore = "manual local timing"]
fn local_release_duration_commands_report_p95() {
    let base = base_fixture();
    let cases = cases();
    println!(
        "Timing environment: OS={}, CPU={}, Cargo profile={}",
        std::env::consts::OS,
        cpu_model(),
        if cfg!(debug_assertions) {
            "debug"
        } else {
            "release"
        }
    );
    println!(
        "Fixture: {NODE_COUNT} nodes; samples per command: {SAMPLES}; p95: 19th sorted sample"
    );

    for case in cases {
        let warmup_project = copy_fixture(base.path());
        let warmup = invoke(&warmup_project, &case.args);
        assert_eq!(
            warmup.status.success(),
            case.succeeds,
            "warm-up failed for {}: {}",
            case.label,
            String::from_utf8_lossy(&warmup.stderr)
        );

        let mut samples = Vec::with_capacity(SAMPLES);
        for sample_index in 0..SAMPLES {
            // Each invocation starts from a fresh 10,000-node DB. This keeps
            // valid derives on the insertion path without timing fixture copy.
            let project = copy_fixture(base.path());
            let started = Instant::now();
            let output = invoke(&project, &case.args);
            let elapsed = started.elapsed();
            assert_eq!(
                output.status.success(),
                case.succeeds,
                "sample {sample_index} failed for {}: {}",
                case.label,
                String::from_utf8_lossy(&output.stderr)
            );
            samples.push(elapsed);
        }
        samples.sort_unstable();
        let p95 = samples[18];
        let command = case.args.join(" ");
        println!(
            "{} | rgt {} | p95={} ms | samples={:?}",
            case.label,
            command,
            p95.as_secs_f64() * 1_000.0,
            samples
                .iter()
                .map(|sample| sample.as_micros())
                .collect::<Vec<_>>()
        );
    }
}
