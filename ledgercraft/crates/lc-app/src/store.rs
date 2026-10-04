//! Project storage. One folder per client and financial year:
//!
//! ```text
//! <data>/clients/<client>/mapping.json        remembered mapping (reused every year)
//! <data>/clients/<client>/<fy>/settings.json  options, sign-off, tags, inputs
//! <data>/clients/<client>/<fy>/inputs/…       imported files / Tally snapshots
//! <data>/clients/<client>/<fy>/audit.jsonl    tamper-evident audit trail
//! ```
//! Every write is atomic (temporary file, then rename). The audit trail is
//! append-only and hash-chained: each event carries the SHA-256 of the
//! previous one, so any edit or deletion is detected on verification.

use chrono::Local;
use lc_core::far::BookBasis;
use lc_core::model::{Engagement, EntityType, TrialBalance, Voucher};
use lc_core::report::{ReportOptions, SignOff};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

pub fn slug(s: &str) -> String {
    let mut out = String::new();
    for c in s.trim().chars() {
        if c.is_alphanumeric() {
            out.push(c);
        } else if matches!(c, ' ' | '-' | '_' | '.' | '&') && !out.ends_with('-') {
            out.push('-');
        }
    }
    out.trim_matches('-').to_string()
}

pub fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), String> {
    if let Some(p) = path.parent() {
        fs::create_dir_all(p).map_err(|e| e.to_string())?;
    }
    let tmp = path.with_extension(format!("tmp{}", std::process::id()));
    {
        let mut f = fs::File::create(&tmp).map_err(|e| e.to_string())?;
        f.write_all(bytes).map_err(|e| e.to_string())?;
        f.sync_all().map_err(|e| e.to_string())?;
    }
    fs::rename(&tmp, path).map_err(|e| e.to_string())
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Inputs {
    pub tb: Option<String>,
    pub py_tb: Option<String>,
    pub vouchers: Option<String>,
    pub far: Option<String>,
    pub accounts_master: Option<String>,
    /// Set when the data came from Tally (snapshots stored as JSON).
    pub tally_company: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Settings {
    pub entity_name: String,
    pub entity_type: String,
    pub fy: String,
    pub options: ReportOptions,
    pub signoff: SignOff,
    pub profit_sharing: Vec<(String, u32)>,
    /// Ledger → tags (msme, transporter, disputed, doubtful, exempt, related).
    pub tags: BTreeMap<String, Vec<String>>,
    pub depreciation_basis: BookBasis,
    pub inputs: Inputs,
    /// Manual adjustment entries over the imported books.
    pub adjustments: Vec<lc_core::adjust::Adjustment>,
    pub created: String,
    pub modified: String,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            entity_name: String::new(),
            entity_type: "firm".into(),
            fy: String::new(),
            options: ReportOptions::default(),
            signoff: SignOff::default(),
            profit_sharing: Vec::new(),
            tags: BTreeMap::new(),
            depreciation_basis: BookBasis::IncomeTaxRates,
            inputs: Inputs::default(),
            adjustments: Vec::new(),
            created: String::new(),
            modified: String::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AuditEvent {
    pub seq: u64,
    pub ts: String,
    pub actor: String,
    pub action: String,
    pub details: Value,
    pub prev: String,
    pub hash: String,
}

fn event_hash(
    seq: u64,
    ts: &str,
    actor: &str,
    action: &str,
    details: &Value,
    prev: &str,
) -> String {
    let body = format!(
        "{seq}|{ts}|{actor}|{action}|{}|{prev}",
        serde_json::to_string(details).unwrap_or_default()
    );
    format!("{:x}", Sha256::digest(body.as_bytes()))
}

/// Result of checking the audit trail.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ChainStatus {
    pub events: usize,
    pub intact: bool,
    pub problem: Option<String>,
}

pub struct Project {
    pub dir: PathBuf,
    pub client_dir: PathBuf,
}

impl Project {
    pub fn settings_path(&self) -> PathBuf {
        self.dir.join("settings.json")
    }

    pub fn load_settings(&self) -> Result<Settings, String> {
        let s = fs::read_to_string(self.settings_path())
            .map_err(|e| format!("cannot open project: {e}"))?;
        serde_json::from_str(&s).map_err(|e| format!("project settings damaged: {e}"))
    }

    pub fn save_settings(&self, s: &Settings) -> Result<(), String> {
        let mut s = s.clone();
        s.modified = Local::now().to_rfc3339();
        write_atomic(
            &self.settings_path(),
            serde_json::to_string_pretty(&s)
                .map_err(|e| e.to_string())?
                .as_bytes(),
        )
    }

    pub fn mapping(&self) -> HashMap<String, String> {
        fs::read_to_string(self.client_dir.join("mapping.json"))
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    // ---- pinned rule and format packs (TRUTH-MODEL §8) ------------------------
    fn pins_dir(&self) -> PathBuf {
        self.dir.join("pins")
    }

    /// Pin this client year to the packs shipped with this build, if not yet pinned.
    pub fn ensure_pins(&self) -> Result<(), String> {
        let d = self.pins_dir();
        if d.join("rules.json").exists() && d.join("format.json").exists() {
            return Ok(());
        }
        let s = self.load_settings()?;
        let et = EntityType::parse(&s.entity_type).ok_or("unknown entity type")?;
        write_atomic(
            &d.join("rules.json"),
            lc_core::rules::builtin_text().as_bytes(),
        )?;
        write_atomic(
            &d.join("format.json"),
            lc_core::statements::FormatPack::builtin_text(et).as_bytes(),
        )?;
        let r = lc_core::rules::RulesPack::builtin();
        let f = lc_core::statements::FormatPack::for_entity(et);
        self.log(
            "system",
            "rules_pinned",
            serde_json::json!({"rules_version": r.version, "format_pack": f.id, "format_status": f.status}),
        )?;
        Ok(())
    }

    pub fn rules_pack(&self) -> Result<lc_core::rules::RulesPack, String> {
        self.ensure_pins()?;
        let t =
            fs::read_to_string(self.pins_dir().join("rules.json")).map_err(|e| e.to_string())?;
        lc_core::rules::RulesPack::from_json(&t).map_err(|e| format!("pinned rules pack: {e}"))
    }

    pub fn format_pack(&self) -> Result<lc_core::statements::FormatPack, String> {
        self.ensure_pins()?;
        let t =
            fs::read_to_string(self.pins_dir().join("format.json")).map_err(|e| e.to_string())?;
        serde_json::from_str(&t).map_err(|e| format!("pinned format pack: {e}"))
    }

    /// Move this year to the packs of this build. Returns what changed.
    pub fn migrate_pins(&self) -> Result<serde_json::Value, String> {
        let old_r = self.rules_pack()?;
        let old_f = self.format_pack()?;
        let s = self.load_settings()?;
        let et = EntityType::parse(&s.entity_type).ok_or("unknown entity type")?;
        let new_r = lc_core::rules::RulesPack::builtin();
        let new_f = lc_core::statements::FormatPack::for_entity(et);
        let changes = lc_core::rules::changed_rules(&old_r, &new_r);
        let d = self.pins_dir();
        write_atomic(
            &d.join("rules.json"),
            lc_core::rules::builtin_text().as_bytes(),
        )?;
        write_atomic(
            &d.join("format.json"),
            lc_core::statements::FormatPack::builtin_text(et).as_bytes(),
        )?;
        let v = serde_json::json!({
            "rules_from": old_r.version, "rules_to": new_r.version, "rule_changes": changes,
            "format_changed": old_f != new_f, "format_pack": new_f.id,
        });
        self.log("user", "rules_migrated", v.clone())?;
        Ok(v)
    }

    /// Group and balance side at the time each mapping was confirmed.
    pub fn mapping_context(&self) -> HashMap<String, String> {
        fs::read_to_string(self.client_dir.join("mapping_context.json"))
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn save_mapping_context(&self, m: &HashMap<String, String>) -> Result<(), String> {
        write_atomic(
            &self.client_dir.join("mapping_context.json"),
            serde_json::to_string_pretty(m)
                .map_err(|e| e.to_string())?
                .as_bytes(),
        )
    }

    pub fn save_mapping(&self, m: &HashMap<String, String>) -> Result<(), String> {
        write_atomic(
            &self.client_dir.join("mapping.json"),
            serde_json::to_string_pretty(m)
                .map_err(|e| e.to_string())?
                .as_bytes(),
        )
    }

    pub fn inputs_dir(&self) -> PathBuf {
        self.dir.join("inputs")
    }

    /// Take an imported file out of the project. The file itself is kept in
    /// `inputs/removed/` so the audit trail can always be traced back.
    pub fn remove_input(&self, kind: &str) -> Result<String, String> {
        let mut st = self.load_settings()?;
        let slot = match kind {
            "tb" => &mut st.inputs.tb,
            "py_tb" => &mut st.inputs.py_tb,
            "vouchers" => &mut st.inputs.vouchers,
            "far" => &mut st.inputs.far,
            "accounts_master" => &mut st.inputs.accounts_master,
            _ => return Err("unknown file kind".into()),
        };
        let file = slot.take().ok_or("Nothing is loaded here.")?;
        let from = self.inputs_dir().join(&file);
        if from.exists() {
            let to = self
                .inputs_dir()
                .join("removed")
                .join(format!("{}-{file}", Local::now().format("%Y%m%d-%H%M%S")));
            fs::create_dir_all(to.parent().unwrap()).map_err(|e| e.to_string())?;
            fs::rename(&from, &to).map_err(|e| e.to_string())?;
        }
        if kind == "tb" {
            st.inputs.tally_company = None;
        }
        self.save_settings(&st)?;
        self.log(
            "user",
            "file_removed",
            serde_json::json!({"kind": kind, "file": file}),
        )?;
        Ok(file)
    }

    // ---- audit trail ---------------------------------------------------------
    fn audit_path(&self) -> PathBuf {
        self.dir.join("audit.jsonl")
    }

    pub fn audit(&self) -> Vec<AuditEvent> {
        fs::read_to_string(self.audit_path())
            .map(|s| {
                s.lines()
                    .filter(|l| !l.trim().is_empty())
                    .filter_map(|l| serde_json::from_str(l).ok())
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Append an event (append-only; cannot be switched off).
    pub fn log(&self, actor: &str, action: &str, details: Value) -> Result<AuditEvent, String> {
        let events = self.audit();
        let (seq, prev) = events
            .last()
            .map(|e| (e.seq + 1, e.hash.clone()))
            .unwrap_or((1, "0".repeat(64)));
        let ts = Local::now().to_rfc3339();
        let hash = event_hash(seq, &ts, actor, action, &details, &prev);
        let ev = AuditEvent {
            seq,
            ts,
            actor: actor.into(),
            action: action.into(),
            details,
            prev,
            hash,
        };
        let mut f = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(self.audit_path())
            .map_err(|e| e.to_string())?;
        writeln!(
            f,
            "{}",
            serde_json::to_string(&ev).map_err(|e| e.to_string())?
        )
        .map_err(|e| e.to_string())?;
        f.sync_all().map_err(|e| e.to_string())?;
        Ok(ev)
    }

    pub fn verify_audit(&self) -> ChainStatus {
        let raw = fs::read_to_string(self.audit_path()).unwrap_or_default();
        let mut prev = "0".repeat(64);
        let mut n = 0;
        for (i, line) in raw.lines().filter(|l| !l.trim().is_empty()).enumerate() {
            let e: AuditEvent = match serde_json::from_str(line) {
                Ok(e) => e,
                Err(_) => {
                    return ChainStatus {
                        events: n,
                        intact: false,
                        problem: Some(format!("line {} is not a valid event", i + 1)),
                    }
                }
            };
            if e.seq != i as u64 + 1 {
                return ChainStatus {
                    events: n,
                    intact: false,
                    problem: Some(format!("event {} is missing or out of order", i + 1)),
                };
            }
            if e.prev != prev
                || event_hash(e.seq, &e.ts, &e.actor, &e.action, &e.details, &e.prev) != e.hash
            {
                return ChainStatus {
                    events: n,
                    intact: false,
                    problem: Some(format!("event {} was altered after it was recorded", e.seq)),
                };
            }
            prev = e.hash;
            n += 1;
        }
        ChainStatus {
            events: n,
            intact: true,
            problem: None,
        }
    }

    // ---- building the engagement ---------------------------------------------
    /// The engagement with every active manual adjustment applied.
    pub fn engagement(&self) -> Result<Engagement, String> {
        Ok(self.engagement_adjusted()?.0)
    }

    pub fn engagement_adjusted(
        &self,
    ) -> Result<(Engagement, Vec<lc_core::adjust::Applied>), String> {
        let s = self.load_settings()?;
        let mut eng = self.engagement_books()?;
        let applied = lc_core::adjust::apply(&mut eng, &s.adjustments)?;
        Ok((eng, applied))
    }

    /// The engagement exactly as imported (no manual adjustments).
    pub fn engagement_books(&self) -> Result<Engagement, String> {
        let s = self.load_settings()?;
        let entity_type = EntityType::parse(&s.entity_type).ok_or("unknown entity type")?;
        let (fy_start, fy_end) =
            lc_core::date::parse_fy(&s.fy).ok_or("financial year must look like 2025-26")?;
        let inp = self.inputs_dir();
        let master = s.inputs.accounts_master.as_ref().map(|f| inp.join(f));
        let read_tb = |name: &Option<String>| -> Result<Option<TrialBalance>, String> {
            match name {
                Some(f) if f.ends_with(".json") => Ok(Some(
                    serde_json::from_str(
                        &fs::read_to_string(inp.join(f)).map_err(|e| e.to_string())?,
                    )
                    .map_err(|e| e.to_string())?,
                )),
                Some(f) => Ok(Some(lc_io::read::read_trial_balance_with(
                    &inp.join(f),
                    master.as_deref(),
                )?)),
                None => Ok(None),
            }
        };
        let mut cy = read_tb(&s.inputs.tb)?
            .ok_or("No trial balance yet – import from Tally or upload a file.")?;
        let mut py = read_tb(&s.inputs.py_tb)?;
        let vouchers: Vec<Voucher> = match &s.inputs.vouchers {
            Some(f) if f.ends_with(".json") => {
                serde_json::from_str(&fs::read_to_string(inp.join(f)).map_err(|e| e.to_string())?)
                    .map_err(|e| e.to_string())?
            }
            Some(f) => lc_io::read::read_vouchers(&inp.join(f))?,
            None => Vec::new(),
        };
        for tb in std::iter::once(&mut cy).chain(py.iter_mut()) {
            for l in tb.ledgers.iter_mut() {
                if let Some(t) = s.tags.get(&l.name) {
                    l.tags = t.clone();
                }
            }
        }
        let far = match &s.inputs.far {
            Some(f) => Some(lc_io::read::read_far(&inp.join(f), s.depreciation_basis)?),
            None => None,
        };
        Ok(Engagement {
            entity_name: s.entity_name.clone(),
            entity_type,
            fy_start,
            fy_end,
            cy,
            py,
            vouchers,
            mapping_memory: self.mapping(),
            mapping_context: self.mapping_context(),
            format_pack: Some(self.format_pack()?),
            far,
            profit_sharing: s.profit_sharing.clone(),
        })
    }
}

/// The data folder with all clients.
pub struct Store {
    pub root: PathBuf,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProjectSummary {
    pub id: String,
    pub entity_name: String,
    pub entity_type: String,
    pub fy: String,
    pub modified: String,
}

impl Store {
    pub fn new(root: PathBuf) -> Result<Store, String> {
        fs::create_dir_all(root.join("clients"))
            .map_err(|e| format!("cannot create data folder {}: {e}", root.display()))?;
        Ok(Store { root })
    }

    /// Project id is "<client>~<fy>".
    pub fn project(&self, id: &str) -> Result<Project, String> {
        let (c, fy) = id.split_once('~').ok_or("bad project id")?;
        if c.is_empty()
            || c.contains("..")
            || fy.contains("..")
            || c.contains('/')
            || c.contains('\\')
            || fy.contains('/')
            || fy.contains('\\')
        {
            return Err("bad project id".into());
        }
        let client_dir = self.root.join("clients").join(c);
        let dir = client_dir.join(fy);
        if !dir.join("settings.json").exists() {
            return Err(format!("project '{id}' not found"));
        }
        Ok(Project { dir, client_dir })
    }

    pub fn create(&self, name: &str, entity_type: &str, fy: &str) -> Result<String, String> {
        if name.trim().is_empty() {
            return Err("Please enter the entity name.".into());
        }
        EntityType::parse(entity_type).ok_or("Please choose the type of entity.")?;
        let (start, _) =
            lc_core::date::parse_fy(fy).ok_or("Financial year must look like 2025-26.")?;
        let fy = lc_core::date::fy_label(start);
        let id = format!("{}~{}", slug(name), fy);
        let client_dir = self.root.join("clients").join(slug(name));
        let dir = client_dir.join(&fy);
        if dir.join("settings.json").exists() {
            return Err(format!(
                "{name} – FY {fy} already exists. Open it from the list."
            ));
        }
        fs::create_dir_all(dir.join("inputs")).map_err(|e| e.to_string())?;
        let p = Project { dir, client_dir };
        // Carry presentation choices and sign-off from the previous year, if any.
        let mut s = Settings {
            entity_name: name.trim().into(),
            entity_type: entity_type.into(),
            fy: fy.clone(),
            created: Local::now().to_rfc3339(),
            ..Default::default()
        };
        let prev_fy = lc_core::date::fy_label(start.with_year_safe(-1));
        if let Ok(prev) = (Project {
            dir: p.client_dir.join(&prev_fy),
            client_dir: p.client_dir.clone(),
        })
        .load_settings()
        {
            s.options = prev.options;
            s.signoff = SignOff {
                udin: String::new(),
                date: String::new(),
                ..prev.signoff
            };
            s.tags = prev.tags;
            s.profit_sharing = prev.profit_sharing;
            s.depreciation_basis = prev.depreciation_basis;
        }
        p.save_settings(&s)?;
        p.log(
            "user",
            "project_created",
            serde_json::json!({"entity": name, "type": entity_type, "fy": fy}),
        )?;
        p.ensure_pins()?;
        Ok(id)
    }

    fn bin(&self) -> PathBuf {
        self.root.join("Recycle Bin")
    }

    /// Delete = move to `<data>/Recycle Bin/<client>/<fy>~<time>`; nothing is
    /// erased, so a deleted year can be restored. The client's remembered
    /// mapping stays in place for its other years.
    pub fn delete(&self, id: &str) -> Result<String, String> {
        let p = self.project(id)?;
        let (client, fy) = id.split_once('~').ok_or("bad project id")?;
        p.log(
            "user",
            "project_deleted",
            serde_json::json!({"moved_to": "Recycle Bin"}),
        )?;
        let to = self
            .bin()
            .join(client)
            .join(format!("{fy}~{}", Local::now().format("%Y%m%d-%H%M%S")));
        fs::create_dir_all(to.parent().unwrap()).map_err(|e| e.to_string())?;
        fs::rename(&p.dir, &to).map_err(|e| format!("could not move to the Recycle Bin: {e}"))?;
        Ok(to.display().to_string())
    }

    /// Items in the Recycle Bin. Id is "<client>~<fy>~<time>".
    pub fn deleted(&self) -> Vec<ProjectSummary> {
        let mut out = Vec::new();
        let Ok(clients) = fs::read_dir(self.bin()) else {
            return out;
        };
        for c in clients.flatten() {
            for y in fs::read_dir(c.path()).into_iter().flatten().flatten() {
                if let Some(s) = fs::read_to_string(y.path().join("settings.json"))
                    .ok()
                    .and_then(|t| serde_json::from_str::<Settings>(&t).ok())
                {
                    out.push(ProjectSummary {
                        id: format!(
                            "{}~{}",
                            c.file_name().to_string_lossy(),
                            y.file_name().to_string_lossy()
                        ),
                        entity_name: s.entity_name,
                        entity_type: s.entity_type,
                        fy: s.fy,
                        modified: s.modified,
                    });
                }
            }
        }
        out.sort_by(|a, b| b.id.cmp(&a.id));
        out
    }

    pub fn restore(&self, bin_id: &str) -> Result<String, String> {
        let mut it = bin_id.splitn(3, '~');
        let (Some(client), Some(fy), Some(stamp)) = (it.next(), it.next(), it.next()) else {
            return Err("bad item".into());
        };
        for part in [client, fy, stamp] {
            if part.is_empty() || part.contains("..") || part.contains('/') || part.contains('\\') {
                return Err("bad item".into());
            }
        }
        let from = self.bin().join(client).join(format!("{fy}~{stamp}"));
        if !from.join("settings.json").exists() {
            return Err("This item is no longer in the Recycle Bin.".into());
        }
        let client_dir = self.root.join("clients").join(client);
        let to = client_dir.join(fy);
        if to.exists() {
            return Err(format!(
                "FY {fy} of this client already exists. Delete or rename that one first."
            ));
        }
        fs::create_dir_all(&client_dir).map_err(|e| e.to_string())?;
        fs::rename(&from, &to).map_err(|e| e.to_string())?;
        let id = format!("{client}~{fy}");
        self.project(&id)?.log(
            "user",
            "project_restored",
            serde_json::json!({"from": "Recycle Bin"}),
        )?;
        Ok(id)
    }

    pub fn list(&self) -> Vec<ProjectSummary> {
        let mut out = Vec::new();
        let Ok(clients) = fs::read_dir(self.root.join("clients")) else {
            return out;
        };
        for c in clients.flatten() {
            let Ok(years) = fs::read_dir(c.path()) else {
                continue;
            };
            for y in years.flatten() {
                let sp = y.path().join("settings.json");
                if let Ok(s) = fs::read_to_string(&sp)
                    .map_err(|_| ())
                    .and_then(|t| serde_json::from_str::<Settings>(&t).map_err(|_| ()))
                {
                    out.push(ProjectSummary {
                        id: format!(
                            "{}~{}",
                            c.file_name().to_string_lossy(),
                            y.file_name().to_string_lossy()
                        ),
                        entity_name: s.entity_name,
                        entity_type: s.entity_type,
                        fy: s.fy,
                        modified: s.modified,
                    });
                }
            }
        }
        out.sort_by(|a, b| b.modified.cmp(&a.modified));
        out
    }
}

trait YearShift {
    fn with_year_safe(self, delta: i32) -> chrono::NaiveDate;
}
impl YearShift for chrono::NaiveDate {
    fn with_year_safe(self, delta: i32) -> chrono::NaiveDate {
        use chrono::Datelike;
        self.with_year(self.year() + delta).unwrap_or(self)
    }
}
