//! Standard wording for "Entity information" and "Significant accounting
//! policies". It is a starting text: the preparer must review it (a warning
//! is always raised).

use super::ReportOptions;
use crate::far::BookBasis;
use crate::model::{Engagement, EntityType};

pub fn entity_information(eng: &Engagement, opt: &ReportOptions) -> Vec<String> {
    let mut v = vec![format!(
        "These financial statements relate to {}, a {}.",
        eng.entity_name,
        eng.entity_type.label().to_lowercase()
    )];
    if !opt.entity_details.is_empty() {
        v.push(opt.entity_details.join(", ") + ".");
    }
    v
}

pub fn accounting_policies(
    eng: &Engagement,
    opt: &ReportOptions,
    has_inventory: bool,
    has_employees: bool,
) -> Vec<(String, String)> {
    let framework = match eng.entity_type {
        EntityType::Company => "the Accounting Standards notified under section 133 of the Companies Act, 2013 read with the Companies (Accounting Standards) Rules, 2021 and the other relevant provisions of the Act. The financial statements are presented in the format prescribed by Division I of Schedule III to the Companies Act, 2013",
        EntityType::Llp => "the Accounting Standards issued by the Institute of Chartered Accountants of India, to the extent applicable. The financial statements are presented in the format given in the ICAI Guidance Note on Financial Statements of Limited Liability Partnerships",
        _ => "the Accounting Standards issued by the Institute of Chartered Accountants of India, to the extent applicable. The financial statements are presented in the format given in the ICAI Guidance Note on Financial Statements of Non-Corporate Entities",
    };
    let mut p = vec![
        (
            "Basis of preparation".to_string(),
            format!("The financial statements have been prepared under the historical cost convention on the accrual basis of accounting, in accordance with the generally accepted accounting principles in India, including {framework}."),
        ),
        (
            "Use of estimates".to_string(),
            "The preparation of financial statements requires estimates and assumptions that affect the reported amounts of assets, liabilities, income and expenses. Differences between actual results and estimates are recognised in the period in which they are known.".to_string(),
        ),
    ];
    let dep = match eng.far.as_ref().map(|r| r.basis) {
        Some(BookBasis::ScheduleIiSlm) => "Depreciation is provided on the straight-line method over the useful lives prescribed in Schedule II to the Companies Act, 2013, pro rata from the date the asset is put to use.",
        Some(BookBasis::ScheduleIiWdv) => "Depreciation is provided on the written down value method over the useful lives prescribed in Schedule II to the Companies Act, 2013, pro rata from the date the asset is put to use.",
        Some(BookBasis::IncomeTaxRates) => "Depreciation is provided on the written down value method at the rates prescribed under the Income-tax law; assets put to use for less than 180 days in the year are depreciated at half the rate.",
        None => "Depreciation is provided on the written down value method at the rates considered appropriate by the management.",
    };
    p.push(("Property, plant and equipment".into(), format!("Property, plant and equipment are stated at cost of acquisition, including attributable costs of bringing the asset to its working condition, less accumulated depreciation. {dep}")));
    if has_inventory {
        p.push(("Inventories".into(), format!("Inventories are valued at the lower of cost and net realisable value. Cost is determined on the {} basis.", opt.inventory_cost_formula)));
    }
    p.push(("Revenue recognition".into(), "Revenue from sale of goods is recognised when significant risks and rewards of ownership are transferred to the buyer, and revenue from services when the service is rendered, net of goods and services tax.".into()));
    if has_employees {
        p.push(("Employee benefits".into(), "Short-term employee benefits are recognised as an expense in the year in which the related service is rendered. Contributions to provident fund and other defined contribution schemes are charged when due.".into()));
    }
    p.push((
        "Taxes on income".into(),
        // Deferred tax is not computed by LedgerCraft: the policy sentence is
        // printed only when the books carry a deferred tax balance.
        if eng.entity_type.is_company() && has_deferred_tax(eng) {
            "Current tax is the amount of tax payable on the taxable income for the year. Deferred tax is recognised on timing differences, subject to prudence, in accordance with AS 22."
        } else if eng.entity_type.is_company() {
            "Current tax is the amount of tax payable on the taxable income for the year."
        } else {
            "Provision for current tax is made on the basis of the taxable income for the year under the Income-tax law."
        }
        .into(),
    ));
    p.push(("Provisions and contingent liabilities".into(), "A provision is recognised when there is a present obligation as a result of a past event and a reliable estimate can be made of the outflow. Contingent liabilities are disclosed by way of notes.".into()));
    p
}

/// The books carry a deferred tax ledger (LedgerCraft itself computes none).
pub fn has_deferred_tax(eng: &Engagement) -> bool {
    eng.cy
        .ledgers
        .iter()
        .any(|l| l.name.to_ascii_lowercase().contains("deferred tax"))
}
