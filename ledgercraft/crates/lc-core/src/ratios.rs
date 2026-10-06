//! The 11 analytical ratios required by Schedule III (2021 amendment).
//! Computed on exact (unrounded) figures. Averages use opening and closing
//! balances where last year's balance sheet is available.

use crate::facts::YearFacts;
use crate::mapping::Head;
use crate::money::Money;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Ratio {
    pub name: String,
    pub numerator: String,
    pub denominator: String,
    pub unit: String,
    pub cy: Option<f64>,
    pub py: Option<f64>,
    pub variance_pct: Option<f64>,
    /// Change of more than 25% must be explained.
    pub needs_explanation: bool,
    /// Figures behind the ratio (rupees), for drill-down.
    #[serde(default)]
    pub num_cy: Option<f64>,
    #[serde(default)]
    pub den_cy: Option<f64>,
    #[serde(default)]
    pub num_py: Option<f64>,
    #[serde(default)]
    pub den_py: Option<f64>,
    /// Points about the figures: inferred components, basis differences,
    /// ratios that are not meaningful.
    #[serde(default)]
    pub notes: Vec<String>,
    /// A person should look at the components before relying on the ratio.
    #[serde(default)]
    pub review: bool,
    /// The formula has not been verified against the official text.
    #[serde(default)]
    pub formula_verified: bool,
}

fn f(m: Money) -> f64 {
    m.as_f64()
}

struct Y<'a>(&'a YearFacts);

impl Y<'_> {
    fn h(&self, h: Head) -> f64 {
        f(self.0.head(h))
    }
    fn current_assets(&self) -> f64 {
        [
            Head::CurrentInvestments,
            Head::Inventories,
            Head::TradeReceivables,
            Head::CashBank,
            Head::StLoansAdvances,
            Head::OtherCurrentAssets,
        ]
        .iter()
        .map(|x| self.h(*x))
        .sum()
    }
    fn current_liabilities(&self) -> f64 {
        [
            Head::StBorrowings,
            Head::TradePayables,
            Head::OtherCurrentLiabilities,
            Head::StProvisions,
        ]
        .iter()
        .map(|x| self.h(*x))
        .sum()
    }
    fn debt(&self) -> f64 {
        self.h(Head::LtBorrowings) + self.h(Head::StBorrowings)
    }
    fn equity(&self) -> f64 {
        self.h(Head::Capital) + self.h(Head::ReservesSurplus)
    }
    fn pat(&self) -> f64 {
        f(self.0.profit())
    }
    fn revenue(&self) -> f64 {
        self.h(Head::RevenueOps)
    }
    fn purchases(&self) -> f64 {
        self.h(Head::Purchases)
    }
    fn cogs(&self) -> f64 {
        self.h(Head::Purchases) + f(self.0.change_in_inventories)
    }
    fn investments(&self) -> f64 {
        self.h(Head::NcInvestments) + self.h(Head::CurrentInvestments)
    }
    fn investment_income(&self) -> f64 {
        self.0
            .lines
            .get(&Head::OtherIncome)
            .map(|v| {
                v.iter()
                    .filter(|l| {
                        let n = l.name.to_ascii_lowercase();
                        n.contains("dividend")
                            || n.contains("mutual fund")
                            || n.contains("investment")
                            || n.contains("fixed deposit")
                            || n.contains(" fd")
                    })
                    .map(|l| f(l.amount))
                    .sum()
            })
            .unwrap_or(0.0)
    }
}

/// Ratio of two figures. None when the denominator is nil, or negative
/// (for example negative equity or working capital), where it is not meaningful.
fn ratio(num: f64, den: f64) -> Option<f64> {
    if den.abs() < 0.005 || den < 0.0 {
        None
    } else {
        Some(num / den)
    }
}

struct Spec<'a> {
    name: &'a str,
    num: &'a str,
    den: &'a str,
    unit: &'a str,
    cy: (f64, f64),
    py: Option<(f64, f64)>,
    notes: Vec<String>,
}

fn finish(s: Spec, average_cy: bool) -> Ratio {
    let pct = s.unit == "%";
    let scale = |v: Option<f64>| v.map(|v| if pct { v * 100.0 } else { v });
    let cyv = scale(ratio(s.cy.0, s.cy.1));
    let pyv = scale(s.py.and_then(|(n, d)| ratio(n, d)));
    let mut notes = s.notes;
    if s.cy.1 < 0.0 {
        notes.push("Denominator is negative this year: ratio not meaningful.".into());
    }
    if let Some((_, d)) = s.py {
        if d < 0.0 {
            notes.push("Denominator is negative last year: ratio not meaningful.".into());
        }
    }
    if average_cy && s.py.is_some() {
        notes.push("This year uses average balances; last year uses closing balances (its opening balances are not available), so the change is not strictly like for like.".into());
    }
    let variance = match (cyv, pyv) {
        (Some(a), Some(b)) if b.abs() > 1e-9 => Some((a - b) / b.abs() * 100.0),
        _ => None,
    };
    let review = !notes.is_empty();
    Ratio {
        name: s.name.into(),
        numerator: s.num.into(),
        denominator: s.den.into(),
        unit: s.unit.into(),
        cy: cyv,
        py: pyv,
        variance_pct: variance,
        needs_explanation: variance.map(|v| v.abs() > 25.0).unwrap_or(false),
        num_cy: Some(s.cy.0),
        den_cy: Some(s.cy.1),
        num_py: s.py.map(|x| x.0),
        den_py: s.py.map(|x| x.1),
        notes,
        review,
        formula_verified: false,
    }
}

/// `principal_repaid`: loans repaid during the current year (from the loan register).
pub fn compute(cy: &YearFacts, py: Option<&YearFacts>, principal_repaid: Money) -> Vec<Ratio> {
    let c = Y(cy);
    let p = py.map(Y);
    let avg = |cur: f64, prev: Option<f64>| prev.map(|x| (cur + x) / 2.0).unwrap_or(cur);
    let prev = |g: fn(&Y) -> f64| p.as_ref().map(g);
    let pyc = |g: &dyn Fn(&Y) -> (f64, f64)| p.as_ref().map(g);
    let cogs_note = || {
        "Cost of goods sold taken as purchases plus change in inventories; direct expenses and materials consumed are not included.".to_string()
    };
    let mut out = Vec::new();

    out.push(finish(
        Spec {
            name: "Current ratio",
            num: "Current assets",
            den: "Current liabilities",
            unit: "times",
            cy: (c.current_assets(), c.current_liabilities()),
            py: pyc(&|y| (y.current_assets(), y.current_liabilities())),
            notes: vec![],
        },
        false,
    ));
    out.push(finish(
        Spec {
            name: "Debt-equity ratio",
            num: "Total borrowings",
            den: "Owners' funds / shareholders' equity",
            unit: "times",
            cy: (c.debt(), c.equity()),
            py: pyc(&|y| (y.debt(), y.equity())),
            notes: vec![],
        },
        false,
    ));
    let fin = c.h(Head::FinanceCosts);
    out.push(finish(Spec { name: "Debt service coverage ratio", num: "Profit after tax + depreciation + finance costs", den: "Finance costs + principal repaid", unit: "times",
        cy: (c.pat() + c.h(Head::Depreciation) + fin, fin + f(principal_repaid)), py: None,
        notes: vec!["Principal repaid comes from the loan register (day book); last year is not computed.".into()] }, false));
    out.push(finish(
        Spec {
            name: "Return on equity",
            num: "Profit after tax",
            den: "Average owners' funds",
            unit: "%",
            cy: (c.pat(), avg(c.equity(), prev(|y| y.equity()))),
            py: pyc(&|y| (y.pat(), y.equity())),
            notes: vec![],
        },
        true,
    ));
    out.push(finish(
        Spec {
            name: "Inventory turnover ratio",
            num: "Cost of goods sold",
            den: "Average inventory",
            unit: "times",
            cy: (
                c.cogs(),
                avg(c.h(Head::Inventories), prev(|y| y.h(Head::Inventories))),
            ),
            py: pyc(&|y| (y.cogs(), y.h(Head::Inventories))),
            notes: vec![cogs_note()],
        },
        true,
    ));
    out.push(finish(
        Spec {
            name: "Trade receivables turnover ratio",
            num: "Revenue from operations",
            den: "Average trade receivables",
            unit: "times",
            cy: (
                c.revenue(),
                avg(
                    c.h(Head::TradeReceivables),
                    prev(|y| y.h(Head::TradeReceivables)),
                ),
            ),
            py: pyc(&|y| (y.revenue(), y.h(Head::TradeReceivables))),
            notes: vec![],
        },
        true,
    ));
    out.push(finish(Spec { name: "Trade payables turnover ratio", num: "Purchases", den: "Average trade payables", unit: "times",
        cy: (c.purchases(), avg(c.h(Head::TradePayables), prev(|y| y.h(Head::TradePayables)))), py: pyc(&|y| (y.purchases(), y.h(Head::TradePayables))),
        notes: vec!["Purchases are purchases of stock-in-trade only; purchases of services and other expenses on credit are not included.".into()] }, true));
    out.push(finish(
        Spec {
            name: "Net capital turnover ratio",
            num: "Revenue from operations",
            den: "Working capital (current assets - current liabilities)",
            unit: "times",
            cy: (c.revenue(), c.current_assets() - c.current_liabilities()),
            py: pyc(&|y| (y.revenue(), y.current_assets() - y.current_liabilities())),
            notes: vec![],
        },
        false,
    ));
    out.push(finish(
        Spec {
            name: "Net profit ratio",
            num: "Profit after tax",
            den: "Revenue from operations",
            unit: "%",
            cy: (c.pat(), c.revenue()),
            py: pyc(&|y| (y.pat(), y.revenue())),
            notes: vec![],
        },
        false,
    ));
    let ebit = |y: &Y| f(y.0.pbt()) + y.h(Head::FinanceCosts);
    out.push(finish(
        Spec {
            name: "Return on capital employed",
            num: "Profit before interest and tax",
            den: "Capital employed (owners' funds + borrowings)",
            unit: "%",
            cy: (ebit(&c), c.equity() + c.debt()),
            py: pyc(&|y| (ebit(y), y.equity() + y.debt())),
            notes: vec![],
        },
        false,
    ));
    let inv_note = if c.investment_income() != 0.0 || c.investments() != 0.0 {
        vec!["Income from investments is identified by ledger names (dividend, mutual fund, investment, fixed deposit); check the ledgers included.".to_string()]
    } else {
        vec![]
    };
    out.push(finish(
        Spec {
            name: "Return on investment",
            num: "Income from investments",
            den: "Average investments",
            unit: "%",
            cy: (
                c.investment_income(),
                avg(c.investments(), prev(|y| y.investments())),
            ),
            py: pyc(&|y| (y.investment_income(), y.investments())),
            notes: inv_note,
        },
        true,
    ));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nil_and_negative_denominators_give_no_ratio() {
        assert_eq!(ratio(10.0, 0.0), None);
        assert_eq!(ratio(10.0, -5.0), None);
        assert_eq!(ratio(-10.0, 5.0), Some(-2.0));
        let r = finish(
            Spec {
                name: "x",
                num: "n",
                den: "d",
                unit: "times",
                cy: (10.0, -5.0),
                py: Some((10.0, 5.0)),
                notes: vec![],
            },
            false,
        );
        assert!(r.cy.is_none() && r.review && r.notes[0].contains("not meaningful"));
        assert!(!r.needs_explanation, "no comparison without both years");
    }

    #[test]
    fn change_over_25_percent_is_flagged_and_signs_are_kept() {
        let r = finish(
            Spec {
                name: "x",
                num: "n",
                den: "d",
                unit: "%",
                cy: (13.0, 100.0),
                py: Some((10.0, 100.0)),
                notes: vec![],
            },
            false,
        );
        assert!(r.needs_explanation && (r.variance_pct.unwrap() - 30.0).abs() < 1e-9);
        let r = finish(
            Spec {
                name: "x",
                num: "n",
                den: "d",
                unit: "%",
                cy: (12.0, 100.0),
                py: Some((10.0, 100.0)),
                notes: vec![],
            },
            false,
        );
        assert!(!r.needs_explanation);
        // Loss to smaller loss: improvement shown as a positive change.
        let r = finish(
            Spec {
                name: "x",
                num: "n",
                den: "d",
                unit: "%",
                cy: (-5.0, 100.0),
                py: Some((-10.0, 100.0)),
                notes: vec![],
            },
            false,
        );
        assert!(r.variance_pct.unwrap() > 0.0);
    }

    #[test]
    fn average_basis_difference_is_stated() {
        let r = finish(
            Spec {
                name: "x",
                num: "n",
                den: "d",
                unit: "times",
                cy: (1.0, 1.0),
                py: Some((1.0, 1.0)),
                notes: vec![],
            },
            true,
        );
        assert!(r.review && r.notes.iter().any(|n| n.contains("closing balances")));
        assert!(!r.formula_verified);
    }
}
