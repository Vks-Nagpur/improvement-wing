//! Fixed asset register and depreciation.
//!
//! * Books – Companies Act, Schedule II: useful life, residual value (5%),
//!   SLM or WDV, pro-rata by days for additions and disposals; or, as many
//!   non-corporate entities do, Income-tax rates in the books.
//! * Income-tax – block of assets, WDV, half rate for assets put to use for
//!   less than 180 days in the year, sale proceeds reduce the block.
//!
//! Depreciation is an estimate: rate arithmetic uses f64 and each result is
//! rounded to the paisa once, then all further arithmetic is exact.

use crate::money::Money;
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum BookBasis {
    /// Companies Act Schedule II – straight line.
    ScheduleIiSlm,
    /// Companies Act Schedule II – written down value.
    #[default]
    ScheduleIiWdv,
    /// Income-tax block rates applied in the books (180-day half-rate rule).
    IncomeTaxRates,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Asset {
    pub name: String,
    /// Trial-balance ledger the asset belongs to.
    pub ledger: String,
    #[serde(default)]
    pub book_class: String,
    pub it_block: String,
    pub put_to_use: NaiveDate,
    pub cost: Money,
    /// Books accumulated depreciation at the start of the year.
    #[serde(default)]
    pub opening_acc_dep: Money,
    #[serde(default)]
    pub sold_on: Option<NaiveDate>,
    #[serde(default)]
    pub sale_value: Money,
    #[serde(default)]
    pub useful_life_years: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct Register {
    pub assets: Vec<Asset>,
    /// Income-tax opening WDV by block key.
    #[serde(default)]
    pub it_opening: BTreeMap<String, Money>,
    #[serde(default)]
    pub basis: BookBasis,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BookClass {
    pub key: String,
    pub label: String,
    pub life: f64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ItBlock {
    pub key: String,
    pub label: String,
    pub rate: f64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DepPack {
    pub version: String,
    pub status: String,
    pub residual_pct: f64,
    pub book_classes: Vec<BookClass>,
    pub it_blocks: Vec<ItBlock>,
}

impl DepPack {
    pub fn builtin() -> DepPack {
        serde_json::from_str(include_str!("../packs/depreciation.json"))
            .expect("depreciation pack is valid")
    }
    pub fn class(&self, key: &str) -> Option<&BookClass> {
        self.book_classes.iter().find(|c| c.key == key)
    }
    pub fn block(&self, key: &str) -> Option<&ItBlock> {
        self.it_blocks.iter().find(|c| c.key == key)
    }
}

/// One line of the PPE schedule (per trial-balance ledger).
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct PpeRow {
    pub ledger: String,
    pub gross_opening: Money,
    pub additions: Money,
    pub deletions: Money,
    pub gross_closing: Money,
    pub dep_opening: Money,
    pub dep_for_year: Money,
    pub dep_on_deletions: Money,
    pub dep_closing: Money,
    pub net_closing: Money,
    pub net_opening: Money,
    pub gain_on_sale: Money,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct ItRow {
    pub block: String,
    pub label: String,
    pub rate: f64,
    pub opening_wdv: Money,
    pub additions_180_or_more: Money,
    pub additions_less_180: Money,
    pub sale_proceeds: Money,
    pub depreciation: Money,
    pub closing_wdv: Money,
    /// Short-term capital gain when sale proceeds exceed the block.
    pub stcg: Money,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct FarResult {
    pub ppe: Vec<PpeRow>,
    pub it: Vec<ItRow>,
    pub book_dep_total: Money,
    /// Problems found in the register itself (unknown class/block etc.).
    pub errors: Vec<String>,
    /// Books depreciation for the year of each asset, in register order
    /// (zero for assets outside the year).
    #[serde(default)]
    pub asset_dep: Vec<Money>,
}

fn paise(x: f64) -> Money {
    Money(x.round() as i64)
}

fn days(a: NaiveDate, b: NaiveDate) -> i64 {
    (b - a).num_days() + 1
}

pub fn compute(
    reg: &Register,
    pack: &DepPack,
    fy_start: NaiveDate,
    fy_end: NaiveDate,
) -> FarResult {
    let mut res = FarResult::default();
    let year_days = days(fy_start, fy_end) as f64;
    let mut ppe: BTreeMap<String, PpeRow> = BTreeMap::new();
    let mut it: BTreeMap<String, ItRow> = BTreeMap::new();
    for (k, v) in &reg.it_opening {
        let Some(b) = pack.block(k) else {
            res.errors
                .push(format!("Unknown Income-tax block '{k}' in opening WDV"));
            continue;
        };
        let r = it.entry(k.clone()).or_insert_with(|| ItRow {
            block: k.clone(),
            label: b.label.clone(),
            rate: b.rate,
            ..Default::default()
        });
        r.opening_wdv += *v;
    }

    res.asset_dep = vec![Money::ZERO; reg.assets.len()];
    for (ai, a) in reg.assets.iter().enumerate() {
        if a.sold_on.map(|d| d < fy_start).unwrap_or(false) || a.put_to_use > fy_end {
            continue;
        }
        let row = ppe.entry(a.ledger.clone()).or_insert_with(|| PpeRow {
            ledger: a.ledger.clone(),
            ..Default::default()
        });
        let existing = a.put_to_use < fy_start;
        if existing {
            row.gross_opening += a.cost;
            row.dep_opening += a.opening_acc_dep;
        } else {
            row.additions += a.cost;
        }
        let start = a.put_to_use.max(fy_start);
        let end = a.sold_on.unwrap_or(fy_end).min(fy_end);
        let used = days(start, end).max(0) as f64;
        let carrying = a.cost - a.opening_acc_dep;
        let residual = paise(a.cost.paise() as f64 * pack.residual_pct / 100.0);

        // Books depreciation for the year.
        let dep = match reg.basis {
            BookBasis::IncomeTaxRates => match pack.block(&a.it_block) {
                Some(b) => {
                    let half = !existing && days(a.put_to_use, fy_end) < 180;
                    let rate = if half { b.rate / 2.0 } else { b.rate };
                    let full_year = if a.sold_on.is_some() {
                        used / year_days
                    } else {
                        1.0
                    };
                    paise(carrying.paise() as f64 * rate / 100.0 * full_year)
                        .min(carrying)
                        .max(Money::ZERO)
                }
                None => {
                    res.errors.push(format!(
                        "{}: unknown Income-tax block '{}'",
                        a.name, a.it_block
                    ));
                    Money::ZERO
                }
            },
            BookBasis::ScheduleIiSlm | BookBasis::ScheduleIiWdv => {
                let life = a
                    .useful_life_years
                    .or_else(|| pack.class(&a.book_class).map(|c| c.life));
                match life {
                    Some(life) if life > 0.0 => {
                        let max_dep = (carrying - residual).max(Money::ZERO);
                        let raw = if reg.basis == BookBasis::ScheduleIiSlm {
                            (a.cost - residual).paise() as f64 / life * used / year_days
                        } else {
                            let rate = if a.cost.is_zero() {
                                0.0
                            } else {
                                1.0 - (residual.paise() as f64 / a.cost.paise() as f64)
                                    .powf(1.0 / life)
                            };
                            carrying.paise() as f64 * rate * used / year_days
                        };
                        paise(raw).min(max_dep)
                    }
                    _ => {
                        res.errors.push(format!(
                            "{}: unknown Schedule II class '{}' and no useful life given",
                            a.name, a.book_class
                        ));
                        Money::ZERO
                    }
                }
            }
        };
        row.dep_for_year += dep;
        res.asset_dep[ai] = dep;
        if let Some(sold) = a.sold_on {
            if sold <= fy_end {
                row.deletions += a.cost;
                row.dep_on_deletions += a.opening_acc_dep + dep;
                row.gain_on_sale += a.sale_value - (carrying - dep);
            }
        }

        // Income-tax block.
        match pack.block(&a.it_block) {
            Some(b) => {
                let r = it.entry(a.it_block.clone()).or_insert_with(|| ItRow {
                    block: b.key.clone(),
                    label: b.label.clone(),
                    rate: b.rate,
                    ..Default::default()
                });
                if !existing {
                    if days(a.put_to_use, fy_end) >= 180 {
                        r.additions_180_or_more += a.cost;
                    } else {
                        r.additions_less_180 += a.cost;
                    }
                }
                if a.sold_on.map(|d| d <= fy_end).unwrap_or(false) {
                    r.sale_proceeds += a.sale_value;
                }
            }
            None => res.errors.push(format!(
                "{}: unknown Income-tax block '{}'",
                a.name, a.it_block
            )),
        }
    }

    for r in ppe.values_mut() {
        r.gross_closing = r.gross_opening + r.additions - r.deletions;
        r.dep_closing = r.dep_opening + r.dep_for_year - r.dep_on_deletions;
        r.net_closing = r.gross_closing - r.dep_closing;
        r.net_opening = r.gross_opening - r.dep_opening;
        res.book_dep_total += r.dep_for_year;
    }
    for r in it.values_mut() {
        let mut full = r.opening_wdv + r.additions_180_or_more - r.sale_proceeds;
        let mut half = r.additions_less_180;
        if full.is_cr() {
            half += full;
            full = Money::ZERO;
        }
        if half.is_cr() {
            r.stcg = -half;
            half = Money::ZERO;
        }
        r.depreciation = paise(full.paise() as f64 * r.rate / 100.0)
            + paise(half.paise() as f64 * r.rate / 200.0);
        r.closing_wdv = full + half - r.depreciation;
    }
    res.ppe = ppe.into_values().collect();
    res.it = it.into_values().collect();
    res
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(y: i32, m: u32, dd: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, dd).unwrap()
    }

    #[test]
    fn income_tax_block_with_half_rate_and_sale() {
        let pack = DepPack::builtin();
        let reg = Register {
            assets: vec![
                Asset {
                    name: "Lathe".into(),
                    ledger: "Plant & Machinery".into(),
                    book_class: "plant_general".into(),
                    it_block: "plant_general".into(),
                    put_to_use: d(2025, 5, 1),
                    cost: Money::rupees(100_000),
                    opening_acc_dep: Money::ZERO,
                    sold_on: None,
                    sale_value: Money::ZERO,
                    useful_life_years: None,
                },
                Asset {
                    name: "Drill".into(),
                    ledger: "Plant & Machinery".into(),
                    book_class: "plant_general".into(),
                    it_block: "plant_general".into(),
                    put_to_use: d(2026, 1, 15),
                    cost: Money::rupees(40_000),
                    opening_acc_dep: Money::ZERO,
                    sold_on: None,
                    sale_value: Money::ZERO,
                    useful_life_years: None,
                },
            ],
            it_opening: [("plant_general".to_string(), Money::rupees(200_000))]
                .into_iter()
                .collect(),
            basis: BookBasis::IncomeTaxRates,
        };
        let r = compute(&reg, &pack, d(2025, 4, 1), d(2026, 3, 31));
        let b = &r.it[0];
        // (2,00,000 + 1,00,000) × 15% + 40,000 × 7.5% = 45,000 + 3,000.
        assert_eq!(b.depreciation, Money::rupees(48_000));
        assert_eq!(b.closing_wdv, Money::rupees(292_000));
        // Books on IT rates: lathe full rate 15,000 (≥180 days), drill half rate 3,000.
        assert_eq!(r.book_dep_total, Money::rupees(18_000));

        // Selling more than the block value: no depreciation, short-term capital gain.
        let mut reg2 = reg.clone();
        reg2.assets[0].sold_on = Some(d(2026, 2, 1));
        reg2.assets[0].sale_value = Money::rupees(400_000);
        let r2 = compute(&reg2, &pack, d(2025, 4, 1), d(2026, 3, 31));
        // Block: 2,00,000 + 1,00,000 − 4,00,000 = −1,00,000 → absorbs the 40,000 half addition → STCG 60,000.
        assert_eq!(r2.it[0].stcg, Money::rupees(60_000));
        assert_eq!(r2.it[0].depreciation, Money::ZERO);
    }

    #[test]
    fn schedule_ii_slm_and_wdv() {
        let pack = DepPack::builtin();
        let laptop = Asset {
            name: "Laptop".into(),
            ledger: "Computers".into(),
            book_class: "computers_end_user".into(),
            it_block: "computers".into(),
            put_to_use: d(2024, 4, 1),
            cost: Money::rupees(60_000),
            opening_acc_dep: Money::rupees(19_000),
            sold_on: None,
            sale_value: Money::ZERO,
            useful_life_years: None,
        };
        let slm = compute(
            &Register {
                assets: vec![laptop.clone()],
                it_opening: Default::default(),
                basis: BookBasis::ScheduleIiSlm,
            },
            &pack,
            d(2025, 4, 1),
            d(2026, 3, 31),
        );
        // (60,000 − 3,000 residual) / 3 years = 19,000 for a full year.
        assert_eq!(slm.ppe[0].dep_for_year, Money::rupees(19_000));
        assert_eq!(slm.ppe[0].net_closing, Money::rupees(22_000));
        let wdv = compute(
            &Register {
                assets: vec![laptop],
                it_opening: Default::default(),
                basis: BookBasis::ScheduleIiWdv,
            },
            &pack,
            d(2025, 4, 1),
            d(2026, 3, 31),
        );
        // Rate = 1 − 0.05^(1/3) = 63.16%; on carrying 41,000 → about 25,896 (never below residual).
        let dep = wdv.ppe[0].dep_for_year.paise();
        assert!((2_589_000..=2_590_500).contains(&dep), "{dep}");
        assert!(wdv.ppe[0].net_closing >= Money::rupees(3_000));
    }
}

/// The register for the next year: assets sold during this year drop out,
/// accumulated depreciation and Income-tax WDV carry forward.
pub fn roll_forward(reg: &Register, res: &FarResult, fy_end: NaiveDate) -> Register {
    let mut assets = Vec::new();
    for (i, a) in reg.assets.iter().enumerate() {
        if a.sold_on.map(|d| d <= fy_end).unwrap_or(false) {
            continue;
        }
        let mut n = a.clone();
        n.opening_acc_dep += res.asset_dep.get(i).copied().unwrap_or_default();
        assets.push(n);
    }
    let it_opening = res
        .it
        .iter()
        .filter(|r| !r.closing_wdv.is_zero())
        .map(|r| (r.block.clone(), r.closing_wdv))
        .collect();
    Register {
        assets,
        it_opening,
        basis: reg.basis,
    }
}

#[cfg(test)]
mod roll_tests {
    use super::*;

    fn d(y: i32, m: u32, dd: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, dd).unwrap()
    }

    #[test]
    fn roll_forward_carries_depreciation_and_drops_sold_assets() {
        let pack = DepPack::builtin();
        let a = |name: &str, put: NaiveDate, sold: Option<NaiveDate>| Asset {
            name: name.into(),
            ledger: "Plant & Machinery".into(),
            book_class: "plant_general".into(),
            it_block: "plant_general".into(),
            put_to_use: put,
            cost: Money::rupees(100_000),
            opening_acc_dep: Money::rupees(10_000),
            sold_on: sold,
            sale_value: Money::rupees(50_000),
            useful_life_years: None,
        };
        let reg = Register {
            assets: vec![
                a("Kept", d(2023, 4, 1), None),
                a("Sold", d(2023, 4, 1), Some(d(2025, 12, 1))),
            ],
            it_opening: [("plant_general".to_string(), Money::rupees(150_000))].into(),
            basis: BookBasis::IncomeTaxRates,
        };
        let res = compute(&reg, &pack, d(2025, 4, 1), d(2026, 3, 31));
        let next = roll_forward(&reg, &res, d(2026, 3, 31));
        assert_eq!(next.assets.len(), 1);
        assert_eq!(next.assets[0].name, "Kept");
        assert_eq!(
            next.assets[0].opening_acc_dep,
            Money::rupees(10_000) + res.asset_dep[0]
        );
        assert!(res.asset_dep[0] > Money::ZERO);
        let closing = res
            .it
            .iter()
            .find(|r| r.block == "plant_general")
            .unwrap()
            .closing_wdv;
        assert_eq!(next.it_opening["plant_general"], closing);
    }
}
