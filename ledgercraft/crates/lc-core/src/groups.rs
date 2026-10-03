//! Knowledge of the standard (reserved) Tally groups and resolution of
//! user-defined groups to them.

use crate::model::{norm_name, TrialBalance};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Nature {
    Asset,
    Liability,
    Capital,
    Income,
    Expense,
}

impl Nature {
    pub fn is_pl(self) -> bool {
        matches!(self, Nature::Income | Nature::Expense)
    }
}

/// The 28 predefined Tally groups plus the Profit & Loss A/c ledger.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Class {
    BranchDivisions,
    CapitalAccount,
    CurrentAssets,
    CurrentLiabilities,
    DirectExpenses,
    DirectIncomes,
    FixedAssets,
    IndirectExpenses,
    IndirectIncomes,
    Investments,
    LoansLiability,
    MiscExpensesAsset,
    PurchaseAccounts,
    SalesAccounts,
    SuspenseAc,
    BankAccounts,
    BankOdAc,
    CashInHand,
    DepositsAsset,
    DutiesTaxes,
    LoansAdvancesAsset,
    Provisions,
    ReservesSurplus,
    SecuredLoans,
    StockInHand,
    SundryCreditors,
    SundryDebtors,
    UnsecuredLoans,
    ProfitLossAc,
}

impl Class {
    pub fn nature(self) -> Nature {
        use Class::*;
        match self {
            CapitalAccount | ReservesSurplus | ProfitLossAc => Nature::Capital,
            CurrentAssets | FixedAssets | Investments | MiscExpensesAsset | BankAccounts
            | CashInHand | DepositsAsset | LoansAdvancesAsset | StockInHand | SundryDebtors => {
                Nature::Asset
            }
            BranchDivisions | CurrentLiabilities | LoansLiability | SuspenseAc | BankOdAc
            | DutiesTaxes | Provisions | SecuredLoans | SundryCreditors | UnsecuredLoans => {
                Nature::Liability
            }
            DirectExpenses | IndirectExpenses | PurchaseAccounts => Nature::Expense,
            DirectIncomes | IndirectIncomes | SalesAccounts => Nature::Income,
        }
    }

    /// Every reserved group, in the order Tally lists them.
    pub const ALL: [Class; 29] = {
        use Class::*;
        [
            CapitalAccount,
            ReservesSurplus,
            LoansLiability,
            SecuredLoans,
            UnsecuredLoans,
            BankOdAc,
            CurrentLiabilities,
            SundryCreditors,
            DutiesTaxes,
            Provisions,
            FixedAssets,
            Investments,
            CurrentAssets,
            StockInHand,
            DepositsAsset,
            LoansAdvancesAsset,
            SundryDebtors,
            CashInHand,
            BankAccounts,
            MiscExpensesAsset,
            SalesAccounts,
            DirectIncomes,
            IndirectIncomes,
            PurchaseAccounts,
            DirectExpenses,
            IndirectExpenses,
            SuspenseAc,
            BranchDivisions,
            ProfitLossAc,
        ]
    };

    pub fn label(self) -> &'static str {
        use Class::*;
        match self {
            BranchDivisions => "Branch / Divisions",
            CapitalAccount => "Capital Account",
            CurrentAssets => "Current Assets",
            CurrentLiabilities => "Current Liabilities",
            DirectExpenses => "Direct Expenses",
            DirectIncomes => "Direct Incomes",
            FixedAssets => "Fixed Assets",
            IndirectExpenses => "Indirect Expenses",
            IndirectIncomes => "Indirect Incomes",
            Investments => "Investments",
            LoansLiability => "Loans (Liability)",
            MiscExpensesAsset => "Misc. Expenses (ASSET)",
            PurchaseAccounts => "Purchase Accounts",
            SalesAccounts => "Sales Accounts",
            SuspenseAc => "Suspense A/c",
            BankAccounts => "Bank Accounts",
            BankOdAc => "Bank OD A/c",
            CashInHand => "Cash-in-Hand",
            DepositsAsset => "Deposits (Asset)",
            DutiesTaxes => "Duties & Taxes",
            LoansAdvancesAsset => "Loans & Advances (Asset)",
            Provisions => "Provisions",
            ReservesSurplus => "Reserves & Surplus",
            SecuredLoans => "Secured Loans",
            StockInHand => "Stock-in-Hand",
            SundryCreditors => "Sundry Creditors",
            SundryDebtors => "Sundry Debtors",
            UnsecuredLoans => "Unsecured Loans",
            ProfitLossAc => "Profit & Loss A/c",
        }
    }

    pub fn is_loan(self) -> bool {
        matches!(
            self,
            Class::SecuredLoans | Class::UnsecuredLoans | Class::LoansLiability
        )
    }
    pub fn is_cash(self) -> bool {
        self == Class::CashInHand
    }
    pub fn is_bank(self) -> bool {
        matches!(self, Class::BankAccounts | Class::BankOdAc)
    }
}

/// Lookup of reserved group names (normalised) including common aliases.
fn reserved(name: &str) -> Option<Class> {
    use Class::*;
    Some(match name {
        "branch divisions" | "branch division" => BranchDivisions,
        "capital account" | "capital a c" | "capital" => CapitalAccount,
        "current assets" => CurrentAssets,
        "current liabilities" => CurrentLiabilities,
        "direct expenses" | "expenses direct" => DirectExpenses,
        "direct incomes" | "direct income" | "income direct" => DirectIncomes,
        "fixed assets" => FixedAssets,
        "indirect expenses" | "expenses indirect" => IndirectExpenses,
        "indirect incomes" | "indirect income" | "income indirect" => IndirectIncomes,
        "investments" => Investments,
        "loans liability" | "loans" => LoansLiability,
        "misc expenses asset" | "miscellaneous expenses asset" => MiscExpensesAsset,
        "purchase accounts" | "purchase account" | "purchases" => PurchaseAccounts,
        "sales accounts" | "sales account" | "sales" => SalesAccounts,
        "suspense a c" | "suspense ac" | "suspense account" | "suspense" => SuspenseAc,
        "bank accounts" | "bank account" => BankAccounts,
        "bank od a c" | "bank occ a c" | "bank od ac" | "bank od" | "bank overdraft" => BankOdAc,
        "cash in hand" | "cash in hand a c" => CashInHand,
        "deposits asset" => DepositsAsset,
        "duties & taxes" | "duties and taxes" | "duties taxes" => DutiesTaxes,
        "loans & advances asset" | "loans and advances asset" => LoansAdvancesAsset,
        "provisions" => Provisions,
        "reserves & surplus" | "reserves and surplus" | "retained earnings" => ReservesSurplus,
        "secured loans" => SecuredLoans,
        "stock in hand" => StockInHand,
        "sundry creditors" | "trade payables" => SundryCreditors,
        "sundry debtors" | "trade receivables" => SundryDebtors,
        "unsecured loans" => UnsecuredLoans,
        "profit & loss a c"
        | "profit and loss a c"
        | "profit & loss account"
        | "profit and loss account"
        | "primary profit & loss a c" => ProfitLossAc,
        _ => return None,
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GroupError {
    Unknown(String),
    Cycle(String),
}

/// Resolves any group name to its standard class by walking up the user-defined tree.
pub struct GroupResolver {
    parent: HashMap<String, String>,
}

impl GroupResolver {
    pub fn new(tb: &TrialBalance) -> Self {
        let parent = tb
            .groups
            .iter()
            .map(|(g, p)| (norm_name(g), norm_name(p)))
            .collect();
        GroupResolver { parent }
    }

    pub fn resolve(&self, group: &str) -> Result<Class, GroupError> {
        let mut cur = norm_name(group);
        let mut seen = 0;
        loop {
            if let Some(c) = reserved(&cur) {
                return Ok(c);
            }
            match self.parent.get(&cur) {
                Some(p) => {
                    cur = p.clone();
                    seen += 1;
                    if seen > 64 {
                        return Err(GroupError::Cycle(group.to_string()));
                    }
                }
                None => return Err(GroupError::Unknown(group.to_string())),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn resolves_user_groups() {
        let tb = TrialBalance {
            ledgers: vec![],
            groups: vec![
                ("Debtors - Domestic".into(), "Sundry Debtors".into()),
                ("Admin Exp".into(), "Indirect Expenses".into()),
                ("Loop A".into(), "Loop B".into()),
                ("Loop B".into(), "Loop A".into()),
            ],
        };
        let r = GroupResolver::new(&tb);
        assert_eq!(r.resolve("Debtors - Domestic"), Ok(Class::SundryDebtors));
        assert_eq!(r.resolve("Bank OD A/c"), Ok(Class::BankOdAc));
        assert_eq!(r.resolve("Duties & Taxes"), Ok(Class::DutiesTaxes));
        assert!(matches!(r.resolve("Nothing"), Err(GroupError::Unknown(_))));
        assert!(matches!(r.resolve("Loop A"), Err(GroupError::Cycle(_))));
    }
}
