fn main() {
    for s in lc_testdata::scenarios::all() {
        let a = lc_core::analyse(&s.engagement, &lc_core::rules::RulesPack::builtin());
        println!(
            "{:<28} vouchers={:>6} ledgers={:>3} expected={:>2} found={:>2} profit={}",
            s.name,
            s.engagement.vouchers.len(),
            s.engagement.cy.ledgers.len(),
            s.expected.keys.len(),
            a.findings.len(),
            a.statements.profit.0
        );
        if s.name == "firm_with_glitches" {
            for f in &a.findings {
                println!("   [{:?}] {}", f.severity, f.message);
            }
        }
    }
}
