//! Storage: client folders, atomic writes, audit trail checkpoints.

use lc_app::store::{write_atomic, Store};
use std::path::PathBuf;

fn tmp(n: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("lc-store-{}-{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    d
}

#[test]
fn different_clients_never_share_a_folder() {
    let st = Store::new(tmp("slug")).unwrap();
    let a = st.create("A&B Traders", "firm", "2025-26").unwrap();
    let b = st.create("A B Traders", "firm", "2025-26").unwrap();
    let c = st.create("A-B Traders", "firm", "2026-27").unwrap();
    assert_ne!(a.split('~').next(), b.split('~').next(), "{a} {b}");
    assert_ne!(a.split('~').next(), c.split('~').next(), "{a} {c}");
    // The same client (case and spacing ignored) keeps its folder for the next year.
    let a2 = st.create("a&b  traders", "firm", "2026-27").unwrap();
    assert_eq!(a.split('~').next(), a2.split('~').next());
    for id in [&a, &b, &c] {
        let p = st.project(id).unwrap();
        let info: lc_app::store::ClientInfo = serde_json::from_str(
            &std::fs::read_to_string(p.client_dir.join("client.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(info.id.len(), 32);
    }
}

#[test]
fn concurrent_atomic_writes_leave_a_whole_file_and_no_temporaries() {
    let d = tmp("atomic");
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("settings.json");
    let hs: Vec<_> = (0..8)
        .map(|t| {
            let f = f.clone();
            std::thread::spawn(move || {
                for i in 0..40 {
                    write_atomic(
                        &f,
                        format!(
                            "{{\"writer\":{t},\"n\":{i},\"pad\":\"{}\"}}",
                            "x".repeat(2000)
                        )
                        .as_bytes(),
                    )
                    .unwrap();
                }
            })
        })
        .collect();
    for h in hs {
        h.join().unwrap();
    }
    let v: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&f).unwrap()).unwrap();
    assert!(v["writer"].is_number());
    let left: Vec<_> = std::fs::read_dir(&d)
        .unwrap()
        .flatten()
        .filter(|e| e.file_name().to_string_lossy().ends_with(".tmp"))
        .collect();
    assert!(left.is_empty(), "temporary files left: {left:?}");
}

#[test]
fn shortened_or_deleted_audit_trail_is_detected_and_never_looks_clean() {
    let st = Store::new(tmp("audit")).unwrap();
    let id = st.create("Audit Test", "firm", "2025-26").unwrap();
    let p = st.project(&id).unwrap();
    for i in 0..3 {
        p.log("user", "test", serde_json::json!({"i": i})).unwrap();
    }
    assert!(p.verify_audit().intact);
    let path = p.dir.join("audit.jsonl");
    let text = std::fs::read_to_string(&path).unwrap();
    let mut lines: Vec<&str> = text.lines().collect();
    lines.pop();
    std::fs::write(&path, lines.join("\n") + "\n").unwrap();
    let s = p.verify_audit();
    assert!(!s.intact && s.problem.unwrap().contains("missing from the end"));
    // Deleted entirely, then the app carries on: the loss is reported for ever.
    std::fs::remove_file(&path).unwrap();
    p.log("user", "after", serde_json::json!({})).unwrap();
    let s = p.verify_audit();
    assert!(
        !s.intact && s.problem.unwrap().contains("deleted"),
        "a fresh trail must not look clean"
    );
}
