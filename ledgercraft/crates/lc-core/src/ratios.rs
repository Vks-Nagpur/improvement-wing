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

fn div(a: f64, b: f64) -> Option<f64> {
    if b.abs() < 0.005 {
        None
    } else {
        Some(a / b)
    }
}

/// `principal_repaid`: loans repaid during the current year (from the loan register).
pub fn compute(cy: &YearFacts, py: Option<&YearFacts>, principal_repaid: Money) -> Vec<Ratio> {
    let c = Y(cy);
    let p = py.map(Y);
    let avg = |cur: f64, prev: Option<f64>| prev.map(|x| (cur + x) / 2.0).unwrap_or(cur);
    let mut out = Vec::new();
    let mut push =
        |name: &str, num: &str, den: &str, unit: &str, cyv: Option<f64>, pyv: Option<f64>| {
            let pct = unit == "%";
            let cyv = cyv.map(|v| if pct { v * 100.0 } else { v });
            let pyv = pyv.map(|v| if pct { v * 100.0 } else { v });
            let variance = match (cyv, pyv) {
                (Some(a), Some(b)) if b.abs() > 1e-9 => Some((a - b) / b.abs() * 100.0),
                _ => None,
            };
            out.push(Ratio {
                name: name.into(),
                numerator: num.into(),
                denominator: den.into(),
                unit: unit.into(),
                cy: cyv,
                py: pyv,
                variance_pct: variance,
                needs_explanation: variance.map(|v| v.abs() > 25.0).unwrap_or(false),
            });
        };
    let pf = |g: &dyn Fn(&Y) -> Option<f64>| p.as_ref().and_then(g);

    push(
        "Current ratio",
        "Current assets",
        "Current liabilities",
        "times",
        div(c.current_assets(), c.current_liabilities()),
        pf(&|y| div(y.current_assets(), y.current_liabilities())),
    );
    push(
        "Debt-equity ratio",
        "Total borrowings",
        "Owners' funds / shareholders' equity",
        "times",
        div(c.debt(), c.equity()),
        pf(&|y| div(y.debt(), y.equity())),
    );
    let fin = c.h(Head::FinanceCosts);
    let ebds = c.pat() + c.h(Head::Depreciation) + fin;
    push(
        "Debt service coverage ratio",
        "Profit after tax + depreciation + finance costs",
        "Finance costs + principal repaid",
        "times",
        div(ebds, fin + f(principal_repaid)),
        None,
    );
    let prev = |g: fn(&Y) -> f64| p.as_ref().map(g);
    push(
        "Return on equity",
        "Profit after tax",
        "Average owners' funds",
        "%",
        div(c.pat(), avg(c.equity(), prev(|y| y.equity()))),
        pf(&|y| div(y.pat(), y.equity())),
    );
    push(
        "Inventory turnover ratio",
        "Cost of goods sold",
        "Average inventory",
        "times",
        div(
            c.cogs(),
            avg(c.h(Head::Inventories), prev(|y| y.h(Head::Inventories))),
        ),
        pf(&|y| div(y.cogs(), y.h(Head::Inventories))),
    );
    push(
        "Trade receivables turnover ratio",
        "Revenue from operations",
        "Average trade receivables",
        "times",
        div(
            c.revenue(),
            avg(
                c.h(Head::TradeReceivables),
                prev(|y| y.h(Head::TradeReceivables)),
            ),
        ),
        pf(&|y| div(y.revenue(), y.h(Head::TradeReceivables))),
    );
    push(
        "Trade payables turnover ratio",
        "Purchases",
        "Average trade payables",
        "times",
        div(
            c.purchases(),
            avg(c.h(Head::TradePayables), prev(|y| y.h(Head::TradePayables))),
        ),
        pf(&|y| div(y.purchases(), y.h(Head::TradePayables))),
    );
    push(
        "Net capital turnover ratio",
        "Revenue from operations",
        "Working capital (current assets - current liabilities)",
        "times",
        div(c.revenue(), c.current_assets() - c.current_liabilities()),
        pf(&|y| div(y.revenue(), y.current_assets() - y.current_liabilities())),
    );
    push(
        "Net profit ratio",
        "Profit after tax",
        "Revenue from operations",
        "%",
        div(c.pat(), c.revenue()),
        pf(&|y| div(y.pat(), y.revenue())),
    );
    let ebit = |y: &Y| f(y.0.pbt()) + y.h(Head::FinanceCosts);
    push(
        "Return on capital employed",
        "Profit before interest and tax",
        "Capital employed (owners' funds + borrowings)",
        "%",
        div(ebit(&c), c.equity() + c.debt()),
        pf(&|y| div(ebit(y), y.equity() + y.debt())),
    );
    push(
        "Return on investment",
        "Income from investments",
        "Average investments",
        "%",
        div(
            c.investment_income(),
            avg(c.investments(), prev(|y| y.investments())),
        ),
        pf(&|y| div(y.investment_income(), y.investments())),
    );
    out
}
