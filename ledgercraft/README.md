# LedgerCraft

Free, offline software that turns books into financial statements in the
prescribed format (Schedule III Division I, ICAI Guidance Notes for
non-corporate entities and LLPs), checks the books the way an experienced CA
would, and prepares the auditor's reference workbook. No internet and no AI are
needed; an optional local AI (Ollama) can explain and suggest.

## For users (Windows)

1. Download `LedgerCraft-Setup-<version>.exe` (GitHub → Actions/Releases →
   *LedgerCraft-windows*) and run it. No administrator rights are needed; it
   adds LedgerCraft, the user guides and the practice books to the Start menu.
   Windows may say "Unknown publisher" (the program is not code-signed):
   choose *More info → Run anyway*. The plain `LedgerCraft.exe` in the same
   download also runs without installing.
2. Open LedgerCraft. A small window opens (keep it open) and the app opens in
   your browser at `http://127.0.0.1:7878`. Nothing is sent anywhere.
3. The first screen asks what you want to do: **Make financial statements**,
   **Check and analyse books**, or **Tax audit help**. Only the steps for that
   job are then shown on the left, numbered, with progress under each and
   "What to do now" at the bottom. Main buttons are always at the top right of
   each screen; help on every screen in English, Hindi or Marathi.
   Find any client with the search bar at the top (Ctrl+K); Alt+1…9 jumps to a step.
4. Your work is saved in `Documents\LedgerCraft Data` (one folder per client
   and year). Copy that folder to back up. Uninstalling never deletes it.
5. User guides: [Hindi](docs/GUIDE-hi.md), [Marathi](docs/GUIDE-mr.md) (this
   README is the English one).

### Import
* **Tally (one click):** in TallyPrime enable the XML server (F1 Help →
  Settings → Connectivity, port 9000), open the company, click *Find
  companies*, then *Import*. Trial balance, last year's balances and the day
  book (month by month) are fetched.
* **Files:** trial balance from Tally, **Zoho Books** (Reports → Trial Balance
  → Export) or **BUSY** (with its *List of Accounts* as the account master), or
  the LedgerCraft template, or a hand-made Excel (title lines, Debit/Credit
  columns and a Total row are fine); day book (CSV/Excel); fixed asset register.
* **Branches:** add each branch's trial balance and day book; they are combined
  with the head office. Inter-branch accounts must cancel out, otherwise the
  final copy is refused.
* **For reconciliation:** bank statements (Excel/CSV from net banking),
  GSTR-2B (JSON or Excel from the GST portal), Form 26AS (text file from TRACES).

### What it checks (flags only; nothing is changed silently)
Trial balance and opening balances against last year; wrong grouping (loan
under creditors, OD/CC under bank, GST/TDS under creditors, asset in
expenses); customer/supplier/bank balances on the wrong side; unbalanced,
duplicate, out-of-period vouchers; negative cash on any day; cash payments
above ₹10,000 (₹35,000 transporters), assets bought in cash, cash receipts of
₹2 lakh or more; loans taken or repaid in cash with **only principal counted as
accepted** (interest and TDS separated); fixed asset register against the
books (net block and depreciation).

### What it prints
Cover and contents; Balance Sheet; Statement of Profit and Loss; Cash Flow
Statement (AS 3); notes with Schedule III sub-classification, ageing of
receivables and payables, MSME split, partner-wise capital movement, PPE
schedule, 11 ratios with reasons; tax depreciation annexure. In ₹, thousands,
lakhs, millions or crores, **always casting exactly**. Two layouts: Boxed or
Ruled. Outputs: PDF, Excel, HTML preview, Auditor Reference Workbook, and a
manifest with SHA-256 fingerprints; Word (.docx) of the statements; an optional
charts annexure (expense make-up, ageing, monthly sales and purchases).
Disclosures you enter (share capital, >5% holders, promoters, contingent
liabilities and commitments, related parties, MSME interest, accounting
policies) go into the notes. Statements can be for a full year, a quarter, a
month or any period up to 18 months; depreciation is pro-rated. See
[DESIGN.md](DESIGN.md).

### Reconciliations
* **Bank:** each statement line is matched with the day book (same amount,
  nearby date, cheque number when present). What is left is the bank
  reconciliation statement: cheques not presented, deposits not cleared, bank
  charges and interest not in the books, and any difference.
* **GSTR-2B and 26AS:** input tax credit per supplier and TDS per deductor in
  the books against the portal files; parties are matched by GSTIN/TAN or a
  similar name, which you check. Both go into the auditor workbook.

### Next year
*Start next year* on a client creates the next year with this year's final
balances (adjustments included) as last year's figures, carries the fixed asset
register (sold assets dropped), settings, tags and signing details, and asks
every disclosure again.

### Your own changes (all optional, all recorded)
* **Adjustments:** pass journal entries over the imported books (provisions,
  rectifications, audit adjustments, closing stock). New ledgers can be
  created. Each can be edited, switched off or deleted; the books themselves
  are never changed. They are listed in the auditor workbook.
* **What to print:** switch the Balance Sheet, P&L, Cash Flow Statement, notes,
  tax depreciation annexure and cover on or off; choose which files to export.
* **Mapping:** change where any ledger is shown; **Reset** returns to the
  automatic choice.
* **Delete** a client year (it goes to the Recycle Bin and can be restored);
  **Remove** an imported file.

### Tax audit
FY 2025-26 and earlier: Income-tax Act, 1961 sections and Form 3CA/3CB/3CD.
From Tax Year 2026-27: Income-tax Act, 2025 (ss.36, 185, 186, 188) and the new
Form 26 (section 63, Rule 47 of the Income-tax Rules, 2026). The app picks the
references by the year of the client. The **tax audit helper** workbook
(`Tax_Audit_Helper_Form_3CD.xlsx` or `..._Form_26.xlsx`) lists the supporting
data by clause; Form 26 clause numbers are not yet mapped from the official
form, and references not checked against the official text are marked so.

### Audit trail
Every import, mapping, setting, export and AI suggestion is recorded in a
tamper-evident, hash-chained log on this PC: editing or removing an entry in the
middle is detected and shown. It is not tamper-proof (someone with full control of
the files could delete the whole history), so keep exports as the outside record.

### What LedgerCraft decides and what it asks you
See [TRUTH-MODEL.md](TRUTH-MODEL.md). In short: arithmetic and certain placements
are automatic; ambiguous placements, name-based placements and AI suggestions
wait for your confirmation; disclosures (share capital, contingent liabilities,
related parties, MSME) are never assumed nil. A **draft** can always be printed
and says how many problems are open; a **final** copy is refused until they are
resolved. Each client year is pinned to its rule and format pack.

### Local AI (optional)
Install Ollama (free, ollama.com), then in LedgerCraft open *Local AI* and
click *Download model* once (recommended `qwen2.5:3b`, about 2 GB, needs an
8 GB RAM PC). It explains findings in English, Hindi or Marathi, suggests
mappings (only from the allowed list), drafts ratio explanations and answers
questions about the engagement. It never changes a figure; every answer is
labelled and logged.

## For developers

```
cargo test --all                       # 30+ tests incl. ground truth, casting, Tally/Zoho mocks, app API
cargo run --release -p lc-app          # the desktop app (LedgerCraft)
cargo run --release -p lc-cli -- help  # command line (ledgercraft)
cargo run --release -p lc-cli -- practice-data --out samples/practice   # 8 practice books with answers
makensis /DDIST=<folder> installer/ledgercraft.nsi                        # Windows installer (CI does this)
cargo run --release -p lc-cli -- bench --vouchers 1000000
cargo build --release --target x86_64-pc-windows-gnu -p lc-app   # Windows build from Linux (mingw)
```

| Crate | Role |
|---|---|
| `lc-core` | Engine: exact money, groups, mapping, checks, figures, rounding, ratios, ageing, capital, FAR/depreciation, report model. Laws and formats as data in `packs/`. |
| `lc-io` | Readers (Excel/CSV, Tally XML, Zoho, BUSY, bank statements, GSTR-2B, 26AS), renderers (Typst PDF, Excel, HTML, Word), tax audit helper, export folder |
| `lc-ai` | Ollama client and guarded assistant |
| `lc-app` | Desktop app: local server (127.0.0.1 + session token), storage, audit trail, UI |
| `lc-cli` | Command line |
| `lc-testdata` | Practice-book generator (firm, company, LLP, HUF, head office + branch, messy Excel) with planted mistakes and the expected answers |

### How correctness is proved
* Generated books (small firm to ~1 million vouchers) with planted mistakes:
  every mistake must be found, clean books must raise nothing (60 random books).
* The printed report must cast in 6 units across 33 books.
* Excel/CSV, Tally XML and the app API must reproduce identical results.
* Deliberately broken rules make the tests fail (mutation checks).
* The Windows build was run under Wine through the full browser flow.

### Licence
No licence is granted (all rights reserved by the owner). Free to use.
Bundled fonts: Liberation Serif (SIL OFL 1.1) and DejaVu Serif (Bitstream Vera
licence) for printed statements, see `crates/lc-io/fonts/LICENSE-*.txt`; IBM Plex
Sans and IBM Plex Sans Devanagari (SIL OFL 1.1) for the app screens, see
`crates/lc-app/src/ui/fonts/LICENSE-IBM-Plex-OFL.txt`.
