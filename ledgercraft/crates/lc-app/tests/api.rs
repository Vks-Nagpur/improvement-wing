//! End-to-end through the app's local HTTP API, as the browser uses it.

use lc_app::server::{serve, App};
use serde_json::{json, Value};
use std::collections::BTreeSet;
use std::sync::Arc;

struct Client {
    base: String,
    token: String,
}

impl Client {
    fn call(&self, method: &str, path: &str, body: Option<Value>) -> Result<Value, String> {
        let req =
            ureq::request(method, &format!("{}{path}", self.base)).set("X-LC-Token", &self.token);
        let r = match body {
            Some(b) => req.send_json(b),
            None => req.call(),
        };
        match r {
            Ok(resp) => Ok(resp.into_json().unwrap_or(Value::Null)),
            Err(ureq::Error::Status(_, resp)) => Err(resp
                .into_json::<Value>()
                .ok()
                .and_then(|v| v["error"].as_str().map(String::from))
                .unwrap_or_default()),
            Err(e) => Err(e.to_string()),
        }
    }
    fn upload(&self, id: &str, kind: &str, path: &std::path::Path) -> Result<Value, String> {
        let bytes = std::fs::read(path).unwrap();
        let url = format!(
            "{}/api/projects/{}/upload?kind={kind}&name={}",
            self.base,
            id,
            path.file_name().unwrap().to_string_lossy()
        );
        match ureq::post(&url)
            .set("X-LC-Token", &self.token)
            .send_bytes(&bytes)
        {
            Ok(r) => Ok(r.into_json().unwrap()),
            Err(ureq::Error::Status(_, r)) => Err(r.into_json::<Value>().unwrap()["error"]
                .as_str()
                .unwrap_or("")
                .to_string()),
            Err(e) => Err(e.to_string()),
        }
    }
}

#[test]
fn full_flow_through_the_app() {
    let dir = std::env::temp_dir().join(format!("lc-app-test-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let app = Arc::new(App::new(dir.join("data"), "http://127.0.0.1:1").unwrap());
    let (tx, rx) = std::sync::mpsc::channel();
    let a2 = app.clone();
    std::thread::spawn(move || serve(a2, 0, |addr| tx.send(addr).unwrap()).unwrap());
    let base = rx.recv().unwrap();

    // Security: no token → refused; foreign Host → refused.
    assert_eq!(
        ureq::get(&format!("{base}/api/projects"))
            .call()
            .unwrap_err()
            .into_response()
            .unwrap()
            .status(),
        403
    );
    let foreign = ureq::get(&format!("{base}/api/projects"))
        .set("Host", "evil.example:80")
        .set("X-LC-Token", &app.token)
        .call();
    assert_eq!(foreign.unwrap_err().into_response().unwrap().status(), 403);
    // The page carries the token for the app itself.
    let html = ureq::get(&format!("{base}/"))
        .call()
        .unwrap()
        .into_string()
        .unwrap();
    assert!(html.contains(&app.token));
    let c = Client {
        base: base.clone(),
        token: app.token.clone(),
    };

    // Practice books with planted problems, written as files a user would pick.
    let s = lc_testdata::scenarios::firm_with_glitches();
    let f = dir.join("files");
    std::fs::create_dir_all(&f).unwrap();
    lc_io::write_inputs::write_trial_balance(&s.engagement.cy, &f.join("tb.xlsx")).unwrap();
    lc_io::write_inputs::write_trial_balance(s.engagement.py.as_ref().unwrap(), &f.join("py.xlsx"))
        .unwrap();
    lc_io::write_inputs::write_vouchers_csv(&s.engagement.vouchers, &f.join("daybook.csv"))
        .unwrap();

    let id = c
        .call(
            "POST",
            "/api/projects",
            Some(json!({"name": "Glitchy Traders", "entity_type": "firm", "fy": "2025-26"})),
        )
        .unwrap()["id"]
        .as_str()
        .unwrap()
        .to_string();
    assert!(
        c.call(
            "POST",
            "/api/projects",
            Some(json!({"name": "Glitchy Traders", "entity_type": "firm", "fy": "2025-26"}))
        )
        .is_err(),
        "duplicate year refused"
    );
    let pid = id.replace('~', "%7E");
    assert!(
        c.upload(&pid, "tb", &f.join("daybook.csv"))
            .unwrap_err()
            .contains("could not be read"),
        "wrong file refused, nothing replaced"
    );
    c.upload(&pid, "tb", &f.join("tb.xlsx")).unwrap();
    c.upload(&pid, "py_tb", &f.join("py.xlsx")).unwrap();
    c.upload(&pid, "vouchers", &f.join("daybook.csv")).unwrap();
    // Tags are kept by LedgerCraft, not in the books.
    c.call(
        "POST",
        &format!("/api/projects/{pid}/tags"),
        Some(json!({"ledger": "Shree Roadlines", "tags": ["transporter"]})),
    )
    .unwrap();

    let a = c
        .call("POST", &format!("/api/projects/{pid}/analyse"), None)
        .unwrap();
    let keys: BTreeSet<String> = a["findings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["key"].as_str().unwrap().to_string())
        .collect();
    let mut want = s.expected.keys.clone();
    want.retain(|k| !k.starts_with("FAR_")); // no register uploaded here
    assert_eq!(keys, want);

    // Map a mis-grouped loan once: remembered, logged, and it changes the statements.
    c.call(
        "POST",
        &format!("/api/projects/{pid}/mapping"),
        Some(json!({"ledger": "Loan from Suresh Kumar", "head": "LT_BORROWINGS"})),
    )
    .unwrap();
    let a = c
        .call("POST", &format!("/api/projects/{pid}/analyse"), None)
        .unwrap();
    let m = a["mapping"]
        .as_array()
        .unwrap()
        .iter()
        .find(|m| m["name"] == "Loan from Suresh Kumar")
        .unwrap()
        .clone();
    assert_eq!(m["head"], "LT_BORROWINGS");
    assert_eq!(m["source"], "memory");

    // Presentation options, preview, export.
    let mut opts: Value = serde_json::to_value(lc_core::report::ReportOptions::default()).unwrap();
    opts["unit"] = json!("lakhs");
    c.call(
        "POST",
        &format!("/api/projects/{pid}/settings"),
        Some(
            json!({"options": opts, "signoff": {"udin": "26123456ABCDEF1234", "place": "Nagpur"}}),
        ),
    )
    .unwrap();
    let preview = ureq::get(&format!(
        "{base}/api/projects/{pid}/preview?t={}",
        app.token
    ))
    .call()
    .unwrap()
    .into_string()
    .unwrap();
    assert!(preview.contains("GLITCHY TRADERS") && preview.contains("₹ lakhs"));
    let err = c.call(
        "POST",
        &format!("/api/projects/{pid}/export"),
        Some(json!({"mode": "final"})),
    );
    assert!(err.is_ok(), "warnings only: final copy allowed: {err:?}");
    let ex = c
        .call(
            "POST",
            &format!("/api/projects/{pid}/export"),
            Some(json!({"mode": "draft"})),
        )
        .unwrap();
    let out = std::path::PathBuf::from(ex["dir"].as_str().unwrap());
    for f in [
        "Financial_Statements.pdf",
        "Financial_Statements.xlsx",
        "Auditor_Reference_Workbook.xlsx",
        "export-manifest.json",
    ] {
        assert!(out.join(f).exists(), "{f}");
    }

    // AI unavailable: clear message, nothing breaks.
    let e = c
        .call(
            "POST",
            &format!("/api/projects/{pid}/ai/explain"),
            Some(json!({"key": a["findings"][0]["key"]})),
        )
        .unwrap_err();
    assert!(e.contains("not running"), "{e}");

    // Audit trail: complete, intact, and tamper-evident.
    let au = c
        .call("GET", &format!("/api/projects/{pid}/audit"), None)
        .unwrap();
    assert_eq!(au["status"]["intact"], true);
    let actions: Vec<String> = au["events"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["action"].as_str().unwrap().to_string())
        .collect();
    for want in [
        "project_created",
        "file_imported",
        "tags_changed",
        "checks_run",
        "mapping_changed",
        "settings_changed",
        "exported",
    ] {
        assert!(
            actions.iter().any(|a| a == want),
            "audit missing {want}: {actions:?}"
        );
    }
    let log = dir.join("data/clients/Glitchy-Traders/2025-26/audit.jsonl");
    let text = std::fs::read_to_string(&log).unwrap();
    std::fs::write(&log, text.replacen("LT_BORROWINGS", "TRADE_PAYABLES", 1)).unwrap();
    let au = c
        .call("GET", &format!("/api/projects/{pid}/audit"), None)
        .unwrap();
    assert_eq!(
        au["status"]["intact"], false,
        "editing the log must be detected"
    );
    assert!(au["status"]["problem"]
        .as_str()
        .unwrap()
        .contains("altered"));

    let _ = c.call("POST", "/api/quit", None);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn adjustments_hiding_removal_and_recycle_bin() {
    let dir = std::env::temp_dir().join(format!("lc-app-test2-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let app = Arc::new(App::new(dir.join("data"), "http://127.0.0.1:1").unwrap());
    let (tx, rx) = std::sync::mpsc::channel();
    let a2 = app.clone();
    std::thread::spawn(move || serve(a2, 0, |addr| tx.send(addr).unwrap()).unwrap());
    let c = Client {
        base: rx.recv().unwrap(),
        token: app.token.clone(),
    };
    let s = lc_testdata::scenarios::firm_with_glitches();
    let f = dir.join("files");
    std::fs::create_dir_all(&f).unwrap();
    lc_io::write_inputs::write_trial_balance(&s.engagement.cy, &f.join("tb.xlsx")).unwrap();
    lc_io::write_inputs::write_trial_balance(s.engagement.py.as_ref().unwrap(), &f.join("py.xlsx"))
        .unwrap();
    lc_io::write_inputs::write_vouchers_csv(&s.engagement.vouchers, &f.join("daybook.csv"))
        .unwrap();
    let id = c
        .call(
            "POST",
            "/api/projects",
            Some(json!({"name": "Adjust Co", "entity_type": "firm", "fy": "2025-26"})),
        )
        .unwrap()["id"]
        .as_str()
        .unwrap()
        .to_string();
    let pid = id.replace('~', "%7E");
    c.upload(&pid, "tb", &f.join("tb.xlsx")).unwrap();
    c.upload(&pid, "py_tb", &f.join("py.xlsx")).unwrap();
    c.upload(&pid, "vouchers", &f.join("daybook.csv")).unwrap();
    let before = c
        .call("POST", &format!("/api/projects/{pid}/analyse"), None)
        .unwrap();
    let profit = |a: &Value| a["key"]["cy"]["profit"].as_i64().unwrap();
    let mismatches = |a: &Value| {
        a["findings"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|f| f["code"] == "VOUCHER_TB_MISMATCH")
            .count()
    };

    // Unbalanced and unknown-ledger entries are refused; nothing is saved.
    let bad = c.call(
        "POST",
        &format!("/api/projects/{pid}/adjustments"),
        Some(json!({"kind": "provision", "narration": "x", "lines": [
            {"ledger": "Audit Fees", "amount": 2500000, "new_group": "Indirect Expenses"},
            {"ledger": "Audit Fees Payable", "amount": -2400000, "new_group": "Provisions"}]})),
    );
    assert!(bad.unwrap_err().contains("do not agree"));
    let bad = c.call(
        "POST",
        &format!("/api/projects/{pid}/adjustments"),
        Some(json!({"kind": "provision", "narration": "x", "lines": [
            {"ledger": "No Such Ledger", "amount": 100}, {"ledger": "Cash", "amount": -100}]})),
    );
    assert!(bad.unwrap_err().contains("not in the books"));

    // A provision of ₹25,000 with two new ledgers: profit falls by exactly that.
    c.call(
        "POST",
        &format!("/api/projects/{pid}/adjustments"),
        Some(
            json!({"kind": "provision", "narration": "Audit fee payable", "lines": [
            {"ledger": "Audit Fees", "amount": 2500000, "new_group": "Indirect Expenses"},
            {"ledger": "Audit Fees Payable", "amount": -2500000, "new_group": "Provisions"}]}),
        ),
    )
    .unwrap();
    let after = c
        .call("POST", &format!("/api/projects/{pid}/analyse"), None)
        .unwrap();
    assert_eq!(profit(&before) - profit(&after), 2500000);
    assert_eq!(
        mismatches(&before),
        mismatches(&after),
        "adjustments flow through the day book too"
    );
    let list = c
        .call("GET", &format!("/api/projects/{pid}/adjustments"), None)
        .unwrap();
    assert_eq!(list["list"][0]["id"], 1);
    assert_eq!(list["effects"].as_array().unwrap().len(), 2);

    // Switched off: back to the books. Deleted: gone, but in the audit trail.
    c.call(
        "POST",
        &format!("/api/projects/{pid}/adjustments/1/active"),
        Some(json!({"active": false})),
    )
    .unwrap();
    let off = c
        .call("POST", &format!("/api/projects/{pid}/analyse"), None)
        .unwrap();
    assert_eq!(profit(&off), profit(&before));
    c.call(
        "POST",
        &format!("/api/projects/{pid}/adjustments/1/active"),
        Some(json!({"active": true})),
    )
    .unwrap();

    // Hide the cash flow and notes; export only the workbook (with adjustments).
    let p = c
        .call("GET", &format!("/api/projects/{pid}"), None)
        .unwrap();
    let mut o = p["settings"]["options"].clone();
    o["cash_flow"] = json!("off");
    o["hidden_sections"] = json!(["notes"]);
    c.call(
        "POST",
        &format!("/api/projects/{pid}/settings"),
        Some(json!({"options": o})),
    )
    .unwrap();
    let html = ureq::get(&format!(
        "{}/api/projects/{pid}/preview?t={}",
        c.base, c.token
    ))
    .call()
    .unwrap()
    .into_string()
    .unwrap();
    assert!(html.contains("Balance Sheet as at"));
    assert!(!html.contains("Cash Flow Statement for"));
    assert!(!html.contains("Notes to the financial statements"));
    let ex = c
        .call(
            "POST",
            &format!("/api/projects/{pid}/export"),
            Some(json!({"mode": "draft", "files": {"pdf": false, "xlsx": false, "html": false, "json": false, "auditor_workbook": true}})),
        )
        .unwrap();
    let names: Vec<String> = ex["files"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f[0].as_str().unwrap().to_string())
        .collect();
    assert_eq!(names, vec!["Auditor_Reference_Workbook.xlsx".to_string()]);
    let wb = std::path::Path::new(ex["dir"].as_str().unwrap()).join(&names[0]);
    let mut book: calamine::Xlsx<_> = calamine::open_workbook(&wb).unwrap();
    assert!(calamine::Reader::sheet_names(&book).contains(&"Adjustments".to_string()));
    let r = calamine::Reader::worksheet_range(&mut book, "Adjustments").unwrap();
    assert!(r
        .rows()
        .any(|row| row.iter().any(|c| *c == "Audit Fees Payable")));
    assert!(c
        .call(
            "POST",
            &format!("/api/projects/{pid}/export"),
            Some(json!({"mode": "draft", "files": {"pdf": false, "xlsx": false, "html": false, "json": false, "auditor_workbook": false}})),
        )
        .unwrap_err()
        .contains("at least one"));

    c.call(
        "POST",
        &format!("/api/projects/{pid}/adjustments/1/delete"),
        None,
    )
    .unwrap();
    // Remove a file: no longer used, copy kept.
    c.call(
        "POST",
        &format!("/api/projects/{pid}/remove-input"),
        Some(json!({"kind": "vouchers"})),
    )
    .unwrap();
    let p = c
        .call("GET", &format!("/api/projects/{pid}"), None)
        .unwrap();
    assert!(p["settings"]["inputs"]["vouchers"].is_null());
    assert!(dir
        .join("data/clients/Adjust-Co/2025-26/inputs/removed")
        .read_dir()
        .unwrap()
        .next()
        .is_some());

    // Delete → Recycle Bin → restore, with the audit trail intact.
    c.call("POST", &format!("/api/projects/{pid}/delete"), None)
        .unwrap();
    assert!(c
        .call("GET", "/api/projects", None)
        .unwrap()
        .as_array()
        .unwrap()
        .is_empty());
    let bin = c.call("GET", "/api/recycle-bin", None).unwrap();
    let item = bin[0]["id"].as_str().unwrap().replace('~', "%7E");
    c.call("POST", &format!("/api/recycle-bin/{item}/restore"), None)
        .unwrap();
    let au = c
        .call("GET", &format!("/api/projects/{pid}/audit"), None)
        .unwrap();
    assert_eq!(au["status"]["intact"], true);
    let actions: Vec<String> = au["events"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["action"].as_str().unwrap().to_string())
        .collect();
    for want in [
        "adjustment_added",
        "adjustment_switched",
        "adjustment_deleted",
        "file_removed",
        "project_deleted",
        "project_restored",
    ] {
        assert!(actions.iter().any(|a| a == want), "audit missing {want}");
    }
    let _ = c.call("POST", "/api/quit", None);
    let _ = std::fs::remove_dir_all(&dir);
}
