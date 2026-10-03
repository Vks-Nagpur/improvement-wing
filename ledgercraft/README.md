# LedgerCraft

Free, offline software that turns books into financial statements in the
prescribed format (Schedule III Division I, ICAI Guidance Notes for
non-corporate entities and LLPs), checks the books the way an experienced CA
would, and prepares the auditor's reference workbook. No internet and no AI are
needed; an optional local AI (Ollama) can explain and suggest.

## For users (Windows)

1. Download `LedgerCraft.exe` (GitHub → Actions/Releases → *LedgerCraft-windows*).
2. Double-click it. A small window opens (keep it open) and the app opens in
   your browser at `http://127.0.0.1:7878`. Nothing is sent anywhere.
3. Work through the steps on the left:
   **Clients and years → Import books → Check → Map ledgers → Presentation →
   Sign and export → Audit trail.**
4. Your work is saved in `Documents\LedgerCraft Data` (one folder per client
   and year). Copy that folder to back up.

### Import
* **Tally (one click):** in TallyPrime enable the XML server (F1 Help →
  Settings → Connectivity, port 9000), open the company, click *Find
  companies*, then *Import*. Trial balance, last year's balances and the day
  book (month by month) are fetched.
* **Files:** trial balance from Tally, **Zoho Books** (Reports → Trial Balance
  → Export) or **BUSY** (with its *List of Accounts* as the account master), or
  the LedgerCraft template; day book (CSV/Excel); fixed asset register.

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
manifest with SHA-256 fingerprints. See [DESIGN.md](DESIGN.md).

### Audit trail
Every import, mapping, setting, export and AI suggestion is recorded in a
hash-chained log. Editing or deleting any past entry is detected and shown.

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
cargo run --release -p lc-cli -- practice-data --out practice
cargo run --release -p lc-cli -- bench --vouchers 1000000
cargo build --release --target x86_64-pc-windows-gnu -p lc-app   # Windows build from Linux (mingw)
```

| Crate | Role |
|---|---|
| `lc-core` | Engine: exact money, groups, mapping, checks, figures, rounding, ratios, ageing, capital, FAR/depreciation, report model. Laws and formats as data in `packs/`. |
| `lc-io` | Readers (Excel/CSV, Tally XML, Zoho, BUSY), renderers (Typst PDF, Excel, HTML), export folder |
| `lc-ai` | Ollama client and guarded assistant |
| `lc-app` | Desktop app: local server (127.0.0.1 + session token), storage, audit trail, UI |
| `lc-cli` | Command line |
| `lc-testdata` | Practice-book generator with planted mistakes and the expected answers |

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
licence); see `crates/lc-io/fonts/LICENSE-*.txt`.
