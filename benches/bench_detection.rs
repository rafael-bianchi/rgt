use std::time::Instant;

fn main() {
    let dir = std::env::temp_dir().join("rgt_bench");
    let _ = std::fs::create_dir_all(&dir);
    let file_path = dir.join("test.txt");
    std::fs::write(&file_path, "12345").unwrap();

    let start = Instant::now();
    let meta = rgt::detection::get_metadata_snapshot(&file_path).unwrap();
    let changed = rgt::detection::tier1_check_changed(&meta, meta.mtime_nsec, meta.file_size);
    let elapsed = start.elapsed();

    assert!(!changed);
    println!("Tier 1 check latency: {:?}", elapsed);
    assert!(elapsed.as_millis() < 1, "Tier 1 check MUST stay under 1ms");
}
