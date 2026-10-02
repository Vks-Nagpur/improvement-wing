//! `ledgercraft` command line.
//!
//!   ledgercraft analyse  --name "ABC & Co" --entity firm --fy 2025-26 --tb tb.xlsx
//!                        [--py-tb py.xlsx] [--vouchers daybook.csv] [--signoff signoff.json]
//!                        [--out folder] [--final] [--expert]
//!   ledgercraft practice-data --out folder      (writes sample books with known answers)
//!   ledgercraft bench --vouchers 1000000        (speed test on a large clean book)

use lc_core::rules::{RulesPack, Severity};
use lc_core::{analyse, EntityType};
use lc_io::export::{export, ExportOptions, Mode, SignOff};
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Instant;

fn args() -> (String, HashMap<String, String>) {
    let mut it = std::env::args().skip(1);
    let cmd = it.next().unwrap_or_default();
    let mut map = HashMap::new();
    let rest: Vec<String> = it.collect();
    let mut i = 0;
    while i < rest.len() {
        if let Some(k) = rest[i].strip_prefix("--") {
            let v = rest.get(i + 1).filter(|v| !v.starts_with("--")).cloned();
            if v.is_some() {
                i += 1;
            }
            map.insert(k.to_string(), v.unwrap_or_else(|| "true".into()));
        }
        i += 1;
    }
    (cmd, map)
}

fn main() -> ExitCode {
    let (cmd, a) = args();
    let r = match cmd.as_str() {
        "analyse" | "analyze" => cmd_analyse(&a),
        "practice-data" => cmd_practice(&a),
        "bench" => cmd_bench(&a),
        _ => {
            eprintln!("{}", include_str!("help.txt"));
            return ExitCode::from(2);
        }
    };
    match r {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("Error: {e}");
            ExitCode::FAILURE
        }
    }
}

fn need<'a>(a: &'a HashMap<String, String>, k: &str) -> Result<&'a str, String> {
    a.get(k)
        .map(|s| s.as_str())
        .ok_or_else(|| format!("missing --{k}"))
}

fn cmd_analyse(a: &HashMap<String, String>) -> Result<(), String> {
    let entity = EntityType::parse(need(a, "entity")?)
        .ok_or("--entity must be one of: company, llp, firm, proprietor, huf, aop, boi")?;
    let (fy_start, fy_end) =
        lc_core::date::parse_fy(need(a, "fy")?).ok_or("--fy must look like 2025-26")?;
    let t0 = Instant::now();
    let cy = lc_io::read::read_trial_balance(&PathBuf::from(need(a, "tb")?))?;
    let py = a
        .get("py-tb")
        .map(|p| lc_io::read::read_trial_balance(&PathBuf::from(p)))
        .transpose()?;
    let vouchers = a
        .get("vouchers")
        .map(|p| lc_io::read::read_vouchers(&PathBuf::from(p)))
        .transpose()?
        .unwrap_or_default();
    let mapping_memory = match a.get("mapping") {
        Some(p) => serde_json::from_str(&std::fs::read_to_string(p).map_err(|e| e.to_string())?)
            .map_err(|e| format!("mapping file: {e}"))?,
        None => HashMap::new(),
    };
    let eng = lc_core::Engagement {
        entity_name: a.get("name").cloned().unwrap_or_else(|| "Entity".into()),
        entity_type: entity,
        fy_start,
        fy_end,
        cy,
        py,
        vouchers,
        mapping_memory,
    };
    let t_read = t0.elapsed();
    let rules = match a.get("rules") {
        Some(p) => RulesPack::from_json(&std::fs::read_to_string(p).map_err(|e| e.to_string())?)?,
        None => RulesPack::builtin(),
    };
    let t1 = Instant::now();
    let res = analyse(&eng, &rules);
    let t_an = t1.elapsed();
    print_report(&eng, &res, a.contains_key("expert"));
    let signoff: SignOff = match a.get("signoff") {
        Some(p) => serde_json::from_str(&std::fs::read_to_string(p).map_err(|e| e.to_string())?)
            .map_err(|e| format!("signoff file: {e}"))?,
        None => SignOff::default(),
    };
    let out = PathBuf::from(
        a.get("out")
            .cloned()
            .unwrap_or_else(|| "LedgerCraft Output".into()),
    );
    let mode = if a.contains_key("final") {
        Mode::Signing
    } else {
        Mode::Draft
    };
    let dir = export(
        &out,
        &eng,
        &res,
        &signoff,
        &ExportOptions {
            mode,
            ..Default::default()
        },
    )?;
    println!(
        "\nRead {} vouchers in {:.2?}; checked in {:.2?}.",
        eng.vouchers.len(),
        t_read,
        t_an
    );
    println!("Files saved in: {}", dir.display());
    Ok(())
}

fn print_report(eng: &lc_core::Engagement, res: &lc_core::Analysis, expert: bool) {
    println!(
        "LedgerCraft – {} ({}) – FY {}",
        eng.entity_name,
        eng.entity_type.label(),
        lc_core::date::fy_label(eng.fy_start)
    );
    println!(
        "Must fix: {}   Check: {}   Notes: {}",
        res.count(Severity::Blocker),
        res.count(Severity::Warning),
        res.count(Severity::Info)
    );
    for f in &res.findings {
        println!("\n[{}] {}", f.severity.label(), f.title);
        println!("  {}", f.message);
        if let Some(s) = &f.suggestion {
            println!("  What to do: {s}");
        }
        if expert && !f.legal_ref.is_empty() {
            println!("  Reference: {}", f.legal_ref);
        }
    }
    let p = res.statements.profit;
    println!("\nProfit / (Loss) for the year: {}", p.0);
    println!("Balance Sheet total: {}", res.statements.total_assets.0);
    println!(
        "{}",
        if res.printable {
            "Ready for a signing copy."
        } else {
            "Not ready for signing: fix the 'Must fix' items first (draft export still produced)."
        }
    );
}

fn cmd_practice(a: &HashMap<String, String>) -> Result<(), String> {
    let out = PathBuf::from(
        a.get("out")
            .cloned()
            .unwrap_or_else(|| "practice-data".into()),
    );
    for s in lc_testdata::scenarios::all() {
        let dir = out.join(&s.name);
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        lc_io::write_inputs::write_trial_balance(
            &s.engagement.cy,
            &dir.join("trial_balance.xlsx"),
        )?;
        if let Some(py) = &s.engagement.py {
            lc_io::write_inputs::write_trial_balance(
                py,
                &dir.join("previous_year_trial_balance.xlsx"),
            )?;
        }
        lc_io::write_inputs::write_vouchers_csv(&s.engagement.vouchers, &dir.join("vouchers.csv"))?;
        let mut exp: Vec<&String> = s.expected.keys.iter().collect();
        exp.sort();
        let info = serde_json::json!({
            "scenario": s.name,
            "description": s.description,
            "entity_name": s.engagement.entity_name,
            "entity": format!("{:?}", s.engagement.entity_type).to_lowercase(),
            "fy": lc_core::date::fy_label(s.engagement.fy_start),
            "expected_findings": exp,
            "expected_profit": s.expected.profit_cy.fmt_indian(),
        });
        std::fs::write(
            dir.join("expected.json"),
            serde_json::to_string_pretty(&info).unwrap(),
        )
        .map_err(|e| e.to_string())?;
        println!(
            "{:<24} {:>6} vouchers  {:>2} planted issues  -> {}",
            s.name,
            s.engagement.vouchers.len(),
            s.expected.keys.len(),
            dir.display()
        );
    }
    Ok(())
}

fn cmd_bench(a: &HashMap<String, String>) -> Result<(), String> {
    let n: usize = a
        .get("vouchers")
        .map(|s| s.parse().map_err(|_| "bad --vouchers"))
        .transpose()?
        .unwrap_or(1_000_000);
    let per_day = (n / 365).max(1);
    let t0 = Instant::now();
    let s =
        lc_testdata::scenarios::clean(lc_testdata::scenarios::Kind::Firm, 42, 300, 150, per_day);
    let lines: usize = s.engagement.vouchers.iter().map(|v| v.lines.len()).sum();
    println!(
        "Generated {} vouchers ({} lines) in {:.2?}",
        s.engagement.vouchers.len(),
        lines,
        t0.elapsed()
    );
    let t1 = Instant::now();
    let res = analyse(&s.engagement, &RulesPack::builtin());
    println!(
        "Checked in {:.2?}: {} findings (expected {}), balance sheet tallies: {}",
        t1.elapsed(),
        res.findings.len(),
        s.expected.keys.len(),
        res.statements.total_assets == res.statements.total_liabilities
    );
    if let Some(p) = a.get("write") {
        let t2 = Instant::now();
        lc_io::write_inputs::write_vouchers_csv(&s.engagement.vouchers, &PathBuf::from(p))?;
        println!("Wrote CSV in {:.2?}; reading back...", t2.elapsed());
        let t3 = Instant::now();
        let back = lc_io::read::read_vouchers(&PathBuf::from(p))?;
        println!(
            "Read {} vouchers back in {:.2?}; identical: {}",
            back.len(),
            t3.elapsed(),
            back == s.engagement.vouchers
        );
    }
    Ok(())
}
