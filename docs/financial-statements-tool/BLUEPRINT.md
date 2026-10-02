# Free Financial Statements Preparation Tool — Product Blueprint

> Working name: **"FinStat Free"** (placeholder — rename anytime)
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
| 9 | **Tamper-evident audit trail, versions & backups** |
| 10 | **Self-correction suggestions & connection doctor** |
| 8 | **One-click roll-forward** to next year |

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

## 9. Notes to Accounts, Toggles & Print Designer

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

## 9E. UDIN workflow

Facts (ICAI UDIN FAQs / portal manual): UDIN is generated on ICAI's UDIN portal (udin.icai.org) by the member; key financial figures (e.g. turnover, net profit, net worth, total assets) are entered at generation; bulk generation exists on the portal; ICAI's API verification is used by authorities (e.g. income-tax e-filing portal validates UDIN). **No public API for third-party software to generate UDIN was found** — so the tool will not generate UDIN itself.

Our workflow:
1. On finalise, the tool shows a **"UDIN key figures" panel** (turnover, net profit/loss, net worth/owners' funds, total assets, etc.) taken from the final statements — copy with one click.
2. Button opens the official UDIN portal in the browser.
3. User pastes the generated UDIN back → stored, validated for format, printed in the audit report/signature block, and logged in the audit trail with date.
4. If figures change after UDIN entry → **warning**: "Figures changed after UDIN was generated — UDIN may need revocation/regeneration."

## 9F. Compliance matrix (what laws/standards the engine follows)

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
| UDIN | ICAI UDIN guidelines | §9E |
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
| 8 | (Optional) ICAI UDIN | Manual entry only | — | Printed on report |

No paid service is required → this is what keeps the product **free**.

---

## 13. Recommended technology

| Choice | Reason |
|---|---|
| **Desktop app (Windows first)** using **Tauri** (or Electron) | Can talk to Tally on localhost; works offline; **client data never leaves the PC** (confidentiality); **no server cost → free forever** |
| UI: React + TypeScript | Fast, toggle-heavy UI |
| Local DB: SQLite (one file per client/year) | Easy backup, portable |
| Engine: rules from format packs (JSON) | Law changes without code changes |
| Print: HTML → PDF (built-in) + Excel/Word export libraries | Pixel-controlled boxes & borders |
| Source code: open-source on GitHub | Trust, community contributions |

Later: optional web version (with file upload only) for Zoho/Excel users.

---

## 14. Roadmap

| Phase | Scope | Outcome |
|---|---|---|
| **0 — Knowledge** (2–3 wks) | Collect official texts: Schedule III + GN Div I, ICAI GN Non-Corporate, ICAI GN LLP, Schedule II, IT depreciation tables (1961 & 2025); sample TBs from 10 real clients; mapping dictionary | Format packs v1 drafted |
| **1 — MVP** | Excel + Tally import, mapping, error engine, **Non-Corporate (Firm, Proprietor, HUF, AOP/BOI)** + **Company Div I**, basic notes, PDF print | Usable by our office |
| **2** | LLP pack, FAR + both depreciation engines, toggles, print designer, roll-forward, Excel/Word export | Public free release |
| **3** | Zoho API, BUSY auto-detect, Cash Flow auto, ratios & ageing, deferred tax | Feature parity+ with paid tools |
| **4** | Ind AS (Div II), XBRL export, ITR schedule export, community mapping dictionary | Advanced |

---

## 15. Keeping it free (non-commercial by design)

- Purpose is public good, not revenue: **no paid tier, no ads, no telemetry of client data**.
- Open-source licence: **AGPL-3.0 recommended** — anyone may use it free, and nobody can take it closed and sell it.
- Zero hosting cost (desktop + GitHub) → nothing to fund.
- Volunteer CA reviewers maintain format packs.
- Disclaimer: tool assists preparation; responsibility for statements remains with the preparer/auditor.

---

## 16. Decisions needed from you

1. Platform: **Desktop-first (recommended)** vs Web-first?
2. First entities for MVP: **Non-corporate + Company Div I** (recommended) — agree?
3. Licence: AGPL-3.0 (recommended, keeps it free forever) — agree?
4. Product name?
5. Can you share 5–10 anonymised real trial balances (Tally/Busy/Excel) to train the auto-mapping?

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
