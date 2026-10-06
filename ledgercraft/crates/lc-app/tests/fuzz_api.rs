//! Fuzz-style test of the local API (J04): damaged URLs, project ids, path
//! segments and JSON bodies. Every request must get an answer; none may
//! crash the handler or write outside the data folder.

use lc_app::server::App;
use lc_testdata::rng::Rng;
use serde_json::json;

const PATHS: &[&str] = &[
    "/api/projects",
    "/api/projects/{id}",
    "/api/projects/{id}/analysis",
    "/api/projects/{id}/mapping",
    "/api/projects/{id}/mapping/confirm",
    "/api/projects/{id}/settings",
    "/api/projects/{id}/adjustments",
    "/api/projects/{id}/adjustments/1/toggle",
    "/api/projects/{id}/disclosures",
    "/api/projects/{id}/legal",
    "/api/projects/{id}/legal/verify",
    "/api/projects/{id}/bankrec/match",
    "/api/projects/{id}/portal/confirm",
    "/api/projects/{id}/export",
    "/api/projects/{id}/preview",
    "/api/projects/{id}/audit",
    "/api/projects/{id}/remove-input",
    "/api/projects/{id}/next-year",
    "/api/recycle-bin/{id}/restore",
    "/api/search?q={id}",
];

const SEGMENTS: &[&str] = &[
    "..",
    "%2e%2e",
    "..%2f..%2f",
    "%00",
    "%",
    "%zz",
    "~",
    "%7E",
    "CON",
    "a/b",
    "\u{202e}",
    "😀",
    "क",
    "",
    " ",
    "?x=1",
    "#",
];

fn body(rng: &mut Rng) -> Vec<u8> {
    let v = match rng.range(0, 7) {
        0 => json!(null),
        1 => {
            json!({"name": "x".repeat(rng.range(0, 3000) as usize), "entity_type": "company", "fy": "2025-26"})
        }
        2 => {
            json!({"ledger": "../../etc", "voucher": 1, "bank_date": "2026-02-30", "amount": i64::MAX})
        }
        3 => json!({"kind": "gstr2b", "ledger": [], "portal_id": {"a": 1}}),
        4 => {
            json!({"lines": [{"ledger": "Cash", "amount": i64::MIN}, {"ledger": "Bank", "amount": i64::MAX}], "narration": "x", "active": true})
        }
        5 => json!({"mode": "final", "item_id": "x", "content_hash": "y"}),
        _ => json!([1, "two", {"three": [3]}]),
    };
    let mut b = v.to_string().into_bytes();
    if rng.chance(30) {
        let n = b.len().max(1) as i64;
        b.truncate(rng.range(0, n) as usize);
    }
    b
}

#[test]
fn api_answers_every_damaged_request() {
    let dir = std::env::temp_dir().join(format!("lc-fuzz-api-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let data = dir.join("data");
    let app = App::new(data.clone(), "http://127.0.0.1:1").unwrap();
    let r = app.handle(
        "POST",
        "/api/projects",
        json!({"name": "Fuzz Co", "entity_type": "firm", "fy": "2025-26"})
            .to_string()
            .as_bytes(),
    );
    assert_eq!(r.status, 200);
    let id: String = serde_json::from_slice::<serde_json::Value>(&r.body).unwrap()["id"]
        .as_str()
        .unwrap()
        .replace('~', "%7E");
    let rounds: usize = std::env::var("LC_FUZZ_ROUNDS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(600);
    let mut rng = Rng::new(42);
    for i in 0..rounds {
        let pid = if rng.chance(60) {
            id.clone()
        } else {
            (0..rng.range(1, 4))
                .map(|_| *rng.pick(SEGMENTS))
                .collect::<Vec<_>>()
                .join("")
        };
        let url = rng.pick(PATHS).replace("{id}", &pid);
        let method = *rng.pick(&["GET", "POST", "PUT", "DELETE"]);
        let b = body(&mut rng);
        let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            app.handle(method, &url, &b)
        }));
        let r = r.unwrap_or_else(|_| panic!("round {i}: {method} {url} crashed"));
        assert!(
            (200..600).contains(&r.status),
            "round {i}: {method} {url} gave {}",
            r.status
        );
    }
    // Nothing was written outside the data folder.
    for e in std::fs::read_dir(&dir).unwrap() {
        assert_eq!(
            e.unwrap().path(),
            data,
            "unexpected file next to the data folder"
        );
    }
    let _ = std::fs::remove_dir_all(&dir);
}
