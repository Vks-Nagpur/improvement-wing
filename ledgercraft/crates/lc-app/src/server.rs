//! Local HTTP API and static UI. Binds to 127.0.0.1 only: nothing is reachable
//! from the network, and no data leaves the PC (except an AI model download
//! the user starts explicitly).

use crate::store::{write_atomic, Store};
use lc_ai::{Assistant, Language, OllamaClient};
use lc_core::mapping::Head;
use lc_core::rules::{RulesPack, Severity};
use lc_core::Analysis;
use lc_io::export::{export, ExportOptions, Mode};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::io::Read;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

const INDEX: &str = include_str!("ui/index.html");
const APP_JS: &str = include_str!("ui/app.js");
const APP_CSS: &str = include_str!("ui/app.css");

#[derive(Default, Clone, serde::Serialize)]
pub struct PullState {
    pub running: bool,
    pub model: String,
    pub status: String,
    pub completed: u64,
    pub total: u64,
    pub error: Option<String>,
    pub done: bool,
}

pub struct App {
    pub store: Store,
    pub ollama_url: String,
    pub model: Mutex<String>,
    pub cache: Mutex<HashMap<String, Analysis>>,
    pub pull: Arc<Mutex<PullState>>,
    pub quit: Mutex<bool>,
    /// Per-run secret: every /api call must carry it (blocks other websites).
    pub token: String,
}

pub struct Reply {
    pub status: u16,
    pub content_type: &'static str,
    pub body: Vec<u8>,
}

fn ok(v: Value) -> Reply {
    Reply {
        status: 200,
        content_type: "application/json; charset=utf-8",
        body: v.to_string().into_bytes(),
    }
}
fn bad(msg: impl Into<String>) -> Reply {
    Reply {
        status: 400,
        content_type: "application/json; charset=utf-8",
        body: json!({"error": msg.into()}).to_string().into_bytes(),
    }
}

fn decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        match b[i] {
            b'+' => out.push(b' '),
            b'%' if i + 2 < b.len() => {
                if let Ok(v) = u8::from_str_radix(&s[i + 1..i + 3], 16) {
                    out.push(v);
                    i += 2;
                } else {
                    out.push(b'%');
                }
            }
            c => out.push(c),
        }
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn query(q: &str) -> HashMap<String, String> {
    q.split('&')
        .filter_map(|kv| kv.split_once('='))
        .map(|(k, v)| (decode(k), decode(v)))
        .collect()
}

fn s(v: &Value, k: &str) -> String {
    v.get(k).and_then(|x| x.as_str()).unwrap_or("").to_string()
}

impl App {
    pub fn new(data: PathBuf, ollama_url: &str) -> Result<App, String> {
        Ok(App {
            store: Store::new(data)?,
            ollama_url: ollama_url.into(),
            model: Mutex::new(lc_ai::RECOMMENDED_MODELS[0].0.into()),
            cache: Mutex::new(HashMap::new()),
            pull: Arc::new(Mutex::new(PullState::default())),
            quit: Mutex::new(false),
            token: {
                use std::hash::{BuildHasher, Hasher};
                let mut t = String::new();
                for _ in 0..2 {
                    let mut h = std::collections::hash_map::RandomState::new().build_hasher();
                    h.write_u128(
                        std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .map(|d| d.as_nanos())
                            .unwrap_or(0),
                    );
                    h.write_u32(std::process::id());
                    t.push_str(&format!("{:016x}", h.finish()));
                }
                t
            },
        })
    }

    fn ai(&self) -> OllamaClient {
        OllamaClient::new(&self.ollama_url, &self.model.lock().unwrap())
    }

    pub fn handle(&self, method: &str, url: &str, body: &[u8]) -> Reply {
        let (path, q) = url.split_once('?').unwrap_or((url, ""));
        let q = query(q);
        let parts: Vec<String> = path.trim_matches('/').split('/').map(decode).collect();
        let p: Vec<&str> = parts.iter().map(|s| s.as_str()).collect();
        let json_body = || -> Value { serde_json::from_slice(body).unwrap_or(Value::Null) };
        let r = match (method, p.as_slice()) {
            ("GET", [""]) | ("GET", ["index.html"]) => return Reply { status: 200, content_type: "text/html; charset=utf-8", body: INDEX.replace("__LC_TOKEN__", &self.token).into_bytes() },
            ("GET", ["app.js"]) => return Reply { status: 200, content_type: "text/javascript; charset=utf-8", body: APP_JS.as_bytes().to_vec() },
            ("GET", ["app.css"]) => return Reply { status: 200, content_type: "text/css; charset=utf-8", body: APP_CSS.as_bytes().to_vec() },
            ("GET", ["api", "status"]) => Ok(self.status()),
            ("GET", ["api", "heads"]) => Ok(json!(Head::ALL.iter().filter(|h| **h != Head::ChangeInInventories).map(|h| json!({"id": h.id(), "label": h.label(), "nature": format!("{:?}", h.nature())})).collect::<Vec<_>>())),
            ("GET", ["api", "projects"]) => Ok(json!(self.store.list())),
            ("POST", ["api", "projects"]) => {
                let b = json_body();
                self.store.create(&s(&b, "name"), &s(&b, "entity_type"), &s(&b, "fy")).map(|id| json!({"id": id}))
            }
            ("GET", ["api", "projects", id]) => self.project_info(id),
            ("POST", ["api", "projects", id, "upload"]) => self.upload(id, q.get("kind").map(|s| s.as_str()).unwrap_or(""), q.get("name").map(|s| s.as_str()).unwrap_or("file"), body),
            ("GET", ["api", "tally", "companies"]) => {
                let port = q.get("port").and_then(|p| p.parse().ok()).unwrap_or(9000);
                lc_io::tally::TallyClient::new(q.get("host").map(|s| s.as_str()).unwrap_or("localhost"), port).companies().map(|c| json!(c))
            }
            ("POST", ["api", "projects", id, "tally"]) => self.tally(id, &json_body()),
            ("POST", ["api", "projects", id, "analyse"]) => self.analyse(id),
            ("POST", ["api", "projects", id, "mapping"]) => self.set_mapping(id, &json_body()),
            ("POST", ["api", "projects", id, "tags"]) => self.set_tags(id, &json_body()),
            ("POST", ["api", "projects", id, "settings"]) => self.set_settings(id, &json_body()),
            ("GET", ["api", "projects", id, "preview"]) => match self.preview(id) {
                Ok(html) => return Reply { status: 200, content_type: "text/html; charset=utf-8", body: html.into_bytes() },
                Err(e) => return Reply { status: 400, content_type: "text/html; charset=utf-8", body: format!("<p style='font-family:sans-serif;padding:24px'>{e}</p>").into_bytes() },
            },
            ("POST", ["api", "projects", id, "export"]) => self.do_export(id, &json_body()),
            ("GET", ["api", "projects", id, "audit"]) => self.store.project(id).map(|p| json!({"events": p.audit(), "status": p.verify_audit()})),
            ("GET", ["api", "ai", "setup"]) => Ok(json!(*self.pull.lock().unwrap())),
            ("POST", ["api", "ai", "setup"]) => self.ai_setup(&json_body()),
            ("POST", ["api", "projects", id, "ai", what]) => self.ai_call(id, what, &json_body()),
            ("POST", ["api", "quit"]) => {
                *self.quit.lock().unwrap() = true;
                Ok(json!({"ok": true}))
            }
            _ => return Reply { status: 404, content_type: "application/json; charset=utf-8", body: json!({"error": format!("not found: {method} {path}")}).to_string().into_bytes() },
        };
        match r {
            Ok(v) => ok(v),
            Err(e) => bad(e),
        }
    }

    fn status(&self) -> Value {
        let c = self.ai();
        let (running, version, models) = match c.version() {
            Ok(v) => (
                true,
                v,
                c.models()
                    .unwrap_or_default()
                    .into_iter()
                    .map(|m| m.name)
                    .collect::<Vec<_>>(),
            ),
            Err(_) => (false, String::new(), vec![]),
        };
        json!({
            "app": "LedgerCraft",
            "version": env!("CARGO_PKG_VERSION"),
            "data_dir": self.store.root.display().to_string(),
            "ai": {"running": running, "version": version, "models": models, "model": c.model, "recommended": lc_ai::RECOMMENDED_MODELS.iter().map(|(m, d)| json!({"name": m, "about": d})).collect::<Vec<_>>()},
        })
    }

    fn project_info(&self, id: &str) -> Result<Value, String> {
        let p = self.store.project(id)?;
        let st = p.load_settings()?;
        Ok(json!({"id": id, "settings": st, "audit": p.verify_audit()}))
    }

    fn invalidate(&self, id: &str) {
        self.cache.lock().unwrap().remove(id);
    }

    fn upload(&self, id: &str, kind: &str, name: &str, body: &[u8]) -> Result<Value, String> {
        let p = self.store.project(id)?;
        let mut st = p.load_settings()?;
        let ext = std::path::Path::new(name)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        if !matches!(ext.as_str(), "xlsx" | "xls" | "csv" | "xlsm" | "ods") {
            return Err("Please choose an Excel (.xlsx/.xls) or CSV file.".into());
        }
        if body.is_empty() {
            return Err("The file is empty.".into());
        }
        let file = format!("{kind}.{ext}");
        // Validate before accepting, so a wrong file never replaces a good one.
        let tmp = p.inputs_dir().join(format!("check-{file}"));
        write_atomic(&tmp, body)?;
        let checked = match kind {
            "tb" | "py_tb" => lc_io::read::read_trial_balance_with(
                &tmp,
                st.inputs
                    .accounts_master
                    .as_ref()
                    .map(|f| p.inputs_dir().join(f))
                    .as_deref(),
            )
            .map(|t| format!("{} ledgers", t.ledgers.len())),
            "vouchers" => lc_io::read::read_vouchers(&tmp).map(|v| format!("{} vouchers", v.len())),
            "far" => lc_io::read::read_far(&tmp, st.depreciation_basis)
                .map(|r| format!("{} assets", r.assets.len())),
            "accounts_master" => Ok("account master".to_string()),
            _ => Err("unknown file kind".to_string()),
        };
        let what = match checked {
            Ok(w) => w,
            Err(e) => {
                let _ = std::fs::remove_file(&tmp);
                return Err(format!("This file could not be read: {e}"));
            }
        };
        std::fs::rename(&tmp, p.inputs_dir().join(&file)).map_err(|e| e.to_string())?;
        let digest = format!("{:x}", <sha2::Sha256 as sha2::Digest>::digest(body));
        match kind {
            "tb" => st.inputs.tb = Some(file.clone()),
            "py_tb" => st.inputs.py_tb = Some(file.clone()),
            "vouchers" => st.inputs.vouchers = Some(file.clone()),
            "far" => st.inputs.far = Some(file.clone()),
            _ => st.inputs.accounts_master = Some(file.clone()),
        }
        if kind == "tb" {
            st.inputs.tally_company = None;
        }
        p.save_settings(&st)?;
        p.log("user", "file_imported", json!({"kind": kind, "original_name": name, "bytes": body.len(), "sha256": digest, "contents": what}))?;
        self.invalidate(id);
        Ok(json!({"ok": true, "contents": what}))
    }

    fn tally(&self, id: &str, b: &Value) -> Result<Value, String> {
        let p = self.store.project(id)?;
        let mut st = p.load_settings()?;
        let (fy_start, fy_end) = lc_core::date::parse_fy(&st.fy).ok_or("bad financial year")?;
        let host = if s(b, "host").is_empty() {
            "localhost".into()
        } else {
            s(b, "host")
        };
        let port = b.get("port").and_then(|x| x.as_u64()).unwrap_or(9000) as u16;
        let company = s(b, "company");
        let client = lc_io::tally::TallyClient::new(&host, port);
        let with_v = b.get("vouchers").and_then(|x| x.as_bool()).unwrap_or(true);
        let (cy, py, vouchers) =
            lc_io::tally::import_year(&client, &company, fy_start, fy_end, with_v, |_, _| {})?;
        let inp = p.inputs_dir();
        write_atomic(
            &inp.join("tb.json"),
            serde_json::to_string(&cy)
                .map_err(|e| e.to_string())?
                .as_bytes(),
        )?;
        st.inputs.tb = Some("tb.json".into());
        if let Some(py) = &py {
            write_atomic(
                &inp.join("py_tb.json"),
                serde_json::to_string(py)
                    .map_err(|e| e.to_string())?
                    .as_bytes(),
            )?;
            st.inputs.py_tb = Some("py_tb.json".into());
        }
        if with_v {
            write_atomic(
                &inp.join("vouchers.json"),
                serde_json::to_string(&vouchers)
                    .map_err(|e| e.to_string())?
                    .as_bytes(),
            )?;
            st.inputs.vouchers = Some("vouchers.json".into());
        }
        st.inputs.tally_company = Some(company.clone());
        p.save_settings(&st)?;
        p.log("user", "tally_import", json!({"company": company, "host": host, "port": port, "ledgers": cy.ledgers.len(), "previous_year": py.is_some(), "vouchers": vouchers.len()}))?;
        self.invalidate(id);
        Ok(
            json!({"ledgers": cy.ledgers.len(), "previous_year": py.is_some(), "vouchers": vouchers.len()}),
        )
    }

    fn analysis(&self, id: &str) -> Result<(lc_core::Engagement, Analysis), String> {
        let p = self.store.project(id)?;
        let eng = p.engagement()?;
        if let Some(a) = self.cache.lock().unwrap().get(id) {
            return Ok((eng, a.clone()));
        }
        let a = lc_core::analyse(&eng, &RulesPack::builtin());
        self.cache.lock().unwrap().insert(id.to_string(), a.clone());
        Ok((eng, a))
    }

    fn analyse(&self, id: &str) -> Result<Value, String> {
        self.invalidate(id);
        let p = self.store.project(id)?;
        let st = p.load_settings()?;
        let (eng, a) = self.analysis(id)?;
        let rep = lc_core::report::build(&eng, &a, &st.options, &st.signoff);
        let mapping: Vec<Value> = a
            .mapping
            .iter()
            .map(|m| {
                json!({
                    "name": m.name, "group": m.group, "standard_group": m.class.map(|c| c.label()),
                    "head": m.head.map(|h| h.id()), "head_label": m.head.map(|h| h.label()),
                    "source": m.source, "reclassified": m.reclassified, "amount": m.amount.fmt_drcr(), "tags": st.tags.get(&m.name).cloned().unwrap_or_default(),
                })
            })
            .collect();
        p.log("user", "checks_run", json!({"must_fix": a.count(Severity::Blocker), "check": a.count(Severity::Warning), "notes": a.count(Severity::Info), "rules_version": a.rules_version}))?;
        Ok(json!({
            "summary": {
                "entity": eng.entity_name, "entity_type": eng.entity_type.label(), "fy": st.fy,
                "ledgers": eng.cy.ledgers.len(), "vouchers": eng.vouchers.len(), "has_previous_year": eng.py.is_some(), "has_far": eng.far.is_some(),
                "must_fix": a.count(Severity::Blocker), "check": a.count(Severity::Warning), "notes": a.count(Severity::Info),
                "profit": a.facts_cy.profit().fmt_indian(), "total_assets": a.facts_cy.total_assets().fmt_indian(), "printable": a.printable,
            },
            "findings": a.findings,
            "mapping": mapping,
            "ratios": a.ratios,
            "loans": a.loans,
            "warnings": rep.warnings,
        }))
    }

    fn set_mapping(&self, id: &str, b: &Value) -> Result<Value, String> {
        let p = self.store.project(id)?;
        let ledger = s(b, "ledger");
        let head = s(b, "head");
        let key = lc_core::model::norm_name(&ledger);
        let mut m = p.mapping();
        let old = m.get(&key).cloned();
        if head.is_empty() {
            m.remove(&key);
        } else {
            Head::from_id(&head).ok_or("unknown head")?;
            m.insert(key, head.clone());
        }
        p.save_mapping(&m)?;
        p.log("user", "mapping_changed", json!({"ledger": ledger, "from": old, "to": if head.is_empty() { Value::Null } else { json!(head) }, "ai_suggested": b.get("ai").and_then(|x| x.as_bool()).unwrap_or(false)}))?;
        self.invalidate(id);
        Ok(json!({"ok": true}))
    }

    fn set_tags(&self, id: &str, b: &Value) -> Result<Value, String> {
        let p = self.store.project(id)?;
        let mut st = p.load_settings()?;
        let ledger = s(b, "ledger");
        let tags: Vec<String> = b
            .get("tags")
            .and_then(|t| t.as_array())
            .into_iter()
            .flatten()
            .filter_map(|t| t.as_str().map(|x| x.trim().to_lowercase()))
            .filter(|t| !t.is_empty())
            .collect();
        let old = st.tags.get(&ledger).cloned();
        if tags.is_empty() {
            st.tags.remove(&ledger);
        } else {
            st.tags.insert(ledger.clone(), tags.clone());
        }
        p.save_settings(&st)?;
        p.log(
            "user",
            "tags_changed",
            json!({"ledger": ledger, "from": old, "to": tags}),
        )?;
        self.invalidate(id);
        Ok(json!({"ok": true}))
    }

    fn set_settings(&self, id: &str, b: &Value) -> Result<Value, String> {
        let p = self.store.project(id)?;
        let mut st = p.load_settings()?;
        let before = serde_json::to_value(&st).map_err(|e| e.to_string())?;
        if let Some(o) = b.get("options") {
            st.options = serde_json::from_value(o.clone()).map_err(|e| format!("options: {e}"))?;
        }
        if let Some(o) = b.get("signoff") {
            st.signoff = serde_json::from_value(o.clone()).map_err(|e| format!("sign-off: {e}"))?;
        }
        if let Some(o) = b.get("profit_sharing") {
            st.profit_sharing =
                serde_json::from_value(o.clone()).map_err(|e| format!("profit sharing: {e}"))?;
        }
        if let Some(o) = b.get("depreciation_basis") {
            st.depreciation_basis = serde_json::from_value(o.clone())
                .map_err(|e| format!("depreciation basis: {e}"))?;
        }
        p.save_settings(&st)?;
        let after = serde_json::to_value(&st).map_err(|e| e.to_string())?;
        let mut changed = serde_json::Map::new();
        for k in ["options", "signoff", "profit_sharing", "depreciation_basis"] {
            if before.get(k) != after.get(k) {
                changed.insert(k.into(), json!({"from": before.get(k), "to": after.get(k)}));
            }
        }
        if !changed.is_empty() {
            p.log("user", "settings_changed", Value::Object(changed))?;
        }
        self.invalidate(id);
        Ok(json!({"ok": true}))
    }

    fn preview(&self, id: &str) -> Result<String, String> {
        let p = self.store.project(id)?;
        let st = p.load_settings()?;
        let (eng, a) = self.analysis(id)?;
        let mut o = st.options.clone();
        o.draft = true;
        Ok(lc_io::render::html::render(&lc_core::report::build(
            &eng,
            &a,
            &o,
            &st.signoff,
        )))
    }

    fn do_export(&self, id: &str, b: &Value) -> Result<Value, String> {
        let p = self.store.project(id)?;
        let st = p.load_settings()?;
        let (eng, a) = self.analysis(id)?;
        let mode = if s(b, "mode") == "final" {
            Mode::Signing
        } else {
            Mode::Draft
        };
        let folder = s(b, "folder");
        let root = if folder.trim().is_empty() {
            self.store.root.join("Exports")
        } else {
            PathBuf::from(folder)
        };
        let ex = export(
            &root,
            &eng,
            &a,
            &st.signoff,
            &ExportOptions {
                mode,
                report: st.options.clone(),
                ..Default::default()
            },
        )?;
        let manifest: Value = std::fs::read_to_string(ex.dir.join("export-manifest.json"))
            .ok()
            .and_then(|t| serde_json::from_str(&t).ok())
            .unwrap_or(Value::Null);
        p.log("user", "exported", json!({"mode": if mode == Mode::Signing { "final" } else { "draft" }, "folder": ex.dir.display().to_string(), "files": manifest.get("files"), "udin": st.signoff.udin}))?;
        Ok(
            json!({"dir": ex.dir.display().to_string(), "files": manifest.get("files"), "warnings": ex.warnings}),
        )
    }

    fn ai_setup(&self, b: &Value) -> Result<Value, String> {
        let model = if s(b, "model").is_empty() {
            self.model.lock().unwrap().clone()
        } else {
            s(b, "model")
        };
        *self.model.lock().unwrap() = model.clone();
        let mut st = self.pull.lock().unwrap();
        if st.running {
            return Err("A download is already running.".into());
        }
        *st = PullState {
            running: true,
            model: model.clone(),
            status: "starting".into(),
            ..Default::default()
        };
        drop(st);
        let state = self.pull.clone();
        let client = OllamaClient::new(&self.ollama_url, &model);
        std::thread::spawn(move || {
            let r = client.pull(|status, done, total| {
                let mut s = state.lock().unwrap();
                s.status = status.to_string();
                s.completed = done;
                s.total = total;
            });
            let mut s = state.lock().unwrap();
            s.running = false;
            s.done = r.is_ok();
            s.error = r.err().map(|e| e.to_string());
        });
        Ok(json!({"started": model}))
    }

    fn ai_call(&self, id: &str, what: &str, b: &Value) -> Result<Value, String> {
        let p = self.store.project(id)?;
        let (eng, a) = self.analysis(id)?;
        let lang = match s(b, "language").as_str() {
            "hindi" => Language::Hindi,
            "marathi" => Language::Marathi,
            _ => Language::English,
        };
        let assistant = Assistant::new(self.ai());
        let out = match what {
            "explain" => {
                let key = s(b, "key");
                let f = a
                    .findings
                    .iter()
                    .find(|f| f.key == key)
                    .ok_or("finding not found – run the checks again")?;
                let r = assistant
                    .explain_finding(f, lang)
                    .map_err(|e| e.to_string())?;
                json!({"text": r.text, "label": r.label, "model": r.model})
            }
            "map" => {
                let ledger = s(b, "ledger");
                let m = a
                    .mapping
                    .iter()
                    .find(|m| m.name == ledger)
                    .ok_or("ledger not found")?;
                let allowed: Vec<Head> = Head::ALL
                    .iter()
                    .copied()
                    .filter(|h| *h != Head::ChangeInInventories)
                    .collect();
                let r = assistant
                    .suggest_mapping(&m.name, &m.group, &m.amount.fmt_drcr(), &allowed)
                    .map_err(|e| e.to_string())?;
                json!({"head": r.head.id(), "head_label": r.head.label(), "reason": r.reason, "label": "AI suggestion – review before use", "model": r.model})
            }
            "ratio" => {
                let name = s(b, "name");
                let r = a
                    .ratios
                    .iter()
                    .find(|r| r.name == name)
                    .ok_or("ratio not found")?;
                let ctx = format!(
                    "Revenue {}; profit {}; total assets {}.",
                    a.facts_cy.head(Head::RevenueOps).fmt_indian(),
                    a.facts_cy.profit().fmt_indian(),
                    a.facts_cy.total_assets().fmt_indian()
                );
                let r = assistant
                    .draft_ratio_reason(r, &ctx)
                    .map_err(|e| e.to_string())?;
                json!({"text": r.text, "label": r.label, "model": r.model})
            }
            "ask" => {
                let ctx = format!(
                    "Entity: {} ({}), FY {}. Revenue {}. Profit {}. Total assets {}. Findings: {}.",
                    eng.entity_name,
                    eng.entity_type.label(),
                    lc_core::date::fy_label(eng.fy_start),
                    a.facts_cy.head(Head::RevenueOps).fmt_indian(),
                    a.facts_cy.profit().fmt_indian(),
                    a.facts_cy.total_assets().fmt_indian(),
                    a.findings
                        .iter()
                        .take(40)
                        .map(|f| f.message.clone())
                        .collect::<Vec<_>>()
                        .join(" | ")
                );
                let r = assistant
                    .ask(&s(b, "question"), &ctx, lang)
                    .map_err(|e| e.to_string())?;
                json!({"text": r.text, "label": r.label, "model": r.model})
            }
            _ => return Err("unknown AI action".into()),
        };
        p.log(
            "ai",
            "ai_suggestion",
            json!({"action": what, "request": b, "response": out}),
        )?;
        Ok(out)
    }
}

/// Serve until /api/quit. Returns the bound address.
pub fn serve(app: Arc<App>, port: u16, on_ready: impl FnOnce(String)) -> Result<(), String> {
    let server = tiny_http::Server::http(("127.0.0.1", port))
        .map_err(|e| format!("cannot start on port {port}: {e}"))?;
    let addr = format!(
        "http://{}",
        server.server_addr().to_ip().ok_or("no address")?
    );
    on_ready(addr);
    for mut req in server.incoming_requests() {
        let method = req.method().as_str().to_string();
        let url = req.url().to_string();
        // Only this app, on this machine: check Host and the session token.
        let host_ok = req
            .headers()
            .iter()
            .find(|h| h.field.equiv("Host"))
            .map(|h| {
                let v = h.value.as_str();
                v.starts_with("127.0.0.1:") || v.starts_with("localhost:")
            })
            .unwrap_or(false);
        let token_ok = req
            .headers()
            .iter()
            .any(|h| h.field.equiv("X-LC-Token") && h.value.as_str() == app.token)
            || url.contains(&format!("t={}", app.token));
        if !host_ok || (url.starts_with("/api/") && !token_ok) {
            let _ =
                req.respond(tiny_http::Response::from_string("forbidden").with_status_code(403));
            continue;
        }
        let mut body = Vec::new();
        if let Err(e) = req.as_reader().take(1 << 30).read_to_end(&mut body) {
            let _ = req.respond(
                tiny_http::Response::from_string(format!("read error: {e}")).with_status_code(400),
            );
            continue;
        }
        let r = app.handle(&method, &url, &body);
        let resp = tiny_http::Response::from_data(r.body)
            .with_status_code(r.status)
            .with_header(
                tiny_http::Header::from_bytes(&b"Content-Type"[..], r.content_type.as_bytes())
                    .unwrap(),
            )
            .with_header(
                tiny_http::Header::from_bytes(&b"Cache-Control"[..], &b"no-store"[..]).unwrap(),
            )
            .with_header(
                tiny_http::Header::from_bytes(&b"X-Content-Type-Options"[..], &b"nosniff"[..])
                    .unwrap(),
            );
        let _ = req.respond(resp);
        if *app.quit.lock().unwrap() {
            break;
        }
    }
    Ok(())
}
