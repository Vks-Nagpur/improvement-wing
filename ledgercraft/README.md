# LedgerCraft – engine (first working version)

LedgerCraft turns a trial balance (and, optionally, the day book) into
financial statements in the prescribed format, and checks the books the way
an experienced CA would – **offline, without AI**.

This folder is the **engine**: the part that reads books, checks them and
produces the statements. The Windows desktop screens will sit on top of it.

## What it checks today

| Area | Checks |
|---|---|
| Trial balance | Debit ≠ credit; difference in opening balances; group not recognised; suspense balance; cash in hand in credit; unusual balance side; miscellaneous expenditure carried as asset |
| Opening balances | Opening ≠ last year's closing (ledger-wise); income/expense ledger with opening; Profit & Loss A/c opening = last year's closing + last year's profit; ledgers renamed / dropped |
| Wrong grouping | Loan grouped as creditor/debtor; OD/CC grouped as bank; GST/TDS grouped as creditor; asset purchase booked as expense |
| Presentation | Customer credit balances → liabilities; supplier debit balances → advances; overdrawn bank → borrowings (no netting) |
| Vouchers | Unbalanced voucher; ledger missing from TB; vouchers don't add up to TB; dated outside the year; possible duplicates; cash negative on any day |
| Cash limits (tax audit help) | Cash payment above ₹10,000 per person per day (₹35,000 transporters); asset bought in cash above ₹10,000; cash receipt ₹2 lakh or more |
| Loans (tax audit help) | Loan register (Form 3CD cl. 31 helper). **Only principal counts as "loan accepted"** – interest credited and TDS are separated. Acceptance test uses unpaid principal; repayment test uses balance with interest. Loans from banks are listed but not tested. |

Every check only **flags** – nothing is changed silently. Legal references switch
automatically: 1961 Act for FY up to 2025-26, Income-tax Act 2025 for FY 2026-27
onwards (new section numbers left "to be confirmed" until verified from the official text).

## Output folder (one click)

```
<Entity>\FY 2025-26\Draft-2026-10-02_v1\
   Financial_Statements.xlsx          Balance Sheet, P&L, Notes, Mapping
   Financial_Statements_print.html    plain A4 layout – open, Print → Save as PDF
   Auditor_Reference_Workbook.xlsx    Summary, all findings, cash limits, loans, opening, grouping, data quality, loan register
   analysis.json                      everything, for other tools
   export-manifest.json               file fingerprints (SHA-256), versions
```
A signing copy (`--final`) is refused while any "Must fix" item is open. Exports never overwrite earlier ones.

## How it was proved

* A generator creates practice books – from a small firm to a company to **~1 million vouchers** – and plants mistakes with known answers.
* Tests require the engine to find **every** planted mistake and raise **zero** false alarms on clean books (60 random clean books per run).
* Books are written to Excel/CSV and read back: answers must be identical to the paisa.
* A deliberately broken rule (interest counted as principal) makes the tests fail – proving the tests have teeth.
* Measured: ~988,000 vouchers checked in about 5 seconds on a normal machine.

## Try it

```
cargo build --release
target/release/ledgercraft practice-data --out practice
target/release/ledgercraft analyse --name "Glitchy Traders" --entity firm --fy 2025-26 \
    --tb practice/firm_with_glitches/trial_balance.xlsx \
    --py-tb practice/firm_with_glitches/previous_year_trial_balance.xlsx \
    --vouchers practice/firm_with_glitches/vouchers.csv --out exports --expert
```

Input template: sheet **Trial Balance** with columns `Ledger, Group, Opening, Closing`
(Dr positive / Cr negative, or `Opening Dr/Cr`, `Closing Dr/Cr`, or values like `1,234.00 Cr`),
optional `Closing Stock`, `Tags` (e.g. `transporter`); optional sheet **Groups** (`Group, Parent`)
for your own sub-groups. Day book: one row per line – `Date, Voucher Type, Voucher No, Ledger, Debit, Credit`.

## Layout

| Crate | Role |
|---|---|
| `lc-core` | Pure engine: money (exact paise), groups, mapping, checks, statements. Laws and formats are data files in `lc-core/packs/`. |
| `lc-io` | Read Excel/CSV, write Excel/HTML/JSON, versioned export folder |
| `lc-testdata` | Practice-book generator with planted mistakes + ground-truth tests |
| `lc-cli` | `ledgercraft` command (until the desktop app is built) |

## Not yet done (next steps)

Direct Tally / Zoho import; fixed asset register and depreciation (Companies Act + Income-tax);
LLP-specific format pack; cash flow statement; ageing, ratios, MSME; rounding units; PDF engine
(the HTML file prints to PDF today); Windows desktop screens; local AI (optional).
Format packs are marked **draft** until each line is verified against the ICAI Guidance Note / Schedule III text.
