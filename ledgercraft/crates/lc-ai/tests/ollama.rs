//! Mock Ollama server for the protocol; plus an optional live check against a
//! real local Ollama (set LC_OLLAMA_LIVE=1).

use lc_ai::{Assistant, Language, OllamaClient, OllamaError};
use lc_core::mapping::Head;
use std::thread;

fn mock(chat_reply: &'static str) -> String {
    let server = tiny_http::Server::http("127.0.0.1:0").unwrap();
    let base = format!("http://{}", server.server_addr().to_ip().unwrap());
    thread::spawn(move || {
        for mut req in server.incoming_requests() {
            let url = req.url().to_string();
            let mut body = String::new();
            std::io::Read::read_to_string(req.as_reader(), &mut body).unwrap();
            let reply = match url.as_str() {
                "/api/version" => r#"{"version":"0.12.0"}"#.to_string(),
                "/api/tags" => r#"{"models":[{"name":"qwen2.5:3b","size":1929912432}]}"#.to_string(),
                "/api/pull" => "{\"status\":\"pulling manifest\"}\n{\"status\":\"downloading\",\"completed\":50,\"total\":100}\n{\"status\":\"success\"}\n".to_string(),
                "/api/chat" => {
                    let v: serde_json::Value = serde_json::from_str(&body).unwrap();
                    assert_eq!(v["stream"], false);
                    assert_eq!(v["messages"][0]["role"], "system");
                    if body.contains("Reply as JSON") {
                        assert!(v["format"]["properties"]["head"]["enum"].is_array(), "mapping must request constrained JSON");
                    }
                    serde_json::json!({"message": {"role": "assistant", "content": chat_reply}, "done": true}).to_string()
                }
                _ => r#"{"error":"not found"}"#.to_string(),
            };
            req.respond(tiny_http::Response::from_string(reply))
                .unwrap();
        }
    });
    base
}

#[test]
fn protocol_and_guards() {
    let base = mock(r#"{"head":"LT_BORROWINGS","reason":"Loan taken from a person."}"#);
    let c = OllamaClient::new(&base, "qwen2.5:3b");
    assert_eq!(c.version().unwrap(), "0.12.0");
    assert!(c.has_model().unwrap());
    let mut seen = Vec::new();
    c.pull(|s, done, total| seen.push((s.to_string(), done, total)))
        .unwrap();
    assert_eq!(seen.last().unwrap().0, "success");

    let a = Assistant::new(c.clone());
    let m = a
        .suggest_mapping(
            "Loan from Suresh Kumar",
            "Sundry Creditors",
            "50,000.00 Cr",
            &[Head::LtBorrowings, Head::TradePayables],
        )
        .unwrap();
    assert_eq!(m.head, Head::LtBorrowings);

    // A head outside the allowed list is rejected, never applied.
    let a2 = Assistant::new(OllamaClient::new(
        &mock(r#"{"head":"REVENUE_OPS","reason":"x"}"#),
        "qwen2.5:3b",
    ));
    assert!(a2
        .suggest_mapping("Loan", "Sundry Creditors", "1 Cr", &[Head::LtBorrowings])
        .is_err());

    let a3 = Assistant::new(OllamaClient::new(
        &mock("यह नकद भुगतान सीमा से अधिक है।"),
        "qwen2.5:3b",
    ));
    let f = lc_core::checks::Finding {
        detection_basis: String::new(),
        unknown_facts: vec![],
        possible_exceptions: vec![],
        verification_status: String::new(),
        professional_review_required: false,
        blocks_final: false,
        code: "CASH_PAYMENT_LIMIT".into(),
        severity: lc_core::rules::Severity::Warning,
        title: "Cash payment above limit".into(),
        message: "₹12,500 paid in cash".into(),
        legal_ref: "s.40A(3)".into(),
        ledger: Some("Repairs".into()),
        voucher: None,
        date: None,
        amount: None,
        suggestion: None,
        key: "k".into(),
    };
    let s = a3.explain_finding(&f, Language::Hindi).unwrap();
    assert!(s.label.contains("AI suggestion"));
}

#[test]
fn friendly_errors() {
    let c = OllamaClient::new("http://127.0.0.1:1", "qwen2.5:3b");
    let e = c.version().unwrap_err();
    assert!(matches!(e, OllamaError::NotRunning(_)));
    assert!(e.to_string().contains("works without it"));
}

/// Live check against the Ollama installed on this machine.
#[test]
fn live_local_ollama() {
    if std::env::var("LC_OLLAMA_LIVE").is_err() {
        return;
    }
    let model = std::env::var("LC_OLLAMA_MODEL").unwrap_or_else(|_| "qwen2.5:3b".into());
    let c = OllamaClient::new("http://127.0.0.1:11434", &model);
    let v = c.version().expect("Ollama is running");
    println!("live Ollama version {v}, models: {:?}", c.models().unwrap());
    if !c.has_model().unwrap() {
        let e = c.chat("s", "hi", None).unwrap_err();
        assert!(matches!(e, OllamaError::ModelMissing(_)), "{e:?}");
        println!("live: model missing reported as: {e}");
    } else {
        println!(
            "live reply: {}",
            c.chat("Reply in one word.", "Say OK", None).unwrap()
        );
        // Constrained JSON: even a weak model can only answer with an allowed head.
        let a = Assistant::new(c.clone());
        match a.suggest_mapping(
            "Loan from Suresh Kumar",
            "Sundry Creditors",
            "50,000.00 Cr",
            &[Head::LtBorrowings, Head::TradePayables],
        ) {
            Ok(m) => println!(
                "live mapping suggestion: {:?} – {}",
                m.head,
                m.reason.chars().take(60).collect::<String>()
            ),
            Err(e) => println!("live mapping rejected safely: {e}"),
        }
    }
}
