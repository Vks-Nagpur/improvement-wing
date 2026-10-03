use lc_core::report::{Layout, ReportOptions, SignOff, Toggle};
use lc_core::units::Unit;
fn main() {
    let out = std::path::PathBuf::from(std::env::args().nth(1).unwrap_or("out/render".into()));
    std::fs::create_dir_all(&out).unwrap();
    let cases = [
        (
            "company",
            lc_testdata::scenarios::clean(lc_testdata::scenarios::Kind::Company, 2, 25, 15, 12),
            Unit::Lakhs,
            2,
            Layout::Boxed,
        ),
        (
            "firm",
            lc_testdata::scenarios::firm_with_glitches(),
            Unit::Rupees,
            2,
            Layout::Ruled,
        ),
        (
            "cleanfirm",
            lc_testdata::scenarios::clean(lc_testdata::scenarios::Kind::Firm, 1, 10, 6, 4),
            Unit::Thousands,
            0,
            Layout::Boxed,
        ),
    ];
    for (name, s, unit, dec, layout) in cases {
        let a = lc_core::analyse(&s.engagement, &lc_core::rules::RulesPack::builtin());
        let opt = ReportOptions {
            unit,
            decimals: dec,
            layout,
            cash_flow: Toggle::On,
            ratios: Toggle::On,
            entity_details: vec![
                "12, Civil Lines, Nagpur 440001".into(),
                "PAN: AAAFC1234D".into(),
            ],
            ..Default::default()
        };
        let r = lc_core::report::build(&s.engagement, &a, &opt, &SignOff::default());
        let t = std::time::Instant::now();
        let pdf = lc_io::render::pdf::render(&r).unwrap_or_else(|e| panic!("{e}"));
        println!(
            "{name}: pdf {} bytes in {:?}; warnings: {:?}",
            pdf.len(),
            t.elapsed(),
            r.warnings
        );
        std::fs::write(out.join(format!("{name}.pdf")), pdf).unwrap();
        std::fs::write(
            out.join(format!("{name}.xlsx")),
            lc_io::render::xlsx::render(&r).unwrap(),
        )
        .unwrap();
        std::fs::write(
            out.join(format!("{name}.html")),
            lc_io::render::html::render(&r),
        )
        .unwrap();
    }
}
