//! Golden book corpus (J01): one clean book per supported entity type. Each
//! book's statements, notes, findings, tax audit helper and legal readiness
//! are written as plain text and must match the saved copy in
//! `tests/golden/` exactly. A change shows up as a readable diff.
//!
//! After an intended change, regenerate with `UPDATE_GOLDEN=1 cargo test
//! --test golden` and review the diff before committing it.
//!
//! The saved copies are machine-generated from synthetic books. They are not
//! yet reviewed by a professional; see `tests/golden/README.md`.

use lc_core::report::Block;
use lc_core::rules::RulesPack;
use lc_core::{analyse, Engagement};
use lc_io::export::{ExportOptions, SignOff};
use std::fmt::Write;
use std::path::PathBuf;

fn snapshot(name: &str, description: &str, eng: &Engagement) -> String {
    let rules = RulesPack::builtin();
    let first = analyse(eng, &rules);
    let eng = lc_testdata::confirm_all(eng, &first);
    let a = analyse(&eng, &rules);
    let mut s = String::new();
    let w = &mut s;
    writeln!(w, "# {name}").unwrap();
    writeln!(w, "{description}").unwrap();
    writeln!(
        w,
        "entity: {:?} | year: {} to {}",
        eng.entity_type, eng.fy_start, eng.fy_end
    )
    .unwrap();
    writeln!(w, "\n## Findings (after every placement is confirmed)").unwrap();
    let mut f: Vec<String> = a
        .findings
        .iter()
        .map(|f| format!("{:?} {}", f.severity, f.key))
        .collect();
    f.sort();
    for x in f {
        writeln!(w, "{x}").unwrap();
    }

    writeln!(w, "\n## Mapping (ledger | group | head | closing | status)").unwrap();
    for m in &a.mapping {
        writeln!(
            w,
            "{} | {} | {} | {} | {:?}{}",
            m.name,
            m.group,
            m.head
                .map(|h| format!("{h:?}"))
                .unwrap_or_else(|| "-".into()),
            m.tb_closing,
            m.status,
            if m.reclassified {
                " (reclassified)"
            } else {
                ""
            }
        )
        .unwrap();
    }

    let rep = lc_io::export::report(&eng, &a, &SignOff::default(), &ExportOptions::default());
    writeln!(w, "\n## Blockers").unwrap();
    for b in &rep.blockers {
        writeln!(w, "{b}").unwrap();
    }
    writeln!(w, "\n## Warnings").unwrap();
    for b in &rep.warnings {
        writeln!(w, "{b}").unwrap();
    }
    for sec in &rep.sections {
        writeln!(w, "\n## {} [{}]", sec.title, sec.id).unwrap();
        for b in &sec.blocks {
            match b {
                Block::Heading { text, .. } => writeln!(w, "### {text}").unwrap(),
                Block::Para { text } => writeln!(w, "{text}").unwrap(),
                Block::Table(t) => {
                    for h in &t.header {
                        let cells: Vec<String> = h.iter().map(|c| c.text.clone()).collect();
                        writeln!(w, "  [{}]", cells.join(" | ")).unwrap();
                    }
                    for r in &t.rows {
                        writeln!(w, "  {}", r.cells.join(" | ")).unwrap();
                    }
                }
                _ => writeln!(w, "[chart]").unwrap(),
            }
        }
        if let Some(n) = &sec.closing_note {
            writeln!(w, "{n}").unwrap();
        }
    }

    // Tax audit helper, as written to Excel and read back.
    static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let n = N.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!("lc-golden-{}-{name}-{n}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let p = dir.join("t.xlsx");
    lc_io::tax_audit::write(&p, &eng, &a).unwrap();
    writeln!(
        w,
        "\n## Tax audit helper ({})",
        lc_io::tax_audit::file_name(&eng)
    )
    .unwrap();
    let mut book: calamine::Xlsx<_> = calamine::open_workbook(&p).unwrap();
    for sheet in calamine::Reader::sheet_names(&book) {
        writeln!(w, "### {sheet}").unwrap();
        let r = calamine::Reader::worksheet_range(&mut book, &sheet).unwrap();
        for row in r.rows() {
            let cells: Vec<String> = row.iter().map(|c| c.to_string()).collect();
            if cells.iter().any(|c| !c.is_empty()) {
                writeln!(w, "  {}", cells.join(" | ")).unwrap();
            }
        }
    }
    let _ = std::fs::remove_dir_all(&dir);

    // Legal content this book relies on, by status as shipped.
    let items = lc_core::legal::applicable(
        &eng,
        &rules,
        &lc_core::statements::FormatPack::for_entity(eng.entity_type),
        &lc_core::far::DepPack::builtin(),
        lc_core::legal::Scope {
            tax_audit: true,
            depreciation: eng.far.is_some(),
        },
    );
    writeln!(
        w,
        "\n## Legal items ({} applicable, none verified)",
        items.len()
    )
    .unwrap();
    for it in &items {
        writeln!(
            w,
            "{} [{}]",
            it.id,
            it.shipped_status.split(' ').next().unwrap_or("")
        )
        .unwrap();
    }
    s
}

#[test]
fn golden_books_for_every_entity_type() {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/golden");
    let update = std::env::var("UPDATE_GOLDEN").is_ok();
    let books = lc_testdata::scenarios::golden();
    let types: std::collections::BTreeSet<String> = books
        .iter()
        .map(|b| format!("{:?}", b.engagement.entity_type))
        .collect();
    assert_eq!(types.len(), 7, "one golden book per entity type: {types:?}");
    let mut changed = Vec::new();
    for b in &books {
        let got = snapshot(&b.name, &b.description, &b.engagement);
        let path = dir.join(format!("{}.txt", b.name));
        if update {
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(&path, &got).unwrap();
            continue;
        }
        let want = std::fs::read_to_string(&path)
            .unwrap_or_else(|_| panic!("{} missing: run with UPDATE_GOLDEN=1", path.display()))
            .replace("\r\n", "\n");
        if want != got {
            let line = want
                .lines()
                .zip(got.lines())
                .position(|(a, b)| a != b)
                .unwrap_or(want.lines().count().min(got.lines().count()));
            std::fs::write(path.with_extension("actual.txt"), &got).unwrap();
            changed.push(format!(
                "{}: first difference at line {} (actual written next to it)",
                b.name,
                line + 1
            ));
        }
    }
    assert!(
        changed.is_empty(),
        "golden books changed:\n{}",
        changed.join("\n")
    );
}

#[test]
fn golden_snapshot_is_deterministic() {
    let b = &lc_testdata::scenarios::golden()[0];
    assert_eq!(
        snapshot(&b.name, &b.description, &b.engagement),
        snapshot(&b.name, &b.description, &b.engagement)
    );
}
