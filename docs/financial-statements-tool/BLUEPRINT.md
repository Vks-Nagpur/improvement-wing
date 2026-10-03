# LedgerCraft — Product Blueprint

> Product name: **LedgerCraft** · Platform: **Windows desktop app** · Works **fully offline**
> Goal: any CA, accountant or entity can go from **Tally / Zoho / Busy / Excel → print-ready financial statements in the ICAI / legal format in 3–4 minutes**, completely free.
>
> **Intent: solve the problem, not sell anything.** Free, open-source, non-commercial. No paid tier, no ads, no data collection.

### Core principles (non-negotiable)
1. **Law first** — every line item, heading and wording traceable to a source (Companies Act / Schedule III, ICAI Guidance Notes, Income-tax Act 2025, AS).
2. **Data can never silently go wrong** — every number reconciles back to the source TB; the tool refuses to print if it doesn't.
3. **Everything is logged** — append-only audit trail; nothing is ever truly deleted.
4. **Map once, reuse forever** — mapping is remembered per client; next year only new ledgers need attention.
5. **Plain, professional output** — conventional black-and-white statements; no decorative design.
6. **Legal wording in statements, simple explanation on screen** — the printed statements use the exact prescribed terminology; the app explains it in simple words.

---

## 1. Why is this required? (simple explanation)

1. **The law now fixes the format for almost everyone.**
   - Companies already must follow **Schedule III** of the Companies Act, 2013.
   - ICAI has issued Guidance Notes that prescribe formats for **non-corporate entities** (proprietorship, HUF, partnership firm, AOP, BOI, etc.) and a separate one for **LLPs**. As per ICAI's ASB announcement (31-03-2026), applicability is phased: **Phase I** from accounting periods beginning on/after **1 April 2025** for entities with turnover above ₹5 crore, **Phase II** from **1 April 2026** for all covered entities. *(Verify wording on icai.org — see §12.)*
   - So from FY 2026-27, practically **every** set of financials a CA signs must be in a prescribed format with notes.
2. **Tally/Busy/Zoho do not produce these formats.** They give a trial balance and a "vertical balance sheet", not Schedule III / Guidance-Note statements with notes, ageing, ratios, comparatives.
3. **Today it is done manually in Excel** — copying TB, grouping 300+ ledgers, typing notes, fixing totals, re-doing it every year. Slow, error-prone, and not uniform across the office.
4. **Paid tools exist but are limited** (company-only, Excel-based, paid per licence). Small firms and individual practitioners need a **free, all-entity** tool.
5. **Laws keep changing** (Schedule III amendments 2021, new Guidance Notes 2023/2026, Income-tax Act 2025 from 1 April 2026). The tool must **update its formats by itself**, not depend on the user.

**One line:** *Accounting software records the books; this tool converts the books into legally formatted, print-ready financial statements — fast, correct, and free.*

---

## 2. Benchmark: Black Horse Excellence (what exists today)

Source: their product listing / user manual (their website is blocked from our research environment; details below are from search-indexed pages — verify by visiting).

| Aspect | Black Horse "Schedule 3 Automation Tool" |
|---|---|
| Platform | **Excel-based** workbook (formulas + linked sheets) |
| Entities | Private / Public Limited companies — **Schedule III Division I (Non-Ind AS)** |
| Import | Tally ERP 9 / TallyPrime integration; other software via Excel TB |
| Grouping | Auto-grouping claimed **50–70%** of ledgers |
| Features | Auto rounding, depreciation schedule (Companies Act), notes & annexures "single click", headers/page setup, analytical graphs |
| Price | Paid (listed on IndiaMART) |
| Learning | Tutorial videos (YouTube): Basics, Master Sheet, Trial Balance Sheet |

### 2.1 Benchmark: Computax — CompuBal + CompuTax (CompuOffice)

Source: indexed vendor/listing pages (computaxonline.com is blocked from our research environment — verify on the site / brochure).

| Product | Features found |
|---|---|
| **CompuBal** (Balance Sheet & Audit Report) | Financial statements in **Horizontal, Vertical and Schedule III** formats · depreciation chart as per **Schedule II** · depreciation calculator per asset · **auto Cash Flow (indirect method)** · **auto notes as per Schedule III** · MGT-9 · **import from Tally (with PY figures) & Excel** · XML import · tax audit forms **3CD, 29B, 10CCC** with IT-department validations and **error locator** · auto ratios in 3CD · DSC signing |
| **CompuTax** (Income tax) | ITR 1–7 computation & e-filing · Form 3CD tax audit · **auto UDIN generation option for 3CA/3CB/3CD & statutory audit** (vendor claim — mechanism not public) · 26AS view/import · **imports Balance Sheet, P&L, computation from CompuBal** into ITR |

**Lessons we adopt from Computax:**
- **Data flow chain**: TB → Financial statements → Tax computation → ITR schedules (no re-typing). We export BS/P&L/depreciation in a form usable for ITR / tax audit.
- **Error locator** concept → our error engine jumps to the exact cell.
- **Validations mirroring government rules** (we mirror ICAI/Schedule III/IT rules).
- **PY figures pulled at import** → we also support it.

**Where we go beyond (our differentiators):**

| # | Our product |
|---|---|
| 1 | **Free for all**, open-source |
| 2 | **All entity types**: Company (Div I; Div II Ind AS later), LLP, Partnership, Proprietor/Individual, HUF, AOP, BOI, Trust/Society (later) |
| 3 | **Auto-mapping target 90%+** using Tally group + ledger-name dictionary + "learns from your past mapping" |
| 4 | **Self-updating format packs** (law changes pushed as versioned data, not new software) |
| 5 | **Both depreciation engines**: Companies Act Schedule II + Income-tax block WDV, with a single Fixed Asset Register |
| 6 | **Error engine** that blocks printing until critical issues are fixed |
| 7 | **Toggles for what to show and when** — with a plain, professional final output |
| 8 | **One-click roll-forward** to next year |
| 9 | **Tamper-evident audit trail, versions & backups** |
| 10 | **Self-correction suggestions & connection doctor** |

---

## 3. The 3–4 minute user journey

```
[1] Choose entity & year  →  [2] Import (1 click)  →  [3] Auto-map ledgers
        →  [4] Error check (red/amber/green)  →  [5] Fixed assets & depreciation
        →  [6] Toggles (notes, display)  →  [7] Preview & Print/Export
```

| Step | What the user does | What the tool does | Target time |
|---|---|---|---|
| 1 | Select entity (e.g. "Partnership Firm"), FY, units (₹ / ₹ '000 / ₹ lakh) | Loads correct format pack | 10 sec |
| 2 | Click **Import from Tally** (or Zoho / Busy / Excel) | Pulls TB with groups, opening/closing, stock, PY figures | 20–30 sec |
| 3 | Review only the **unmapped** list | Auto-maps by Tally group + name rules + memory | 60–90 sec |
| 4 | Fix red errors | Runs 30+ checks (see §6) | 30–60 sec |
| 5 | Confirm FAR additions/deletions | Calculates Companies Act + IT depreciation | 30 sec |
| 6 | Flip toggles | Shows/hides notes, zero rows, PY column, etc. | 20 sec |
| 7 | Print / PDF / Excel / Word | Final statements + notes | 10 sec |

The thinking and professional judgement (policies, contingent liabilities, qualifications, etc.) stay with the CA. The tool removes the mechanical work.

---

## 4. Entity coverage & legal frameworks

| Entity | Framework (format source) | Key presentation differences |
|---|---|---|
| **Company (non-Ind AS)** | Schedule III **Division I** + ICAI *Guidance Note on Division I – Non Ind AS Schedule III* (revised Jan 2022) | Share capital + promoter shareholding, Reserves & Surplus, current/non-current split, ageing schedules, 11 ratios, Additional Regulatory Information (2021 amendment), Cash Flow (not for small cos / OPC exemption — verify per sec 2(40)) |
| **Company (Ind AS)** | Schedule III **Division II** | SOCIE, OCI — **Phase 3 of roadmap** |
| **NBFC (Ind AS)** | Schedule III **Division III** | Later / optional |
| **LLP** | ICAI *Guidance Note on Financial Statements of LLPs* | Partners' funds (contribution + current a/c), partners' remuneration/interest lines |
| **Partnership Firm** (reg./unreg.) | ICAI *Guidance Note on FS of Non-Corporate Entities* | "Owners' Funds" → Partners' Capital & Current accounts, partner-wise movement note |
| **Proprietorship / Individual (business/profession)** | Same Non-Corporate Guidance Note | Proprietor's capital account movement (drawings, capital introduced, profit) |
| **HUF** | Same Non-Corporate Guidance Note | Karta's / HUF capital account |
| **AOP / BOI** | Same Non-Corporate Guidance Note | Members' capital accounts |
| **Trust / Society** (doing business) | Non-Corporate GN (where business/profession) | Corpus / funds — **later phase** |

Design rule: **one engine, many "format packs"**. Each entity = a pack (JSON/YAML) describing line items, sub-heads, notes, validations, and display rules.

---

## 5. Import module (single-click)

### 5.1 TallyPrime / Tally ERP 9 — **one-click, live**
- **How:** Tally exposes **XML over HTTP** (default port **9000**). Enable via *F1 Help → Settings → Advanced Configuration → enable HTTP server / ODBC*. Tool sends an XML request (e.g. report "Trial Balance", "List of Accounts", "Stock Summary") and gets XML back.
- **What we pull:** company list, ledger masters with **parent group hierarchy**, opening & closing balances, closing stock, PY figures (by changing period), fixed-asset ledgers.
- **Constraint:** browser apps cannot reliably call `localhost:9000` (CORS). → This is a key reason to build a **desktop app** (see §10).
- **Fallback:** Tally "Export → Excel/XML" file upload.

### 5.2 Zoho Books — **API (OAuth 2.0)**
- India domains: auth `accounts.zoho.in`, API `https://www.zohoapis.in/books/v3`.
- Endpoints confirmed in docs: **Chart of Accounts**, Reports (P&L, Balance Sheet, account transactions). A dedicated "trial balance" endpoint was **not confirmed** in our research → build TB from chart-of-accounts balances or use Zoho's TB **Excel export** as fallback. *(Verify in official API docs before build.)*
- Needs: register a free client in **Zoho API Console**; user logs in once and grants read-only scope.

### 5.3 BUSY — **file-based (no public API)**
- BUSY **does not provide an API** (as per indexed vendor info). It supports report export to **Excel / XML / PDF / HTML** (Alt+E on a report).
- Approach: user exports **Trial Balance + List of Accounts (with groups)** to Excel → drag-drop into our tool → auto-detect BUSY layout.

### 5.4 Excel Trial Balance — **our standard template**
Downloadable template, one row per ledger:

| Column | Required | Notes |
|---|---|---|
| Ledger Name | ✔ | |
| Parent Group | ✔ | Helps auto-mapping |
| Opening Dr / Cr | optional | for movement notes |
| Closing Dr / Cr (CY) | ✔ | |
| Closing Dr / Cr (PY) | optional | if blank, use last year's file |
| Mapped Head | optional | lets power users pre-map |
| Partner / Member | optional | for capital account split |

Any other software (Marg, QuickBooks, SAP exports, manual books) can use this template.

### 5.5 Other future connectors
Marg ERP, QuickBooks, Vyapar, Miracle — via Excel template first; native later based on demand.

---

## 6. Error engine ("Shows errors")

Severity: 🔴 **Blocker** (cannot print) · 🟠 **Warning** · 🔵 **Info**

| # | Check | Severity |
|---|---|---|
| 1 | TB Dr ≠ Cr | 🔴 |
| 2 | Unmapped ledger(s) | 🔴 |
| 3 | Balance Sheet does not tally after mapping | 🔴 |
| 4 | Opening CY ≠ Closing PY (per ledger) | 🟠 |
| 5 | Ledger with abnormal balance (e.g. Debtor in Cr, Cash in Cr) → suggest reclass | 🟠 |
| 6 | Negative cash / bank balance (overdraft not reclassified to borrowings) | 🟠 |
| 7 | Suspense ledger has balance | 🟠 |
| 8 | Closing stock missing / differs from stock summary | 🟠 |
| 9 | Depreciation as per FAR ≠ depreciation booked in TB | 🟠 |
| 10 | Fixed asset ledger not in FAR | 🟠 |
| 11 | Partners' capital split missing (firm/LLP) | 🔴 |
| 12 | Trade receivables / payables ageing not provided (where required) | 🟠 |
| 13 | MSME payable disclosure not filled | 🟠 |
| 14 | Note referenced but empty / note has data but hidden | 🟠 |
| 15 | Rounding difference > threshold | 🔵 |
| 16 | Ratio variance > 25% without explanation (Company Div I) | 🟠 |
| 17 | Related-party ledgers detected (name match with partners/directors) but no RP note | 🟠 |
| 18 | PY figures missing (first year) — confirm "first year" toggle | 🔵 |

Each error has a **"Fix" button** that jumps to the exact ledger / note.

---

## 7. Ledger mapping — simplest and fastest

**Three-layer auto-mapping:**
1. **Group rule** — Tally/BUSY parent group → head (e.g. *Sundry Debtors → Trade Receivables*, *Duties & Taxes → Other Current Liabilities / Statutory dues*).
2. **Name dictionary** — keywords: "GST", "TDS", "Salary Payable", "Car Loan", "Audit Fee", "Electricity" → specific sub-heads/notes.
3. **Memory** — every manual mapping is remembered per client (and optionally shared as a community dictionary). Next year → near 100% auto.

**UI for the leftovers:**
- Only unmapped ledgers shown, sorted by amount (biggest first).
- **Type-ahead** head selector; keyboard only (↑ ↓ Enter).
- **Bulk select** + map; **split** a ledger across heads (e.g. a loan's current maturity).
- "Same as last year" one-click.
- Right panel shows live Balance Sheet updating as you map.

**Simple ledger creation** (for adjustments not in books): one line → name, head, amount, Dr/Cr. Adjustment entries listed separately and printable as "Adjustments to books".

---

## 8. Fixed Assets & Depreciation

### 8.1 Fixed Asset Register (FAR) — single source
Per asset: name, block/class, date of purchase, **date put to use**, cost, additions (with dates), deletions (date, sale value), residual value, useful life, method.

### 8.2 Two engines, one register

| | **Companies Act / Books** | **Income-tax** |
|---|---|---|
| Basis | Schedule II useful lives (SLM or WDV), residual value | **Block of assets**, WDV at prescribed rates |
| Mid-year | Pro-rata by days | **Half rate** if put to use < 180 days in the year |
| Law reference | Companies Act, 2013 Schedule II | **Income-tax Act, 2025** (in force from **1 April 2026**, tax year 2026-27 onwards) — depreciation section & rate appendix under the new Rules. *For FY 2025-26 and earlier: sec 32 + Appendix I of IT Rules, 1962.* |
| Output | Note on PPE / Intangibles (gross block, dep, net block, CY & PY) | Block-wise IT depreciation chart (for ITR / tax audit) |

- **"IT Master Selector"**: a dropdown of all IT blocks (5%, 10%, 15%, 30%, 40%, … as per the applicable rate table). The tool auto-suggests a block from the asset class.
- Rates are **data in the format pack**, not code → when CBDT changes rates, only the pack updates.
- Non-corporate entities: books depreciation method chosen by toggle (many follow IT rates in books — tool supports "use IT rates in books" toggle).
- Optional: **Deferred tax** working (timing difference books vs IT) — AS 22, for companies.

> ⚠ Section numbering under the Income-tax Act, 2025 is reported differently by secondary sources (Sec 33 vs 34). We must confirm from the **official Act text on incometaxindia.gov.in** before showing section references in print.

---

## 9. Notes to Accounts, Toggles & Print

### 9.1 Notes
- Auto-generated numerical notes (Share capital / Partners' capital, Borrowings, Trade payables with ageing, PPE, Inventories, Revenue, Other expenses, etc.).
- **Text notes library**: significant accounting policies, contingent liabilities, related parties, MSME, events after BS date — pre-written templates per entity type, editable.
- Auto note numbering; hidden notes renumber automatically.

### 9.2 Toggle switches (examples)

| Toggle | Default |
|---|---|
| Show previous-year column | ON |
| Hide rows with zero in both years | ON |
| Show "Note No." column | ON |
| Cash Flow Statement | Auto by entity/size, override allowed |
| Ratios & ageing schedules | Auto by format pack |
| Rounding unit: ₹ / ₹ '000 / ₹ lakh / ₹ crore, decimals | Auto by turnover rule (Company Div I) |
| Merge small "Other expenses" below X% into "Miscellaneous" | OFF |
| First year of entity (no PY) | Auto |
| Print signatures block (partners/directors, auditor, UDIN, place, date) | ON |
| Draft watermark | ON until errors cleared |
| Letterhead / firm logo | OFF |

### 9.3 Print / export — plain and professional (no decorative design)
- **One standard professional layout**: black & white, conventional serif/sans font, simple rules under headings and totals, right-aligned figures, brackets for negatives. **No themes, colours, shading or graphics** in the final output.
- Only functional options: A4 portrait/landscape, page numbers, "continued…" headers, keep a note on one page, letterhead ON/OFF.
- Header: entity name, address, CIN/LLPIN/PAN, period, "All amounts in ₹ …".
- Signature block per entity type (Partners / Proprietor / Karta / Directors / Designated Partners) + auditor block: firm name, FRN, partner name, M. No., **UDIN**, place, date.
- Exports: **PDF**, **Excel (with formulas & links)**, **Word**, and later **XBRL** (MCA AOC-4) / ITR schedules.

### 9.4 Language policy (legal + simple)
| Where | Language |
|---|---|
| **Printed statements & notes** | Exact headings and terminology prescribed by Schedule III / ICAI Guidance Notes / AS (e.g. "Trade Receivables", "Owners' Funds", "Contingent Liabilities and Commitments"). Policy & disclosure text from a **CA-reviewed template library**, each template tagged with its legal source. No informal words. |
| **On screen (help)** | Simple English / Hindi / Marathi explanation next to each line: *"Trade Receivables = money customers still owe you."* |
| **Optional "Reader's Guide"** | A separate one-page plain-language summary for the client (not part of the financial statements, clearly labelled). |

Every template change goes through the same CA-reviewed format-pack process (§11).

---

## 9A. Data integrity — "toughest coding that prevents data errors"

| # | Safeguard | How |
|---|---|---|
| 1 | **No floating-point money** | All amounts stored as integer **paise** (or fixed decimal). Rounding only at display, using one tested rounding routine; rounding difference posted to a visible line and checked. |
| 2 | **Import reconciliation** | After every import: count of ledgers, total Dr, total Cr, and a checksum are compared with the source. Mismatch = import rejected with reason. |
| 3 | **Invariants enforced in code** | TB Dr = Cr; Assets = Liabilities + Owners' funds; P&L profit = movement in reserves/capital; sum of notes = face figure; CY opening = PY closing. Checked on every change, not only at print. |
| 4 | **Single source of truth** | Each figure exists once; statements, notes, ratios, cash flow are **derived** — never typed twice. |
| 5 | **Typed format packs** | Packs validated against a strict schema before loading; a broken pack can't load. |
| 6 | **Locking** | "Finalise" locks the year (read-only). Changes after that require "Unlock with reason" — logged. |
| 7 | **Automated tests** | Golden test files (real anonymised TBs → expected statements), property-based tests on invariants, regression tests for every bug found. Every release must pass all. |
| 8 | **Crash-safe storage** | SQLite with transactions + write-ahead log; autosave; no half-written files. |

## 9B. Data saving, versions & audit trail

Inspired by the audit-trail principle in the proviso to **Rule 3(1), Companies (Accounts) Rules, 2014** (for FY from 1-4-2023: record audit trail of each transaction, edit log with dates, cannot be disabled). Our tool is a preparation tool, not the books, but we follow the same standard for every adjustment and change made inside it.

- **Append-only event log**: who (user), when (timestamp), what (old value → new value), why (optional reason), source (import / manual / auto-fix).
- **Tamper-evident**: each log entry hash-chained to the previous one; the app verifies the chain on open and warns if broken.
- **Cannot be switched off.** No "delete history" option.
- **Snapshots/versions**: automatic snapshot at import, at finalise, and before roll-forward; "Compare versions" shows what changed in figures.
- **Audit trail report** (printable/exportable) for the auditor's file.
- **Backup**: one-click backup to a single encrypted file; scheduled auto-backup to a folder of the user's choice (e.g. OneDrive/Google Drive folder); restore with integrity check.
- **Software trail**: app version + format-pack version stamped on every export and inside the log, so any printed statement can be traced to the exact rules used.

## 9C. Self-correction & troubleshooting

| Situation | Tool behaviour |
|---|---|
| Debtor ledger with Cr balance, cash in Cr, overdraft in bank | Suggest reclassification (one click, logged as "auto-fix") |
| Opening ≠ last year closing | Shows ledger-wise difference and offers "use last year's closing" or "keep and note" |
| Depreciation booked ≠ FAR depreciation | Shows difference, suggests adjustment entry |
| Ledger renamed in Tally | Re-matches using Tally's internal master identity (GUID/Master ID — to be confirmed in testing), not just the name |
| Tally not responding | **Connection doctor**: checks Tally running → company open → HTTP/ODBC port enabled → port number → firewall; tells exactly which step fails and how to fix |
| Zoho token expired | Prompts re-login, keeps data |
| Excel file in wrong layout | Shows which column is missing / wrong, with a sample |
| Unknown crash | Saves state, creates a **diagnostic bundle** (no client data unless user chooses) for reporting the issue on GitHub |

Every auto-fix is a **suggestion the user confirms** — the tool never changes figures silently.

## 9D. Map once, not every time

- Mapping memory stored **per client** (keyed by ledger identity + name) and carried across years.
- **Global dictionary** (common ledger names → heads) improves auto-mapping for new clients.
- Next year, after import, the user sees **only new / changed ledgers** — typically a handful.
- Group-level rules (e.g. "everything under *Indirect Expenses* → Other expenses") cover new ledgers automatically.
- Optional: import a mapping from another client of same type ("use as template").

## 9E. UDIN — paste only (no integration)

Decision: **UDIN is not integrated** for now.
- A simple **UDIN field** (plus Place and Date of signing) on the "Sign-off" screen.
- Whatever is pasted is printed in the auditor's signature block exactly as entered. If left blank, the line prints as "UDIN: ____________" for manual writing.
- The entry is recorded in the audit trail (who/when), like any other change.
- No portal connection, no automatic generation, no key-figure upload.

## 9F. One-click "Export to Folder" (signing-ready)

User ticks the statements needed and presses **Export**:

```
☑ Balance Sheet          ☑ Statement of Profit & Loss   ☑ Notes to Accounts
☑ Cash Flow Statement    ☑ Partners' Capital Accounts   ☐ Depreciation chart (IT)
☑ Fixed Asset Schedule   ☐ Ratios                        ☐ Audit trail report
Format:  ☑ PDF   ☑ Excel   ☐ Word        Mode: (•) Signing copy  ( ) Draft
```

Files appear immediately in a fixed, predictable folder:
```
<Chosen root>\<Client name>\FY 2026-27\Final-2027-06-15_v3\
    01_Balance_Sheet.pdf
    02_Profit_and_Loss.pdf
    03_Notes_to_Accounts.pdf
    ...
    Financial_Statements_Complete.pdf     <- all selected statements, continuous page numbers
    Financial_Statements.xlsx             <- one sheet per statement, live formulas
    export-manifest.json                  <- file list, SHA-256 hashes, app & format-pack version
```
- **Signing copy**: correct signature blocks on each statement (as required by entity type), place/date/UDIN filled from the Sign-off screen, no "Draft" watermark — **allowed only when there are zero blocker errors**.
- **Draft**: watermark "DRAFT", printable anytime.
- Folder opens automatically after export; never overwrites — each export is a new version folder.
- The export is written to a temporary folder first and moved only when every file is complete (no half-written PDFs).
- The manifest hashes let anyone later prove the PDF was not altered after export.

## 9G. Compliance matrix (what laws/standards the engine follows)

| Area | Source | Status in product |
|---|---|---|
| Company formats | Companies Act 2013, **Schedule III Div I** (amended 24-03-2021) + ICAI GN on Div I (Jan 2022) | Phase 1 |
| Company Ind AS formats | Schedule III Div II / III | Phase 4 |
| Non-corporate formats | ICAI GN on FS of Non-Corporate Entities (Aug 2023), phased applicability (§1) | Phase 1 |
| LLP formats | ICAI GN on FS of LLPs | Phase 2 |
| Accounting Standards | ICAI AS (AS 2 inventories, AS 10 PPE, AS 22 deferred tax, etc.) for policy text & disclosures | Template library |
| Book depreciation | Companies Act Schedule II | Phase 2 |
| Tax depreciation | **Income-tax Act, 2025** + Income-tax Rules 2026 (from tax year 2026-27); IT Act 1961 + Rules 1962 for earlier years | Phase 2 |
| Audit trail principle | Rule 3(1) proviso, Companies (Accounts) Rules 2014 | Built-in (§9B) |
| UDIN | Paste-only field, printed as entered | §9E |
| XBRL | MCA AOC-4 XBRL taxonomy & validation tool | Phase 4 |

Each item carries its **official citation** inside the format pack; when the source changes, the pack is revised with an effective date.

## 10. Roll forward (CY → PY)

One button **"Start next year"**:
1. Current year figures move to the **PY column**.
2. Balance Sheet closing becomes opening; P&L reset.
3. FAR closing WDV / net block becomes opening; deletions dropped.
4. All mappings, notes text, toggles, signatories carried forward.
5. Import new year's TB → only new ledgers need mapping.
6. If format pack changed (new law), show **"What changed this year"** banner.

---

## 11. Self-updating law ("format packs")

```
format-packs/
  company-div1/        v2022.1 (GN Jan 2022)  effective_from: 2021-04-01
  non-corporate/       v2026.1                effective_from: 2025-04-01 (Phase I) / 2026-04-01 (Phase II)
  llp/                 v2026.1
  it-depreciation/     ita1961-appendix1, ita2025-rates (effective 2026-04-01)
  companies-sch2/      useful lives
```
- Each pack = JSON/YAML: line items, mapping hints, notes, validations, display rules, **effective date range**, source citation (notification / ICAI GN).
- Packs live in a **public GitHub repo**; the app checks for updates on start (works offline with last-downloaded pack).
- Changes reviewed by a **CA panel** (pull requests) — community-maintained, transparent changelog.
- The app picks the right pack automatically by **entity + financial year**.

---

## 12. External connections required

| # | Connection | Type | Cost | Purpose |
|---|---|---|---|---|
| 1 | **TallyPrime / ERP 9** | Local XML over HTTP (port 9000) | Free | One-click TB, groups, stock |
| 2 | **Zoho Books API** | OAuth 2.0 REST (India DC) | Free API client (Zoho API Console) | Chart of accounts, reports |
| 3 | **BUSY** | Excel/XML file export | Free | TB + groups |
| 4 | **Excel template** | File upload | Free | Any other software |
| 5 | **Format-pack updates** | GitHub (public repo / releases) | Free | Law updates |
| 6 | **App updates** | GitHub Releases | Free | New versions |
| 7 | MCA XBRL taxonomy & validation tool | Download from MCA | Free | Future XBRL export |
| 8 | ICAI UDIN | **Not integrated** — paste field only | — | Printed on report |

No paid service is required → this is what keeps the product **free**.

---

## 13. Recommended technology

| Choice | Reason |
|---|---|
| **Desktop app (Windows first)** using **Tauri v2** | Can talk to Tally on localhost; works offline; **client data never leaves the PC** (confidentiality); **no server cost → free forever**; small installer; built-in signed auto-updater |
| **Calculation engine in Rust** (`rust_decimal`, no floats) | Compiler catches whole classes of bugs (nulls, type mix-ups, unhandled cases); the same engine is used for screen, PDF and Excel so they can never disagree |
| UI: React + TypeScript (strict mode) | Fast, toggle-heavy UI; UI only displays — it never calculates |
| Local DB: SQLite (one file per client/year) | Easy backup, portable |
| Engine: rules from format packs (JSON) | Law changes without code changes |
| Print: HTML → PDF (built-in) + Excel/Word export libraries | Plain, consistent layout on every PC |
| Source code: open-source on GitHub | Trust, community contributions |

Later: optional web version (with file upload only) for Zoho/Excel users.

---

## 14. Roadmap

| Phase | Scope | Outcome |
|---|---|---|
| **0 — Knowledge** (2–3 wks) | Collect official texts: Schedule III + GN Div I, ICAI GN Non-Corporate, ICAI GN LLP, Schedule II, IT depreciation tables (1961 & 2025); sample TBs from 10 real clients; mapping dictionary | Format packs v1 drafted |
| **1 — MVP** | Excel + Tally import, mapping, error engine, **Non-Corporate (Firm, Proprietor, HUF, AOP/BOI)** + **Company Div I**, basic notes, PDF print | Usable by our office |
| **2** | LLP pack, FAR + both depreciation engines, toggles, export-to-folder, roll-forward, Excel/Word export | Public free release |
| **3** | Zoho API, BUSY auto-detect, Cash Flow auto, ratios & ageing, deferred tax | Feature parity+ with paid tools |
| **4** | Ind AS (Div II), XBRL export, ITR schedule export, community mapping dictionary | Advanced |

---

## 15. Keeping it free (non-commercial by design)

- Purpose is public good, not revenue: **no paid tier, no ads, no telemetry of client data**.
- **Licence decision: no licence** (owner's choice). Effect: by default copyright law keeps all rights with the owner — others may download and **use** the released app free, but may not copy, modify or redistribute the source code without permission. A short "Free to use" notice ships with the app.
- Note: free code-signing programmes for open-source (e.g. SignPath Foundation) are meant for open-source-licensed projects, so LedgerCraft will likely **not qualify** (to be confirmed). Options: (a) ship unsigned — Windows shows a SmartScreen warning ("More info → Run anyway") until the app builds reputation; (b) buy a code-signing certificate later.
- Zero hosting cost (desktop + GitHub) → nothing to fund.
- Volunteer CA reviewers maintain format packs.
- Disclaimer: tool assists preparation; responsibility for statements remains with the preparer/auditor.

---

## 15A. Engineering plan — how a project this big is built without errors

No software is bug-free by promise; it becomes reliable by **design + tests + process**. The plan:

### Architecture (layers that cannot mix)
```
 Importers (Tally / Zoho / BUSY / Excel)
      │  every importer outputs the SAME canonical Trial Balance (+ reconciliation totals)
      ▼
 Core engine (Rust, pure functions, no UI, no files)
      │  TB + mapping + format pack + FAR  →  statements, notes, depreciation, checks
      ▼
 Storage (SQLite, transactions, append-only audit log)
      ▼
 Output (PDF / Excel / Word renderers — read-only consumers of engine results)
      ▼
 UI (React) — shows results, sends user commands; never computes figures
```
- **One canonical data model.** Importers only translate; all logic sits in one tested core. A bug fixed once is fixed for every source.
- **Pure core.** Same input → same output, always. Makes it fully testable and reproducible (re-running last year's file gives byte-identical figures).
- **Format packs are data**, validated against a schema. A law change is a reviewed data change, not new code.

### Error-prevention layers
| Layer | What it catches |
|---|---|
| Strong types (Rust/TypeScript strict) | Paise vs rupees mix-ups, missing fields, wrong sign handling |
| Invariant checks at runtime | Any statement that does not balance / reconcile → blocked, never printed |
| Unit tests | Each rule (e.g. 180-day half rate, Schedule II pro-rata) |
| **Golden tests** | 50+ real anonymised TBs across all entity types → approved expected statements; any change in output fails the build |
| Property-based tests | Thousands of random TBs: totals must always reconcile |
| Importer fixtures | Saved real Tally/BUSY/Zoho/Excel exports, incl. odd cases (negative stock, blank groups, Unicode names) |
| CA review tests | Each format pack signed off by 2 CAs against the Guidance Note before merge |
| Continuous Integration (GitHub Actions) | All tests run on every change; nothing merges unless all pass |
| Beta channel | New versions go to volunteer firms first, then to everyone |

### Process
1. Write the rule → write its test → write the code (test-first for all calculations).
2. Every bug report gets a test that reproduces it before the fix.
3. Small, reviewed changes; no direct edits to the main branch.
4. Semantic versioning; every release has a public changelog; rollback possible.

---

## 15B. How users get and use the tool

**Free desktop app, downloaded from GitHub — works offline. Not a website.**

| Step | What happens |
|---|---|
| 1. Download | Project page on GitHub → **Releases** → `LedgerCraft-Setup.exe` (Windows). Later also a simple website page linking to the same file. |
| 2. Install | Per-user install, no admin rights needed. Installer signing: see §15 (no-licence choice affects free signing). |
| 3. First run | Choose data folder (e.g. `D:\LedgerCraft Data`) and export folder; enter firm details once (name, FRN, partners, M. No.). |
| 4. Daily use | Open client → Import (Tally one-click / file) → fix the few flagged items → Sign-off screen (place, date, UDIN paste) → **Export to Folder** → print & sign. |
| 5. Updates | App checks GitHub on start: **signed app updates** (Tauri updater verifies the signature; cannot be disabled) and **format-pack updates** (law changes). User clicks "Update". Works offline with last version. |
| 6. Help | Built-in help + short tutorial videos; issues/suggestions via GitHub "Issues" or a simple feedback form. |

- **Client data stays on the user's PC** (and their own backup folder). Nothing is uploaded.
- Multiple staff: data folder can be on a shared office drive (one user editing a client at a time — file lock).
- Later (optional): a browser version for Excel/Zoho-only users, running entirely inside the browser (no server storage).


---

## 15C. Modular design — separate modules, linked when used together

Each module works **alone** (input: Excel/Tally) or **together** (shares the same client data). Nothing is entered twice.

| # | Module | Works alone? | What it does |
|---|---|---|---|
| 1 | **Core & Data Vault** | (base) | Client files, audit trail, versions, backup |
| 2 | **Import Hub** | ✔ | Tally / Zoho / BUSY / Excel — TB **and full vouchers** |
| 3 | **Mapping** | ✔ | Map once, remember forever |
| 4 | **Statement Builder** | ✔ | Balance Sheet, P&L, Cash Flow, Notes as per the applicable format |
| 5 | **Fixed Assets & Depreciation** | ✔ | FAR, Companies Act + Income-tax depreciation |
| 6 | **Analysis** | ✔ | Ratios, trends, YoY variance, month-wise charts, ratio-variance explanations |
| 7 | **Audit Assist** | ✔ | Voucher-level checks + "Auditor Reference Workbook" (§15E) |
| 8 | **Tax Bridge** | ✔ | Disallowance summary, IT depreciation, figures needed for computation/ITR |
| 9 | **Export & Sign-off** | — | Folder export, PDF/Excel/Word, UDIN paste |
| 10 | **Law Library** | (base) | Versioned format packs & rule packs (law as data) |
| 11 | **Local AI Assistant** | optional | Explanations & suggestions, fully offline (§15F) |

A user can open only "Audit Assist" for an audit client, or only "Statement Builder" for a quick balance sheet.

---

## 15D. Very large data — thousands to millions of vouchers

- **Streaming import**: Tally Day Book pulled month-by-month (small XML chunks) → no memory crash, progress bar, **resumable** if interrupted.
- Storage: SQLite with indexes on date, ledger, voucher type, amount → checks run in seconds even on lakhs of vouchers.
- **Voucher ↔ TB reconciliation**: sum of all vouchers per ledger + opening must equal the TB closing; any difference shown ledger-wise before anything else runs.
- Performance target to be proven by tests: **1,000,000 vouchers** imported and all checks run on a normal office PC.
- "Training on huge data" in practice means: (a) a large **test library** of real anonymised books on which every rule is proved, and (b) the **mapping memory** that learns from every CA's confirmed mapping. The rules themselves are exact law, not guesses — they don't need AI training.

---

## 15E. Audit Assist — checks for the auditor (separate Excel workbook)

Output: **"Auditor Reference Workbook.xlsx"** — summary sheet + one sheet per check, each row = voucher (date, voucher no., party, amount, reason flagged). **Flags only; the auditor decides.**

### A. Cash transaction & disallowance checks
| Check | Limit (unchanged in substance) | Section reference |
|---|---|---|
| Cash **payment** for expense to a person in a day | > ₹10,000 (₹35,000 for transporters) | Old 40A(3) / new Act — *mapped in rule pack* |
| Cash **receipt** from a person (day / transaction / event) | ≥ ₹2,00,000 | Old 269ST / new Act |
| Loan/deposit/specified sum **accepted** in cash | ≥ ₹20,000 | Old 269SS / new Act |
| Loan/deposit **repaid** in cash | ≥ ₹20,000 | Old 269T / new Act |
| Cash share of total receipts & payments (tax-audit threshold test) | ≤ 5% test | Old 44AB / new Act |
| Day-wise **negative cash balance** | any day | Books quality |

> New-Act section numbers for these are reported differently by secondary sources (e.g. old 269T → "220" vs "188"). They are stored **as data** and will be filled only from the **official Act text** before release. Old-Act references are used for FY up to 2025-26.

### B. Other tax-audit helper checks
- Expenses of TDS nature with no TDS deducted / deposited late (old 40(a)(ia)).
- Payments to MSME suppliers beyond allowed days (old 43B(h)) — uses MSME tag on party.
- Statutory dues (GST, PF, ESI, TDS) unpaid at year-end / paid late; employee PF/ESI paid after due date (old 36(1)(va)).
- Payments to related parties / partners / directors (old 40A(2)(b)) — list for review.
- Capital vs revenue: large repairs, items above a threshold in expense ledgers.
- GST blocked-credit indicators (motor vehicle, food, gifts) where ITC taken.

### C. Data-quality / fraud-risk checks (audit analytics)
- Duplicate vouchers (same party, amount, date), voucher-number gaps.
- Entries on Sundays / holidays, posted after year-end but dated within the year (from audit-trail data where available).
- Round-sum entries, unusual journals to cash/bank/revenue, large JVs near year-end.
- Benford's-law first-digit test on expenses.
- Ledgers with both large Dr and Cr flows (possible accommodation entries).

All thresholds and section references live in the **rule pack** (changeable when law changes), never hard-coded.

---

## 15F. Local AI (optional, free, offline)

**LedgerCraft works 100% without AI.** AI is an optional add-on for convenience.

- Uses **Ollama** (free, open-source, runs on the user's own Windows PC) with a free open model. LedgerCraft detects it automatically; one-click setup guide.
- **No internet, no paid API, no client data leaves the PC.**
- What AI may do (suggestions only):
  - Suggest mapping for a new, unusually-named ledger.
  - Explain any line / note / flag in simple language (English / Hindi / Marathi).
  - Draft variance explanations for ratios (>25% change) for CA review.
  - Search the Law Library in plain words ("what is required for partners' remuneration note?").
- What AI may **never** do: calculate or change any figure, mark a check as cleared, or print anything without the user's confirmation. Every AI suggestion is labelled "AI suggestion" and logged in the audit trail.
- Hardware: AI add-on needs a reasonably modern PC (around 16 GB RAM recommended); without it, LedgerCraft runs normally.

---

## 15G. Two user modes — Simple and Expert

| | **Simple mode** (business owner / accountant) | **Expert mode** (CA / auditor) |
|---|---|---|
| Screens | Step-by-step wizard, plain words | Full control, all toggles |
| Legal references | Hidden — the tool just follows the law | Shown: Guidance Note para / Schedule III / section, with source |
| Errors | "Fix this" with one suggested action | Full detail + override with reason (logged) |
| Audit Assist | Hidden | Full |
| Output | Same legally correct statements | Same + auditor workbook + audit trail report |

**No repetition, no unnecessary data:** only applicable lines and notes are printed; zero/not-applicable items are hidden automatically (Expert can force-show).

---

## 16. Decisions needed from you

Decided: **Windows desktop** ✔ · **No licence** ✔ · Name **LedgerCraft** ✔

Still open:
1. First entities for MVP: **Non-corporate + Company Div I** (recommended).
2. Sample data: 5–10 anonymised real books (Tally backup or exports — TB **and** day book) to build the test library.
3. Code signing: ship unsigned initially, or buy a certificate?

---

## 17. Verification status of facts used

| Fact | Status | Source |
|---|---|---|
| Non-Corporate & LLP GN phased applicability (Phase I 1-4-2025 >₹5 cr; Phase II 1-4-2026 all) | Reported from ICAI ASB announcement 31-03-2026; **official page blocked from our environment — please open and confirm** | icai.org/post/asb-announ-310326; TaxScan; CAclubindia |
| Non-Corporate GN issued Aug 2023; LLP has separate GN | Multiple secondary sources agree | Taxmann, ClearTax |
| GN on Div I (Non-Ind AS) Schedule III revised Jan 2022; Schedule III amended by MCA notification 24-03-2021 | ICAI resource listing | resource.cdn.icai.org (gnd1.pdf) |
| Income-tax Act, 2025 in force 1-4-2026 | Multiple sources | Wikipedia; TaxGuru |
| New-Act depreciation section number (33 vs 34) | **Conflicting — verify** | TaxTMI, TaxGuru |
| Tally XML over HTTP, default port 9000 | Official Tally help | help.tallysolutions.com/xml-integration |
| Zoho Books API v3, OAuth, India domain `zohoapis.in` | Official Zoho docs | zoho.com/books/api/v3 |
| BUSY has no public API; Excel/XML export | Vendor/secondary sources | busy.in FAQs |
| Black Horse tool features | Indexed product pages (site blocked) | IndiaMART listing, Scribd manual |
| CompuBal / CompuTax features | Indexed vendor pages (site blocked — verify) | computaxonline.com, computax.in, IndiaMART, SoftwareSuggest |
| UDIN: portal generation, key figures, bulk facility; API verification used by authorities | ICAI UDIN FAQs & portal manual | udin.icai.org |
| No public UDIN-generation API for third-party software | **Not found** in our search — treat as unavailable until ICAI states otherwise | — |
| Audit trail: Rule 3(1) proviso, FY from 1-4-2023 | ICAI CA Journal + multiple sources | cajournal.icai.org |
| SignPath Foundation: free code signing for open-source (publisher shown as SignPath Foundation) | Multiple project references; apply & confirm eligibility | signpath.org |
| Cash limits ₹10,000 / ₹2 lakh / ₹20,000 unchanged under IT Act 2025 | Secondary sources | ClearTax, TaxGarden |
| New-Act section numbers for 269SS/269T/269ST/40A(3) | **Conflicting — confirm from official Act text** | incorpx, taxgarden |
| Ollama runs locally on Windows, free | Project docs / earlier research | ollama.com |
| Tauri v2 updater: signed updates mandatory, static `latest.json` on GitHub Releases, NSIS/MSI | Official Tauri docs | v2.tauri.app/plugin/updater |

---

## 18. Build status (engine v0.1)

Working code is in [`ledgercraft/`](../../ledgercraft/README.md):
- Core engine (Rust, exact paise arithmetic): 32 checks covering trial balance, opening balances, wrong grouping, presentation reclassification, voucher integrity, cash limits and the loan register (principal vs interest).
- Statements for Company (Schedule III Div I) and Non-Corporate entities (draft packs), with notes, comparatives and plain print layout.
- Excel/CSV import, versioned export folder (Excel statements, print-ready HTML, Auditor Reference Workbook, manifest with SHA-256).
- Practice-book generator with planted mistakes and known answers. Tests prove every planted mistake is found, no false alarms on 60 random clean books, identical results after the Excel/CSV round trip, and ~1 million vouchers checked in about 5 seconds.
- Automated test run on every change (GitHub Actions, Linux + Windows).

## 19. Build status (v0.2 – design architecture and pending modules)

Done and tested (see `ledgercraft/README.md` and `ledgercraft/DESIGN.md`):
- **Output design architecture**: content → neutral report model → renderers. Real PDF (embedded Typst engine, offline) with boxed or ruled tables, cover, contents with page numbers, running headers, page x of y, landscape schedules, signature blocks; Excel and HTML preview with the same rules.
- **Exact casting** in ₹ / hundreds / thousands / lakhs / millions / crores (largest-remainder rounding); Schedule III rounding-unit rule checked.
- **Notes with Schedule III sub-classification**, MSME split, ageing schedules (FIFO from the day book), partner-wise capital movement, Cash Flow Statement (AS 3, self-reconciling), 11 ratios with variance reasons, LLP format pack.
- **Fixed asset register and depreciation**: Schedule II (SLM/WDV), Income-tax block method with 180-day rule and STCG, Income-tax rates in books; PPE schedule; tie-out checks.
- **Importers**: Tally one-click (XML over HTTP, month-by-month day book), Zoho Books (export + API chart of accounts), BUSY (export + account master).
- **Local AI** (Ollama): explain findings (English/Hindi/Marathi), guarded mapping suggestions, ratio-reason drafts, Q&A; logged; never changes figures.
- **Windows desktop app**: `LedgerCraft.exe` (local server bound to 127.0.0.1 with session token), clients/years storage, map-once memory, tags, hash-chained tamper-evident audit trail; built by GitHub Actions and verified under Wine.
- New-Act references: ss.185 / 186 / 188 (for 269SS / 269ST / 269T) and s.33 (depreciation).

Open items: verify format packs line-by-line against official texts; new-Act equivalent of s.40A(3); Word (.docx) export; real-world testing on client books; code signing for the installer.
