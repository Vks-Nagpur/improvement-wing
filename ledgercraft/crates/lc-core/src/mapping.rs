//! Ledger -> financial statement head mapping.
//!
//! Order: (1) remembered mapping, (2) name rules within the ledger's nature,
//! (3) standard-group default. Then presentation reclassification by balance
//! side (no netting of assets and liabilities).

use crate::groups::{Class, Nature};
use crate::model::norm_name;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Head {
    // Owners' funds / equity
    Capital,
    ReservesSurplus,
    // Liabilities
    LtBorrowings,
    OtherLtLiabilities,
    LtProvisions,
    StBorrowings,
    TradePayables,
    OtherCurrentLiabilities,
    StProvisions,
    // Assets
    Ppe,
    Intangibles,
    Cwip,
    NcInvestments,
    LtLoansAdvances,
    OtherNcAssets,
    CurrentInvestments,
    Inventories,
    TradeReceivables,
    CashBank,
    StLoansAdvances,
    OtherCurrentAssets,
    // Profit and loss
    RevenueOps,
    OtherIncome,
    Purchases,
    ChangeInInventories,
    EmployeeBenefits,
    FinanceCosts,
    Depreciation,
    PartnersRemuneration,
    OtherExpenses,
    TaxExpense,
}

impl Head {
    pub fn id(self) -> String {
        serde_json::to_value(self)
            .ok()
            .and_then(|v| v.as_str().map(String::from))
            .unwrap_or_default()
    }
    pub fn from_id(s: &str) -> Option<Head> {
        serde_json::from_value(serde_json::Value::String(s.to_string())).ok()
    }
    /// Plain label used in the app.
    pub fn label(self) -> &'static str {
        use Head::*;
        match self {
            Capital => "Capital / Share capital",
            ReservesSurplus => "Reserves and surplus",
            LtBorrowings => "Long-term borrowings",
            OtherLtLiabilities => "Other long-term liabilities",
            LtProvisions => "Long-term provisions",
            StBorrowings => "Short-term borrowings",
            TradePayables => "Trade payables",
            OtherCurrentLiabilities => "Other current liabilities",
            StProvisions => "Short-term provisions",
            Ppe => "Property, plant and equipment",
            Intangibles => "Intangible assets",
            Cwip => "Capital work-in-progress",
            NcInvestments => "Non-current investments",
            LtLoansAdvances => "Long-term loans and advances",
            OtherNcAssets => "Other non-current assets",
            CurrentInvestments => "Current investments",
            Inventories => "Inventories",
            TradeReceivables => "Trade receivables",
            CashBank => "Cash and bank balances",
            StLoansAdvances => "Short-term loans and advances",
            OtherCurrentAssets => "Other current assets",
            RevenueOps => "Revenue from operations",
            OtherIncome => "Other income",
            Purchases => "Purchases of stock-in-trade",
            ChangeInInventories => "Changes in inventories",
            EmployeeBenefits => "Employee benefits expense",
            FinanceCosts => "Finance costs",
            Depreciation => "Depreciation and amortisation",
            PartnersRemuneration => "Partners' / managerial remuneration",
            OtherExpenses => "Other expenses",
            TaxExpense => "Tax expense",
        }
    }

    pub fn nature(self) -> Nature {
        use Head::*;
        match self {
            Capital | ReservesSurplus => Nature::Capital,
            LtBorrowings
            | OtherLtLiabilities
            | LtProvisions
            | StBorrowings
            | TradePayables
            | OtherCurrentLiabilities
            | StProvisions => Nature::Liability,
            Ppe | Intangibles | Cwip | NcInvestments | LtLoansAdvances | OtherNcAssets
            | CurrentInvestments | Inventories | TradeReceivables | CashBank | StLoansAdvances
            | OtherCurrentAssets => Nature::Asset,
            RevenueOps | OtherIncome => Nature::Income,
            Purchases | ChangeInInventories | EmployeeBenefits | FinanceCosts | Depreciation
            | PartnersRemuneration | OtherExpenses | TaxExpense => Nature::Expense,
        }
    }
    pub const ALL: [Head; 31] = {
        use Head::*;
        [
            Capital,
            ReservesSurplus,
            LtBorrowings,
            OtherLtLiabilities,
            LtProvisions,
            StBorrowings,
            TradePayables,
            OtherCurrentLiabilities,
            StProvisions,
            Ppe,
            Intangibles,
            Cwip,
            NcInvestments,
            LtLoansAdvances,
            OtherNcAssets,
            CurrentInvestments,
            Inventories,
            TradeReceivables,
            CashBank,
            StLoansAdvances,
            OtherCurrentAssets,
            RevenueOps,
            OtherIncome,
            Purchases,
            ChangeInInventories,
            EmployeeBenefits,
            FinanceCosts,
            Depreciation,
            PartnersRemuneration,
            OtherExpenses,
            TaxExpense,
        ]
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MapSource {
    Memory,
    NameRule,
    GroupDefault,
}

/// How sure LedgerCraft is of a ledger's placement (see TRUTH-MODEL.md §3–5).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MapStatus {
    /// Only one line is possible for this standard group.
    #[default]
    Rule,
    /// Proposed by LedgerCraft; the user must confirm before a final copy.
    Suggested,
    /// Confirmed earlier, but the group or balance side has changed since.
    Review,
    /// Chosen or confirmed by the user.
    Confirmed,
    /// No placement.
    Unmapped,
}

impl MapStatus {
    pub fn needs_user(self) -> bool {
        matches!(
            self,
            MapStatus::Suggested | MapStatus::Review | MapStatus::Unmapped
        )
    }
}

/// Standard groups whose ledgers have exactly one possible line in the
/// formats, so the default placement is certain.
pub fn is_unambiguous(class: Class) -> bool {
    use Class::*;
    matches!(
        class,
        CapitalAccount
            | ReservesSurplus
            | ProfitLossAc
            | BankOdAc
            | SundryCreditors
            | DutiesTaxes
            | StockInHand
            | SundryDebtors
            | BankAccounts
            | CashInHand
            | SalesAccounts
            | IndirectIncomes
            | PurchaseAccounts
            | DirectExpenses
            | IndirectExpenses
    )
}

/// Context stored with a confirmed mapping: normalised group and balance side.
pub fn memory_context(group: &str, balance: crate::money::Money) -> String {
    format!(
        "{}|{}",
        norm_name(group),
        if balance.is_cr() { "cr" } else { "dr" }
    )
}

/// Word-boundary keyword test on a normalised name.
pub fn has_word(name_norm: &str, words: &[&str]) -> bool {
    let padded = format!(" {name_norm} ");
    words.iter().any(|w| padded.contains(&format!(" {w} ")))
}

/// Substring keyword test on a normalised name (for stems like "depreciat").
pub fn has_stem(name_norm: &str, stems: &[&str]) -> bool {
    stems.iter().any(|s| name_norm.contains(s))
}

pub fn group_default(class: Class) -> Head {
    use Class::*;
    match class {
        CapitalAccount => Head::Capital,
        ReservesSurplus | ProfitLossAc => Head::ReservesSurplus,
        LoansLiability | SecuredLoans | UnsecuredLoans => Head::LtBorrowings,
        BankOdAc => Head::StBorrowings,
        SundryCreditors => Head::TradePayables,
        CurrentLiabilities | DutiesTaxes | BranchDivisions | SuspenseAc => {
            Head::OtherCurrentLiabilities
        }
        Provisions => Head::StProvisions,
        FixedAssets => Head::Ppe,
        Investments => Head::NcInvestments,
        DepositsAsset => Head::LtLoansAdvances,
        LoansAdvancesAsset => Head::StLoansAdvances,
        StockInHand => Head::Inventories,
        SundryDebtors => Head::TradeReceivables,
        BankAccounts | CashInHand => Head::CashBank,
        CurrentAssets | MiscExpensesAsset => Head::OtherCurrentAssets,
        SalesAccounts | DirectIncomes => Head::RevenueOps,
        IndirectIncomes => Head::OtherIncome,
        PurchaseAccounts => Head::Purchases,
        DirectExpenses | IndirectExpenses => Head::OtherExpenses,
    }
}

/// Name-based refinement inside the same nature. Never moves a ledger to a
/// different nature (that is reported as a mis-grouping instead).
pub fn name_rule(class: Class, name: &str) -> Option<Head> {
    let n = norm_name(name);
    match class.nature() {
        Nature::Expense if class != Class::PurchaseAccounts => {
            if has_word(&n, &["partner", "partners", "partner s"])
                && has_stem(&n, &["remuneration", "salary", "interest"])
            {
                Some(Head::PartnersRemuneration)
            } else if has_stem(&n, &["depreciat", "amortis", "amortiz"]) {
                Some(Head::Depreciation)
            } else if has_stem(
                &n,
                &["income tax", "tax expense", "current tax", "deferred tax"],
            ) {
                Some(Head::TaxExpense)
            } else if has_stem(&n, &["interest"]) && !has_stem(&n, &["penal"]) {
                Some(Head::FinanceCosts)
            } else if has_stem(
                &n,
                &[
                    "salar",
                    "wage",
                    "bonus",
                    "staff welfare",
                    "gratuity",
                    "provident",
                    "leave encash",
                ],
            ) || has_word(&n, &["pf", "esi", "esic"])
            {
                Some(Head::EmployeeBenefits)
            } else {
                None
            }
        }
        Nature::Asset if class == Class::FixedAssets => {
            if has_stem(&n, &["capital work", "cwip"]) {
                Some(Head::Cwip)
            } else if has_stem(
                &n,
                &[
                    "software",
                    "goodwill",
                    "trademark",
                    "patent",
                    "copyright",
                    "licence",
                    "license",
                ],
            ) {
                Some(Head::Intangibles)
            } else {
                None
            }
        }
        Nature::Income if class == Class::IndirectIncomes => None,
        _ => None,
    }
}

/// Presentation reclassification by balance side. Returns (head, rule code).
pub fn reclass_by_side(
    head: Head,
    class: Class,
    balance: crate::money::Money,
) -> Option<(Head, &'static str)> {
    match head {
        Head::TradeReceivables if balance.is_cr() => {
            Some((Head::OtherCurrentLiabilities, "RECLASS_DEBTOR_CR"))
        }
        Head::TradePayables if balance.is_dr() => {
            Some((Head::StLoansAdvances, "RECLASS_CREDITOR_DR"))
        }
        Head::CashBank if balance.is_cr() && class == Class::BankAccounts => {
            Some((Head::StBorrowings, "RECLASS_BANK_CR"))
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ids_roundtrip() {
        for h in Head::ALL {
            assert_eq!(Head::from_id(&h.id()), Some(h));
        }
        assert_eq!(Head::LtBorrowings.id(), "LT_BORROWINGS");
    }
    #[test]
    fn name_rules() {
        assert_eq!(
            name_rule(Class::IndirectExpenses, "Salary"),
            Some(Head::EmployeeBenefits)
        );
        assert_eq!(
            name_rule(Class::IndirectExpenses, "Interest on Partners' Capital"),
            Some(Head::PartnersRemuneration)
        );
        assert_eq!(
            name_rule(Class::IndirectExpenses, "Interest on Term Loan"),
            Some(Head::FinanceCosts)
        );
        assert_eq!(
            name_rule(Class::IndirectExpenses, "Depreciation"),
            Some(Head::Depreciation)
        );
        assert_eq!(name_rule(Class::IndirectExpenses, "Rent"), None);
        assert_eq!(
            name_rule(Class::FixedAssets, "Tally Software"),
            Some(Head::Intangibles)
        );
    }
}
