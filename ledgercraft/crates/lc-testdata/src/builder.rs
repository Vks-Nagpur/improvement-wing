//! Book builder: define ledgers, post vouchers chronologically, then derive
//! consistent current-year and previous-year trial balances.

use chrono::NaiveDate;
use lc_core::groups::{Class, GroupResolver};
use lc_core::model::{
    norm_name, Engagement, EntityType, Ledger, TrialBalance, Voucher, VoucherLine,
};
use lc_core::Money;
use std::collections::{BTreeSet, HashMap, HashSet};

#[derive(Debug, Clone, Default)]
pub struct Expected {
    /// Exact finding keys (`CODE:subject`) the engine must report.
    pub keys: BTreeSet<String>,
    pub profit_cy: Money,
    pub profit_py: Option<Money>,
}

struct Spec {
    name: String,
    group: String,
    opening: Money,
    tags: Vec<String>,
    closing_stock: Option<Money>,
    /// Previous-year TB balance of a stock ledger (= opening stock of last year).
    py_stock_tb: Option<Money>,
    in_py: bool,
}

pub struct Builder {
    pub entity_name: String,
    pub entity_type: EntityType,
    pub fy_start: NaiveDate,
    pub fy_end: NaiveDate,
    specs: Vec<Spec>,
    index: HashMap<String, usize>,
    groups: Vec<(String, String)>,
    vouchers: Vec<Voucher>,
    bal: Vec<Money>,
    counters: HashMap<String, u32>,
    sigs: HashSet<(NaiveDate, Vec<(usize, i64)>)>,
    py_pl: Vec<(String, String, Money)>,
    py_only: Vec<(String, String, Money)>,
    opening_tweaks: Vec<(usize, Money)>,
    closing_tweaks: Vec<(usize, Money)>,
    /// Lines that refer to ledgers deliberately missing from the TB.
    ghost_lines: Vec<(usize, String)>,
    pub expected: Expected,
    pub with_py: bool,
    pub far: Option<lc_core::far::Register>,
}

impl Builder {
    pub fn new(name: &str, entity: EntityType, fy_start_year: i32) -> Builder {
        Builder {
            entity_name: name.to_string(),
            entity_type: entity,
            fy_start: NaiveDate::from_ymd_opt(fy_start_year, 4, 1).unwrap(),
            fy_end: NaiveDate::from_ymd_opt(fy_start_year + 1, 3, 31).unwrap(),
            specs: Vec::new(),
            index: HashMap::new(),
            groups: Vec::new(),
            vouchers: Vec::new(),
            bal: Vec::new(),
            counters: HashMap::new(),
            sigs: HashSet::new(),
            py_pl: Vec::new(),
            py_only: Vec::new(),
            opening_tweaks: Vec::new(),
            closing_tweaks: Vec::new(),
            ghost_lines: Vec::new(),
            expected: Expected::default(),
            with_py: true,
            far: None,
        }
    }

    pub fn day(&self, n: i64) -> NaiveDate {
        self.fy_start + chrono::Duration::days(n)
    }

    pub fn group(&mut self, name: &str, parent: &str) {
        self.groups.push((name.to_string(), parent.to_string()));
    }

    /// Opening balance: Dr positive, Cr negative.
    pub fn ledger(&mut self, name: &str, group: &str, opening: Money) {
        self.ledger_tagged(name, group, opening, &[]);
    }

    pub fn ledger_tagged(&mut self, name: &str, group: &str, opening: Money, tags: &[&str]) {
        assert!(
            !self.index.contains_key(&norm_name(name)),
            "duplicate ledger {name}"
        );
        self.index.insert(norm_name(name), self.specs.len());
        self.specs.push(Spec {
            name: name.to_string(),
            group: group.to_string(),
            opening,
            tags: tags.iter().map(|t| t.to_string()).collect(),
            closing_stock: None,
            py_stock_tb: None,
            in_py: true,
        });
        self.bal.push(opening);
    }

    pub fn stock(
        &mut self,
        name: &str,
        group: &str,
        opening: Money,
        closing_stock: Money,
        py_opening_stock: Money,
    ) {
        self.ledger(name, group, opening);
        let i = self.idx(name);
        self.specs[i].closing_stock = Some(closing_stock);
        self.specs[i].py_stock_tb = Some(py_opening_stock);
    }

    /// Set an opening balance before any voucher is posted.
    pub fn set_opening(&mut self, name: &str, opening: Money) {
        assert!(
            self.vouchers.is_empty(),
            "set_opening must be called before posting vouchers"
        );
        let i = self.idx(name);
        self.specs[i].opening = opening;
        self.bal[i] = opening;
    }

    /// Sum of all opening balances so far (should end at zero).
    pub fn opening_total(&self) -> Money {
        self.specs.iter().map(|s| s.opening).sum()
    }

    pub fn has(&self, name: &str) -> bool {
        self.index.contains_key(&norm_name(name))
    }

    fn idx(&self, name: &str) -> usize {
        *self
            .index
            .get(&norm_name(name))
            .unwrap_or_else(|| panic!("no ledger {name}"))
    }

    pub fn bal(&self, name: &str) -> Money {
        self.bal[self.idx(name)]
    }

    pub fn not_in_py(&mut self, name: &str) {
        let i = self.idx(name);
        self.specs[i].in_py = false;
    }

    pub fn py_pl(&mut self, name: &str, group: &str, closing: Money) {
        self.py_pl
            .push((name.to_string(), group.to_string(), closing));
    }

    pub fn py_only(&mut self, name: &str, group: &str, closing: Money) {
        self.py_only
            .push((name.to_string(), group.to_string(), closing));
    }

    /// Change this year's opening (and closing) without changing last year's closing.
    pub fn tweak_opening(&mut self, name: &str, delta: Money) {
        let i = self.idx(name);
        self.opening_tweaks.push((i, delta));
    }

    /// Change the TB closing without a voucher (simulates an edited trial balance).
    pub fn tweak_closing(&mut self, name: &str, delta: Money) {
        let i = self.idx(name);
        self.closing_tweaks.push((i, delta));
    }

    pub fn expect(&mut self, key: impl Into<String>) {
        self.expected.keys.insert(key.into());
    }

    fn signature(
        &self,
        date: NaiveDate,
        lines: &[(&str, Money)],
    ) -> (NaiveDate, Vec<(usize, i64)>) {
        let mut s: Vec<(usize, i64)> = lines
            .iter()
            .map(|(n, a)| (self.idx(n), a.paise()))
            .collect();
        s.sort();
        (date, s)
    }

    /// Post a balanced voucher. Returns `None` (and posts nothing) if an
    /// identical voucher already exists on that date.
    pub fn try_v(
        &mut self,
        date: NaiveDate,
        vtype: &str,
        lines: &[(&str, Money)],
    ) -> Option<String> {
        let total: Money = lines.iter().map(|(_, a)| *a).sum();
        assert!(
            total.is_zero(),
            "unbalanced voucher in generator: {lines:?}"
        );
        let sig = self.signature(date, lines);
        if self.sigs.contains(&sig) {
            return None;
        }
        self.sigs.insert(sig);
        Some(self.post(date, vtype, lines))
    }

    pub fn v(&mut self, date: NaiveDate, vtype: &str, lines: &[(&str, Money)]) -> String {
        self.try_v(date, vtype, lines)
            .unwrap_or_else(|| panic!("duplicate voucher in generator: {date} {lines:?}"))
    }

    /// Post without balance / duplicate checks (for planting errors).
    pub fn v_raw(&mut self, date: NaiveDate, vtype: &str, lines: &[(&str, Money)]) -> String {
        self.post(date, vtype, lines)
    }

    /// Post a voucher where one line refers to a ledger that will not be in the TB.
    pub fn v_with_ghost(
        &mut self,
        date: NaiveDate,
        vtype: &str,
        ghost: &str,
        amount: Money,
        other: &str,
    ) -> String {
        let key = self.post(date, vtype, &[(other, -amount)]);
        let k = self.vouchers.len() - 1;
        self.vouchers[k].lines.insert(
            0,
            VoucherLine {
                ledger: ghost.to_string(),
                amount,
            },
        );
        self.ghost_lines.push((k, ghost.to_string()));
        key
    }

    fn post(&mut self, date: NaiveDate, vtype: &str, lines: &[(&str, Money)]) -> String {
        let n = self.counters.entry(vtype.to_string()).or_insert(0);
        *n += 1;
        let number = n.to_string();
        let mut vl = Vec::with_capacity(lines.len());
        for (name, amt) in lines {
            let i = self.idx(name);
            self.bal[i] += *amt;
            vl.push(VoucherLine {
                ledger: self.specs[i].name.clone(),
                amount: *amt,
            });
        }
        let v = Voucher {
            date,
            number,
            vtype: vtype.to_string(),
            narration: String::new(),
            lines: vl,
        };
        let key = v.key();
        self.vouchers.push(v);
        key
    }

    pub fn finish(mut self) -> (Engagement, Expected) {
        let cy_groups = TrialBalance {
            ledgers: vec![],
            groups: self.groups.clone(),
        };
        let res = GroupResolver::new(&cy_groups);
        let class = |g: &str| res.resolve(g).ok();

        // Previous year.
        let py = if self.with_py {
            let mut ledgers = Vec::new();
            let mut py_pl_net = Money::ZERO; // Dr positive = loss
            for (n, g, c) in &self.py_pl {
                py_pl_net += *c;
                ledgers.push(Ledger {
                    name: n.clone(),
                    group: g.clone(),
                    opening: Money::ZERO,
                    closing: *c,
                    closing_stock: None,
                    tags: vec![],
                });
            }
            let mut stock_change = Money::ZERO;
            let mut plac: Option<usize> = None;
            for s in &self.specs {
                let c = class(&s.group);
                if !s.in_py || c.map(|c| c.nature().is_pl()).unwrap_or(false) {
                    continue;
                }
                let mut l = Ledger {
                    name: s.name.clone(),
                    group: s.group.clone(),
                    opening: Money::ZERO,
                    closing: s.opening,
                    closing_stock: None,
                    tags: s.tags.clone(),
                };
                if c == Some(Class::StockInHand) {
                    let tb = s.py_stock_tb.unwrap_or(s.opening);
                    l.closing = tb;
                    l.closing_stock = Some(s.opening);
                    stock_change += s.opening - tb;
                }
                if c == Some(Class::ProfitLossAc) {
                    plac = Some(ledgers.len());
                }
                ledgers.push(l);
            }
            for (n, g, c) in &self.py_only {
                ledgers.push(Ledger {
                    name: n.clone(),
                    group: g.clone(),
                    opening: Money::ZERO,
                    closing: *c,
                    closing_stock: None,
                    tags: vec![],
                });
            }
            py_pl_net -= stock_change;
            let p = plac.expect(
                "scenario needs a Profit & Loss A/c ledger when previous year is generated",
            );
            ledgers[p].closing -= py_pl_net;
            // py_only ledgers must be balanced by the scenario itself.
            let total: Money = ledgers.iter().map(|l| l.closing).sum();
            assert!(
                total.is_zero(),
                "generated previous-year TB does not balance: {total}"
            );
            self.expected.profit_py = Some(-py_pl_net);
            Some(TrialBalance {
                ledgers,
                groups: self.groups.clone(),
            })
        } else {
            None
        };

        for (i, d) in &self.opening_tweaks {
            self.specs[*i].opening += *d;
            self.bal[*i] += *d;
        }
        for (i, d) in &self.closing_tweaks {
            self.bal[*i] += *d;
        }

        let mut cy_profit = Money::ZERO;
        let mut ledgers = Vec::with_capacity(self.specs.len());
        for (i, s) in self.specs.iter().enumerate() {
            let c = class(&s.group);
            let closing = self.bal[i];
            match c {
                Some(cl) if cl.nature().is_pl() => cy_profit -= closing,
                Some(Class::StockInHand) => {
                    cy_profit += s.closing_stock.unwrap_or(closing) - closing
                }
                _ => {}
            }
            ledgers.push(Ledger {
                name: s.name.clone(),
                group: s.group.clone(),
                opening: s.opening,
                closing,
                closing_stock: s.closing_stock,
                tags: s.tags.clone(),
            });
        }
        self.expected.profit_cy = cy_profit;

        let eng = Engagement {
            entity_name: self.entity_name.clone(),
            entity_type: self.entity_type,
            fy_start: self.fy_start,
            fy_end: self.fy_end,
            cy: TrialBalance {
                ledgers,
                groups: self.groups.clone(),
            },
            py,
            vouchers: self.vouchers,
            mapping_memory: HashMap::new(),
            mapping_context: HashMap::new(),
            format_pack: None,
            far: self.far.clone(),
            profit_sharing: Vec::new(),
        };
        (eng, self.expected)
    }
}
