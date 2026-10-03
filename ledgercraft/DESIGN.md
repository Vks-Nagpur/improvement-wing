# LedgerCraft – Design Architecture of the Final Output

This document explains how LedgerCraft turns books into printed financial
statements, and the design rules every printed page follows. The rules are
implemented in code; nothing here is aspirational.

## 1. Three layers, strictly separated

```
Books (Tally / Zoho / BUSY / Excel)
   │
   ▼
1. CONTENT  – lc-core: checks, mapping, figures, notes, wording
   │          (YearFacts → exact rounding → Report)
   ▼
2. DOCUMENT – lc-core::report::Report  (neutral: sections, blocks, tables, rows)
   │
   ▼
3. DESIGN   – lc-io::render: PDF (Typst template), Excel, HTML preview
```

* **Content decides what is said.** Line items come from the format packs
  (`packs/company_div1.json`, `non_corporate.json`, `llp.json`), so a change in
  Schedule III or an ICAI Guidance Note is a data change.
* **The document model is neutral.** It knows a row is a "Total" but not what
  a total looks like.
* **Design decides how it looks.** All visual rules live in one place per
  renderer: `templates/report.typ` (PDF), `render/xlsx.rs`, `render/html.rs`.
  The renderers never compute; every figure arrives formatted and cast.

## 2. Exact casting in any unit

Statements can be printed in ₹, hundreds, thousands, lakhs, millions or crores
with 0 or 2 decimals. Naive rounding makes totals disagree with the lines
above them. LedgerCraft prevents this:

1. Every ledger line is a "leaf". In trial-balance sign, all leaves add to zero.
2. Leaves are rounded to the chosen step with the **largest-remainder method**,
   keeping their total exactly zero (`rounding.rs`).
3. Every face figure, subtotal, total, note and schedule is then the plain sum
   of rounded leaves.

As a result the Balance Sheet tallies after rounding, every note total equals
its face line, the PPE schedule ties to the face, and the Cash Flow Statement
ends exactly at the Balance Sheet cash figure. Tests prove this for 33 sets of
books in 6 units (`lc-testdata/tests/report.rs`).

Schedule III rounding rule (Division I): a company with turnover below
₹100 crore may use hundreds, thousands, lakhs or millions; at ₹100 crore or
more, lakhs, millions or crores. A wrong choice raises a warning.

## 3. Page design (PDF)

| Element | Rule |
|---|---|
| Paper | A4 portrait; wide schedules (PPE, ratios, tax depreciation) on landscape pages with their note title |
| Margins | 22 mm top, 20 mm bottom, 20 mm left (binding), 16 mm right |
| Type | Liberation Serif (Times-metric, SIL OFL), with DejaVu Serif for the ₹ sign; body 10 pt, dense schedules 8.5 pt |
| Title block | Entity name 13 pt bold capitals, details line 8.5 pt, statement title 11.5 pt bold, unit note right-aligned italic |
| Running header | From the second page of a section: entity at left, "Section (continued)" at right, hairline rule |
| Footer | "Page x of y" centred; "Draft – for discussion only" at left on drafts |
| Watermark | Large, very light "DRAFT" behind drafts only |
| Cover and contents | Optional cover page; contents with real page numbers |
| Signatures | Two columns: auditor (firm, FRN, partner, M. No., UDIN) and entity (signatories with DIN/DPIN); place and date; never split across pages; never left alone on a page |

### Tables: two professional layouts

| | Boxed (default) | Ruled |
|---|---|---|
| Frame | Full outer box | Rule above and below the table |
| Columns | Vertical rules between all columns | None |
| Rows | Light horizontal rules | None |
| Header | Shaded, bold, firm rule below | Bold, hairline below |
| Subtotal | Rule above | Rule above |
| Total | Firm rules above and below | Firm rule above, double underline under figures |

Common to both: header rows repeat when a table runs to the next page; small
notes are kept on one page; figures right-aligned with Indian digit grouping
(12,34,567.89), negatives in brackets, nil as "-"; indents of 4 mm per level;
multi-level headers with merged cells (e.g. Gross block / Depreciation / Net block).

## 4. Statement structure

* **Balance Sheet / Statement of Profit and Loss** – in the prescribed format
  for the entity (Schedule III Div I, ICAI Non-Corporate GN, ICAI LLP GN).
  Nil lines hidden (optional); remaining items re-lettered (a), (b), (c) and
  (i), (ii) automatically; main P&L items keep fixed Roman numbering.
* **Cash Flow Statement** – AS 3 indirect method; every balance-sheet head is
  classified exactly once, so it always reconciles to cash and bank.
* **Notes** – numbered in order of first reference:
  1 Entity information, 2 Accounting policies (standard wording to review),
  then one note per face line with Schedule III sub-classification:
  borrowings (secured / unsecured / repayable on demand), trade payables
  (MSME / others) with ageing, trade receivables (good / doubtful) with ageing,
  cash on hand / balances with banks, inventories, PPE schedule from the fixed
  asset register, changes in inventories, owners' capital movement
  (partner-wise: opening, introduced, remuneration and interest, share of
  profit, withdrawals, closing) with comparatives, and the 11 ratios with
  variance reasons.
* **Annexure** (not part of the statements): depreciation as per the
  Income-tax Act, block-wise.

## 5. Excel and on-screen preview

The Excel file follows the same rules: one sheet per statement, Times New
Roman, boxed or ruled borders, shaded headers, merged multi-level headers,
indents, figures as real numbers in the chosen unit, A4 print setup (fit to
width, landscape for wide sheets, page numbers in the footer). The HTML
preview in the app uses the same tokens and both layouts.

## 6. Where to change things

| To change | Edit |
|---|---|
| A line item or its wording | `crates/lc-core/packs/*.json` |
| Limits, legal references | `crates/lc-core/packs/rules.json` |
| Useful lives, block rates | `crates/lc-core/packs/depreciation.json` |
| Note content and sub-classification | `crates/lc-core/src/report/build.rs` |
| Look of the PDF | `crates/lc-io/templates/report.typ` (tokens at the top) |
| Look of Excel / preview | `crates/lc-io/src/render/xlsx.rs`, `html.rs` |

## 7. Still to verify against official texts

Format packs are marked *draft* until each line is compared with the official
Schedule III text and the ICAI Guidance Notes (the official sites could not be
opened from the build environment). Income-tax Act 2025 references for
ss.185, 186, 188 (cash loans and receipts) and s.33 (depreciation) come from
incometaxindia.gov.in section listings and secondary sources. s.36 is used as
the counterpart of s.40A(3) on the strength of published section listings;
its sub-section, and the counterpart of the s.43(1) cash proviso, still have to
be read in the statute. Form 26 (Rule 47, Income-tax Rules 2026, notified by
G.S.R. 198(E) of 20 March 2026 as reported) replaces Forms 3CA/3CB/3CD from
Tax Year 2026-27; its clause numbers are not yet mapped because the official
form could not be opened from the build environment.
