#!/usr/bin/env python3
"""Builds the LedgerCraft build report (HTML, then Word and PDF via LibreOffice).

Tables of rules, formats, rates, ratios, screens, routes, tests and commits are
read from the source tree at build time, so they match the code exactly.
Run from the repository root:  python3 ledgercraft/docs/report/make_report.py
"""
import html
import json
import re
import subprocess
import sys
from datetime import date
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
LC = ROOT / "ledgercraft"
OUT = LC / "docs" / "report"
HIST = Path(sys.argv[1]) if len(sys.argv) > 1 else None  # optional: extracted chat history (json)

e = html.escape
parts = []


def h(level, text, anchor=None):
    parts.append(f"<h{level}>{e(text)}</h{level}>")


def p(text):
    parts.append(f"<p>{text}</p>")


def ul(items):
    parts.append("<ul>" + "".join(f"<li>{i}</li>" for i in items) + "</ul>")


def table(head, rows, widths=None):
    th = "".join(f"<th>{e(x)}</th>" for x in head)
    body = "".join(
        "<tr>" + "".join(f"<td>{c}</td>" for c in r) + "</tr>" for r in rows
    )
    parts.append(f'<table border="1" cellspacing="0" cellpadding="4"><thead><tr>{th}</tr></thead><tbody>{body}</tbody></table>')


def read(p_):
    return (LC / p_).read_text(encoding="utf-8")


def sh(cmd):
    return subprocess.run(cmd, shell=True, capture_output=True, text=True, cwd=ROOT).stdout


def tag(s):
    colors = {"BUILT": "#1d6b46", "TESTED": "#1d6b46", "UNVERIFIED": "#9a5b00", "PENDING": "#a3262a", "NOT BUILT": "#a3262a", "PARTLY": "#9a5b00", "CONSIDERED": "#555"}
    return f'<b style="color:{colors.get(s, "#333")}">[{s}]</b>'


rules = json.loads(read("crates/lc-core/packs/rules.json"))
dep = json.loads(read("crates/lc-core/packs/depreciation.json"))
packs = {k: json.loads(read(f"crates/lc-core/packs/{k}.json")) for k in ["company_div1", "llp", "non_corporate"]}
commit = sh("git -C ledgercraft rev-parse --short HEAD").strip()
today = date.today().strftime("%d %B %Y")

# ---------------------------------------------------------------- cover
parts.append(f"""<h1>LedgerCraft: complete build report</h1>
<p><b>What this is:</b> everything decided, built, referenced and left pending in LedgerCraft, from the first request to the current build, so it can be checked line by line.</p>
<p><b>Build:</b> branch <code>LedgerCraft</code>, commit <code>{commit}</code>, report generated {today}. Repository: github.com/Vks-Nagpur/improvement-wing (folder <code>ledgercraft/</code>).</p>
<p><b>How to read the status tags:</b> {tag("BUILT")} in the program and covered by automatic tests; {tag("PARTLY")} built with a stated limitation; {tag("UNVERIFIED")} built, but the legal text it relies on has not been read from the official source; {tag("NOT BUILT")} not in the program; {tag("CONSIDERED")} studied and deliberately not used.</p>
<p><b>Honesty note:</b> nothing in this program has yet been run on real client books. All testing used generated practice books with known answers. Official government and ICAI websites could not be opened from the build environment, so several legal references rest on search results and secondary websites; each such item is marked {tag("UNVERIFIED")} below and in the app itself.</p>""")

parts.append("""<h2>Contents</h2><ol>
<li>Your brief, in your words</li><li>Every request and what was done</li><li>Status at a glance</li>
<li>Principles: the accounting truth model</li><li>Legal and professional basis (formats, sections, guidance notes, rates)</li>
<li>Checks the program runs (all rules, with references)</li><li>Depreciation</li><li>Ratios, ageing and analysis</li>
<li>Tax audit helper, reconciliations and the auditor workbook</li><li>App flow and every screen</li>
<li>Theme and design</li><li>Imports: what files are read and how</li><li>Exports: what files are produced</li>
<li>Technical architecture</li><li>Local AI</li><li>Testing and verification</li><li>Installation and distribution</li>
<li>Sources consulted</li><li>Verification register (facts and their status)</li><li>Pending, unverified and known limitations</li>
<li>Change log (every commit)</li><li>Appendix A: Truth model (full text)</li><li>Appendix B: Your messages (verbatim)</li></ol>""")

# ---------------------------------------------------------------- 1 brief
h(2, "1. Your brief, in your words")
p("Taken from your first messages (quoted, spelling as typed):")
parts.append("""<blockquote>I want to build a software that can literally help Entities prepare Financial statements in Format Prescribed by ICAI AND LAW as per Guidance note which updates itself from time to time.. Give simple explanation of why this is required. Right from Import Tally Data with a single click button, import from zoho books, Import from busy, Import excel trial balance in particular format. Shift Current Year to right side (to make Next year financials so current year becomes previous year). Shows error. Ledger making most simple and fast possible... Search for Black horse excellence ... I want a make a product free for all. Self responsible.. compatible of managing Financials Depreciation calculation inventory and All step by step. Toggle switches etc.. Even to that extent that Print options... What to display when to. Notes to Accounts. Design Format of Boxes ... Companies, individual, huf, aop, boi, Partnership firm, LLP. Dep as per income tax master selector... fixed asset register. All shall be such simple ... done in under 3-4 mins after all imports.</blockquote>""")
p("Later additions that shaped the build: intent is to solve the problem, not to sell; study CompuTax/CompuBal; toughest coding that prevents data errors; self-correction and troubleshooting; data saving and audit trail; map once, not every time; UDIN is a paste-only space (no integration); signing-ready output in one folder (PDF/Excel); Windows desktop; no licence; product name LedgerCraft; check very large books (thousands of vouchers); cash-transaction compliance (old ss.269SS, 269ST, 269T, 40A(3)) in a separate sheet for the auditor; work offline without AI, with optional free local AI (Ollama); opening balance differences, loans taken/repaid counting principal only, wrong grouping (loan under creditors); proper design structure for tables and boxes; manual adjustments; first screen asks intent; professional look like CompuTax with client search; new Income-tax Act 2025 and new forms; last-year comparison for decisions.")

# ---------------------------------------------------------------- 2 timeline
h(2, "2. Every request and what was done")
timeline = [
    ("Best GitHub repositories for non-developers; ideas for Finance + tech", "Researched and answered (lists of useful open-source apps; concept ideas). No code.", "Research only"),
    ("Build software for ICAI/law-format financial statements; imports; roll forward; errors; all entity types; FAR; toggles; print; notes; free", "Wrote the product blueprint (docs/financial-statements-tool/BLUEPRINT.md): why it is needed, benchmark (Black Horse Excellence, CompuBal/CompuTax), entity frameworks, import design, error engine, mapping, FAR, notes, toggles, export, roll forward, versioned law packs, roadmap.", "BLUEPRINT.md"),
    ("Intent is not to sell; study CompuTax/CompuBal; toughest error prevention; self-correction; audit trail; UDIN source; legal yet simple language", "Blueprint sections 9A to 9G added: data integrity, saving and audit trail, self-correction, map once, UDIN, export, compliance matrix.", "BLUEPRINT.md"),
    ("UDIN paste only; statements straight to a folder (PDF/Excel); how will users get it", "UDIN is a paste-only field printed as entered. One-click export to a dated folder. Distribution via GitHub download (Windows).", "Export, Sign and export screen"),
    ("Windows desktop, no licence, name LedgerCraft", "Named LedgerCraft; no licence file (all rights reserved, free to use); Windows build.", "README Licence"),
    ("Train on huge data; cash compliance checks (269SS/ST/T); auditor sheet; offline without AI; optional local AI", "Engine v0.1: exact paise arithmetic, 32 checks at the time, practice-book generator with planted mistakes, ~1 million vouchers checked in about 5 seconds, auditor workbook, Ollama assistant.", "lc-core, lc-testdata, lc-ai"),
    ("Opening balance differences, loans principal only, loan grouped under creditors; find glitches yourself", "Planted ~30 glitches in practice books; each must be found exactly; clean books must raise nothing. Loan register separates principal from interest and TDS.", "Checks; tests"),
    ("Proper design structure for final output (tables, boxes); install Ollama", "Report model with exact rounding in any unit, Typst PDF (Boxed and Ruled layouts), Excel and HTML renderers, FAR with Schedule II and IT depreciation, cash flow (AS 3), ratios, ageing, LLP pack. Ollama installed and connected.", "lc-core/report, lc-io/render"),
    ("Summary for a non-technical person; what more can you do; connectors; better than Black Horse? studied YouTube?", "Plain summary given. Connectors listed (Tally XML server, Zoho Books API, BUSY files, GST/26AS downloads). Black Horse video studied via its page and indexed material (site partly blocked).", "Chat"),
    ("Buttons and features in proper place and size; optional buttons (hide cash flow, delete ...)", "Actions grouped top right of every screen; switches to hide statements/notes; delete client year to a Recycle Bin; remove an imported file.", "UI"),
    ("Manual adjustments are important", "Adjustments: journal entries over imported books, new ledgers, edit, switch off, delete, all audited; listed in auditor workbook.", "Adjustments screen"),
    ("accountingtool.in: improve wherever we lag; fix lag; first screen asks intent", "Start screen with three intents and per-intent steps; each request on its own thread (no freeze); buttons verified by a browser test.", "Start screen; server"),
    ("Look like CompuTax: client search, bars; Form 3CD replaced under new Act 2025", "Top bar with client search (Ctrl+K), status bar; tax references switch by year: Income-tax Act 1961 and Form 3CD up to FY 2025-26; Income-tax Act 2025 and Form 26 from Tax Year 2026-27.", "UI; rules pack"),
    ("Share images", "Screenshots sent.", "Chat"),
    ("Install design and fonts skills; alignment everywhere", "Design pass: IBM Plex Sans and Plex Devanagari bundled, spacing scale, aligned tables and buttons.", "UI"),
    ("Rename branch", "Default branch renamed by you to LedgerCraft; all work pushed there.", "Git"),
    ("Where are we lagging?", "Gap list given: disclosures, roll forward, branches, periods, Word, graphs, tax audit workbook, GST 2B / 26AS, bank reconciliation, installer, Hindi/Marathi guide, practice books, small screens.", "Chat"),
    ("Do all of it; testing at the end", "All gap items built (see sections 5 to 17).", "Many"),
    ("External review pasted (compiler not AI; confidence mapping; three disclosure states; draft vs final; rule provenance; pinned packs; tamper-evident)", "All adopted. TRUTH-MODEL.md written; mapping status Rule/Suggested/Review/Confirmed; disclosures Known/Nil/Unknown; final copy blocked on open items; every rule carries source and verification; packs pinned per client year; audit trail called tamper-evident.", "TRUTH-MODEL.md"),
    ("Share screenshots", "14 screenshots sent.", "Chat"),
    ("How to install", "Built the Windows installer here and sent it; steps given.", "Installer"),
    ("Bento dashboard animation; remove unnecessary; clean", "Dashboard in a bento grid with entrance motion; detail tables folded; help boxes fold after first visit; extra home note removed.", "Dashboard"),
    ("Comparison from last year, important for decisions", "Every dashboard figure shows last year and the change; 'Points to look at'; 'Biggest movements'; note-by-note comparison; last-year column in Map ledgers.", "Dashboard; Map ledgers"),
    ("This report", "This document, generated from the source code.", "docs/report"),
]
table(["#", "Your request (short)", "What was done", "Where"], [[str(i + 1), e(a), e(b), e(c)] for i, (a, b, c) in enumerate(timeline)])

# ---------------------------------------------------------------- 3 status
h(2, "3. Status at a glance")
status = [
    ("Import: Tally one click (XML server, port 9000)", "BUILT", "Tested against a mock Tally server, not a real TallyPrime."),
    ("Import: Zoho Books (API and Excel export)", "PARTLY", "Excel export tested; API code written against Zoho's documented endpoints but not run against a live Zoho account."),
    ("Import: BUSY (Excel + List of Accounts)", "BUILT", "Tested on files written in BUSY's documented layout, not on a real BUSY export."),
    ("Import: Excel/CSV trial balance, day book, FAR (incl. hand-made sheets)", "BUILT", ""),
    ("Checks (40 rules)", "BUILT", "Legal references of several are unverified (section 6)."),
    ("Mapping with confidence status, memory per client", "BUILT", ""),
    ("Formats: Company Div I, LLP, Non-corporate (Firm, Proprietor, HUF, AOP, BOI)", "UNVERIFIED", "Line items not yet compared word-by-word with the official Schedule III / ICAI texts; packs marked draft."),
    ("Notes, ageing, MSME split, partner capital, PPE schedule, cash flow (AS 3), 11 ratios", "BUILT", ""),
    ("Depreciation: Schedule II (book) and IT block rates, 180-day rule", "UNVERIFIED", "Rates from secondary sources; new-Act section number conflicting (33 vs 34)."),
    ("Manual adjustments", "BUILT", ""),
    ("Disclosures: three states, final copy blocked when unknown", "BUILT", ""),
    ("Draft vs final copy", "BUILT", ""),
    ("Roll forward to next year", "BUILT", ""),
    ("Branch consolidation", "BUILT", ""),
    ("Monthly / quarterly / custom periods", "BUILT", ""),
    ("Outputs: PDF, Excel, Word, HTML preview, auditor workbook, manifest", "BUILT", "Word opens in LibreOffice; not yet opened in Microsoft Word by us."),
    ("Tax audit helper (Form 3CD to FY 2025-26; Form 26 from TY 2026-27)", "PARTLY", "Form 26 clause numbers not mapped (official form not read)."),
    ("Bank reconciliation", "BUILT", "Tested on generated statements; real bank formats vary."),
    ("GSTR-2B and 26AS reconciliation", "PARTLY", "Built to the published file layouts; not yet tried on real downloads."),
    ("Dashboard (bento), last-year comparison, charts", "BUILT", ""),
    ("Audit trail (hash-chained, tamper-evident)", "BUILT", ""),
    ("Local AI (Ollama), explain/suggest only", "BUILT", "Optional."),
    ("Help in English, Hindi, Marathi; tour; guides", "BUILT", "Hindi/Marathi wording written by us; a native-speaker review is advisable."),
    ("Windows installer", "PARTLY", "Built and packaged; not yet installed on a real Windows PC by us; not code-signed."),
    ("Ind AS (Div II/III), XBRL, ITR schedules, deferred tax computation", "NOT BUILT", ""),
]
table(["Area", "Status", "Note"], [[e(a), tag(b), e(c)] for a, b, c in status])

# ---------------------------------------------------------------- 4 truth model summary
h(2, "4. Principles: the accounting truth model")
ul([
    "LedgerCraft is a <b>financial-statement compiler</b>, not an AI accounting program: books go in, deterministic and traceable statements come out.",
    "Money is held as whole paise (integers). Debit is positive, credit negative. No floating-point rounding errors in totals.",
    "The imported books are never changed. Your adjustments sit on top and can be switched off.",
    "Mapping states: <b>Rule</b> (only one place possible), <b>Suggested</b> (LedgerCraft's proposal, needs your confirmation), <b>Review</b> (your earlier choice but the books changed), <b>Confirmed</b>, <b>Unmapped</b>. Memory is kept per client with context (group and Dr/Cr side), not just the name.",
    "Disclosures have three states: <b>given</b>, <b>explicitly Nil</b>, <b>unknown</b>. Unknown never becomes Nil; the draft prints 'information not provided'.",
    "A <b>draft</b> can always be printed (it says how many problems are open). A <b>final signing copy</b> is refused while any blocker is open (section 4 of Appendix A).",
    "Rules and formats are data with provenance (authority, document, section, effective dates, verification) and are pinned per client year; a newer pack is adopted only when you choose to migrate.",
    "The audit trail is hash-chained: editing or deleting an entry in the middle is detected. It is tamper-evident, not tamper-proof.",
    "AI is outside the trust boundary: it explains and suggests, never changes a figure, and every answer is labelled and logged.",
])
p("Full text in Appendix A.")

# ---------------------------------------------------------------- 5 legal basis
h(2, "5. Legal and professional basis")
h(3, "5.1 Which format applies to which entity")
table(["Entity", "Format followed", "Status"], [
    ["Company (not Ind AS)", "Companies Act, 2013, Schedule III, Division I (as amended by MCA notification of 24-03-2021) and ICAI Guidance Note on Division I (Non Ind AS) Schedule III (revised January 2022)", tag("UNVERIFIED") + " line items to be compared with the official text"],
    ["LLP", "ICAI Guidance Note on Financial Statements of Limited Liability Partnerships", tag("UNVERIFIED")],
    ["Partnership firm, Proprietor, HUF, AOP, BOI", "ICAI Guidance Note on Financial Statements of Non-Corporate Entities (issued August 2023). Applicability reported as phased: Phase I from periods beginning 1-4-2025 (turnover above ₹5 crore), Phase II from 1-4-2026 (all covered entities), per ICAI ASB announcement of 31-03-2026 as reported by secondary sources", tag("UNVERIFIED") + " official ICAI page could not be opened"],
    ["Company (Ind AS), NBFC", "Schedule III Divisions II and III", tag("NOT BUILT")],
])
h(3, "5.2 Line items of each format (exactly as in the build)")
p("Codes: H = heading, S = sub-heading, I = line item (shown with a note number), SUM = computed total. 'keep' = printed even when nil.")
for k, pk in packs.items():
    h(4, f"{pk['name']}  ({pk['id']}, status: {pk['status']})")
    p(f"Titles: '{e(pk['bs_title'])}' and '{e(pk['pl_title'])}'. Profit of the year goes to: {e(pk['profit_to'])} ('{e(pk['profit_note_label'])}').")
    for side in ["balance_sheet", "profit_loss"]:
        rows = []
        for r in pk[side]:
            t = {"h": "H", "s": "S", "i": "I", "sum": "SUM"}.get(r["t"], r["t"].upper())
            extra = r.get("head") or ", ".join(r.get("heads", [])) or r.get("calc", "")
            rows.append([t, e(r["label"]), e(str(extra)), "keep" if r.get("keep") else ""])
        parts.append(f"<p><b>{'Balance Sheet' if side == 'balance_sheet' else 'Statement of Profit and Loss'}</b></p>")
        table(["Type", "Line", "Built from (internal head)", ""], rows)

h(3, "5.3 Income-tax: which Act and which form, by year")
table(["Year of the client", "Act used for references", "Tax audit form", "Status"], [
    ["FY 2025-26 and earlier", "Income-tax Act, 1961", "Form 3CA / 3CB with Form 3CD", tag("BUILT")],
    ["Tax Year 2026-27 onwards (from " + e(rules["new_act_from"]) + ")", "Income-tax Act, 2025", "Form 26 (section 63; Rule 47, Income-tax Rules 2026, G.S.R. 198(E) of 20-03-2026 as reported)", tag("UNVERIFIED") + " clause numbers of Form 26 not mapped"],
])
h(3, "5.4 Old and new section numbers used")
table(["Item", "Income-tax Act, 1961", "Income-tax Act, 2025 (as used)", "Status"], [
    ["Cash payment of expenses above ₹10,000 (₹35,000 transporters)", "s.40A(3)", "s.36", tag("UNVERIFIED") + " per published section listings; sub-section not read"],
    ["Asset bought in cash (cost not allowed for depreciation)", "s.43(1), 6th proviso", "not confirmed (shown as 'actual-cost rule')", tag("UNVERIFIED")],
    ["Cash receipt of ₹2 lakh or more", "s.269ST", "s.186", tag("UNVERIFIED")],
    ["Loan/deposit accepted in cash or by journal (₹20,000 or more)", "s.269SS", "s.185", tag("UNVERIFIED")],
    ["Loan/deposit repaid in cash", "s.269T", "s.188", tag("UNVERIFIED")],
    ["Depreciation", "s.32", "s.33 (rules pack) — secondary sources also cite s.34: conflicting", tag("UNVERIFIED")],
    ["Tax audit", "s.44AB", "s.63", tag("UNVERIFIED")],
])
p("In the app every unverified reference is printed with '[reference not yet verified against the official text]'.")
h(3, "5.5 Thresholds (data in rules pack)")
table(["Threshold", "Value"], [[e(k), f"₹{v / 100:,.0f}"] for k, v in rules["thresholds"].items()])
h(3, "5.6 Other legal and professional points built in")
table(["Point", "Reference", "How it is built", "Status"], [
    ["Audit trail (edit log) in accounting software", "Companies (Accounts) Rules, 2014, rule 3(1) proviso, from FY 2023-24", "Hash-chained log of every import, mapping, setting, adjustment, export and AI answer; cannot be switched off", tag("BUILT")],
    ["UDIN", "ICAI UDIN guidelines", "Paste-only field, printed as entered; no integration (no public generation API found)", tag("BUILT")],
    ["Rounding off", "Schedule III general instructions (units by turnover)", "Choice of ₹, thousands, lakhs, millions, crores; totals always cast exactly after rounding (largest remainder)", tag("BUILT")],
    ["Ageing of trade receivables and payables", "Schedule III (2021 amendment): less than 6 months, 6 months-1 year, 1-2, 2-3, more than 3 years; disputed / undisputed; MSME split for payables", "Ageing note and dashboard charts", tag("BUILT")],
    ["Ratios with reasons for change over 25%", "Schedule III (2021 amendment), 11 ratios", "Computed both years; changes over 25% flagged 'explain'", tag("BUILT")],
    ["MSME disclosure", "MSMED Act, 2006 s.22 interest disclosures", "Six interest figures entered by you; principal from creditors tagged msme", tag("BUILT")],
    ["Share capital, >5% holders, promoter shareholding", "Schedule III Division I", "Disclosures screen; total must agree with books or final copy is refused", tag("BUILT")],
    ["Cash flow statement", "AS 3 (indirect method)", "Prepared; can be switched off", tag("BUILT")],
    ["Accounting policies wording", "AS 1 / framework per entity", "Standard text per entity type, editable; a warning always reminds you to review it", tag("BUILT")],
])

# ---------------------------------------------------------------- 6 rules
h(2, "6. Checks the program runs (all rules, with references)")
p(f"Rules pack <code>{e(rules['pack_id'])}</code>, version <code>{e(rules['version'])}</code>. Pack-level verification note: {e(rules['verification'])}.")
p(e(rules["note"]))
sev = {"blocker": "Must fix", "warning": "Check", "info": "Note"}
rows = []
for code, r in rules["rules"].items():
    src = r.get("source", {})
    ref = " / ".join(x for x in [r.get("old_ref", ""), r.get("new_ref", "")] if x)
    rows.append([f"<code>{e(code)}</code>", sev.get(r["severity"], r["severity"]), e(r["title"]), e(r.get("simple", "")), e(ref),
                 e(" · ".join(x for x in [src.get("authority", ""), src.get("document", ""), src.get("section", "")] if x)),
                 tag("UNVERIFIED") if r.get("verification") == "unverified" else e(r.get("verification", ""))])
table(["Code", "Level", "Title", "Plain meaning", "Section (old / new)", "Source", "Verification"], rows)

# ---------------------------------------------------------------- 7 depreciation
h(2, "7. Depreciation")
p(f"Pack status: {e(dep['status'])}. Residual value used for book depreciation: {dep['residual_pct']}%.")
h(3, "7.1 Book depreciation: useful lives (Companies Act, 2013, Schedule II)")
table(["Key", "Asset class", "Useful life (years)"], [[e(b["key"]), e(b["label"]), str(b["life"])] for b in dep["book_classes"]])
h(3, "7.2 Income-tax depreciation: block rates")
table(["Key", "Block of assets", "Rate %"], [[e(b["key"]), e(b["label"]), str(b["rate"])] for b in dep["it_blocks"]])
ul([
    "Additions put to use for less than 180 days in the year get half the rate (income-tax).",
    "Sold assets reduce the block (sale proceeds); the written-down value is carried forward on roll forward.",
    "Short periods (month, quarter): book depreciation is pro-rated by days of the 12-month year; the income-tax annexure is omitted.",
    "The fixed asset register is checked against the books (net block and depreciation).",
])

# ---------------------------------------------------------------- 8 ratios / analysis
h(2, "8. Ratios, ageing and analysis")
src = read("crates/lc-core/src/ratios.rs")
rat = re.findall(r'push\(\s*"([^"]+)",\s*"([^"]+)",\s*"([^"]+)",\s*"([^"]+)"', src)
table(["Ratio", "Numerator", "Denominator", "Unit"], [[e(a), e(b), e(c), e(d)] for a, b, c, d in rat])
p("A ratio is flagged when it moves by more than 25% against last year.")
h(3, "8.1 Dashboard (what it shows and how it decides)")
ul([
    "Profit for the year with change on last year (₹ and %), margin, and revenue / other income / expenses and tax for both years.",
    "Final copy readiness: must-fix problems, placements to confirm, disclosures to answer, each with an Open button.",
    "Key figures (revenue, total assets, cash and bank, debtors, creditors, borrowings): this year, last year, change in ₹ and %, and two bars.",
    "<b>Points to look at</b> (facts only, no conclusions): profit up/down 25% or more; profit turning to loss; margin moving by 2 percentage points or more; revenue up/down 25% or more; debtors, stock, creditors or employee costs growing at least 15 points faster than revenue; borrowings, finance costs or cash moving 25% or more; ratios moving more than 25%. Small amounts (under 0.5% of the larger of revenue and total assets) are ignored.",
    "<b>Biggest movements</b>: the 8 note lines with the largest change in rupees (capital-account movement lines left out).",
    "Charts: sales and purchases by month, expense make-up this year and last year, ageing of debtors and creditors. Tax audit items with counts and amounts. Ratios for both years.",
    "Detailed tables (folded): key figures, every note line this year against last year, loans, ratios, ageing.",
])

# ---------------------------------------------------------------- 9 tax audit etc
h(2, "9. Tax audit helper, reconciliations and the auditor workbook")
ta = read("crates/lc-io/src/tax_audit.rs")
sheets = sorted(set(re.findall(r'set_name\("([^"]+)"\)', ta)))
table(["Helper workbook sheet (by clause)", "Status"], [[e(s_), tag("BUILT")] for s_ in sheets])
p("File name: Tax_Audit_Helper_Form_3CD.xlsx (up to FY 2025-26) or Tax_Audit_Helper_Form_26.xlsx (from Tax Year 2026-27). For Form 26 the sheets keep the 3CD clause subjects; Form 26 clause numbers are not yet mapped.")
ul([
    "Bank reconciliation: statement lines matched with the day book on the same amount, date within 10 days, cheque number when both have one. Open items: cheques issued not presented, deposits not cleared, credits and debits by the bank only, difference.",
    "GSTR-2B: JSON (data.docdata b2b and cdnr) or portal Excel (B2B sheet, two header rows); ITC per supplier against books; invoices with ITC not available are left out.",
    "Form 26AS: TRACES text file (caret-separated) or Excel; TDS per deductor (TAN) against books.",
    "Parties matched by GSTIN/TAN or a similar name (noise words removed); 'name looks alike' cases shown for you to check.",
    "Auditor workbook sheets: Summary, Index, All findings, Loan register, Adjustments, one sheet per bank reconciliation, GST 2B recon, 26AS recon.",
])

# ---------------------------------------------------------------- 10 app flow
h(2, "10. App flow and every screen")
appjs = read("crates/lc-app/src/ui/app.js")
flows = re.findall(r'(\w+): \{ name: "([^"]+)", steps: \[([^\]]+)\]', appjs)
idx = read("crates/lc-app/src/ui/index.html")
labels = dict(re.findall(r'data-view="(\w+)"[^>]*>(?:<b>\d*</b>)?<span><span class="sn">([^<]+)</span>', idx))
labels.setdefault("home", "Start")
table(["Route (what you chose on the start screen)", "Steps shown on the left, in order"],
      [[e(n), " → ".join(e(labels.get(x.strip().strip('"'), x.strip().strip('"'))) for x in st.split(","))] for _, n, st in flows])
p("Always available: Start (home) to change the intent; client search (Ctrl+K); Alt+1…9 jumps to a step; Help menu (language English/Hindi/Marathi, help on/off, expert mode, tour, shortcuts); 'What to do now' box with a Go button; status bar (client, save state, Act and form in use, rules version, data folder, version).")
h(3, "10.1 Screens: title, purpose and every button")
for m in re.finditer(r'<section data-panel="(\w+)"[^>]*>(.*?)</section>\s*(?=<!--|<section|</main>)', idx, re.S):
    view, body = m.group(1), m.group(2)
    t = re.search(r"<h1>([^<]+)</h1>", body)
    lead = re.search(r'<p class="lead">(.*?)</p>', body, re.S)
    btns = [re.sub(r"<[^>]+>", "", b).strip() for b in re.findall(r"<button[^>]*>(.*?)</button>", body, re.S)]
    btns = [b for b in dict.fromkeys(btns) if b and len(b) < 60]
    sw = [re.sub(r"<[^>]+>", "", s_).strip() for s_ in re.findall(r'<label class="switch">(.*?)</label>', body, re.S)]
    parts.append(f"<p><b>{e(labels.get(view, view))}</b>{(' — ' + e(t.group(1))) if t and t.group(1) != labels.get(view) else ''}<br/>{re.sub(r'<[^>]+>', '', lead.group(1)) if lead else ''}</p>")
    if btns:
        parts.append("<p><i>Buttons:</i> " + "; ".join(e(b) for b in btns) + "</p>")
    if sw:
        parts.append("<p><i>Switches:</i> " + "; ".join(e(s_) for s_ in sw) + "</p>")
p("Buttons created by the program at run time (inside lists and tables): Open, Start next year, Delete, Restore, Choose file / Replace, Remove, Confirm, Suggest, Reset, See list, Explain, Edit, Switch off / on, Open (dashboard readiness), Import last year.")

# ---------------------------------------------------------------- 11 theme
h(2, "11. Theme and design")
css = read("crates/lc-app/src/ui/app.css")
root = re.search(r":root\s*\{(.*?)\}", css, re.S).group(1)
toks = re.findall(r"(--[\w-]+):\s*([^;]+);", root)
table(["Design token", "Value"], [[f"<code>{e(a)}</code>", e(b.strip())] for a, b in toks])
ul([
    "Light theme, deep green accent (OKLCH colour space; neutrals tinted slightly green). Red only for 'Must fix'; amber for 'Check'.",
    "Fonts: IBM Plex Sans (Latin) and IBM Plex Sans Devanagari, bundled (SIL Open Font Licence), so Hindi and Marathi render on any PC. PDF uses Liberation Serif / DejaVu Serif (bundled, free licences).",
    "Layout: top bar (logo, client search, Help, Local AI, Quit); left step bar; main area with title, short purpose line and the screen's buttons at top right; status bar at the bottom. Tables scroll inside their box on small screens with the Actions column pinned.",
    "Dashboard: bento grid (12 columns): Profit (large), Final-copy ring, key figure tiles, Points to look at, Biggest movements, monthly chart, tax audit list, expense make-up, ageing, ratios.",
    "Motion: tiles rise in turn (380 ms, 45 ms apart), figures count up (600 ms), bars grow, chart lines draw; plays once per set of results; off when Windows 'reduce motion' is on. Only transform/opacity are animated.",
    "Charts: categorical colours #2a78d6, #eb6834, #1baf7a, #eda100, #e87ba4, #008300 (checked with a colour-blind validator; direct labels added because two colours are low-contrast); ageing uses one blue from light (recent) to dark (oldest).",
    "PDF: A4, cover and contents, running headers, page x of y, two table styles (Boxed or Ruled), landscape schedules, DRAFT watermark and 'N problems unresolved' footer on drafts.",
    "Screen sizes checked: 1440, 1280 and 1024 pixels wide.",
])

# ---------------------------------------------------------------- 12 imports
h(2, "12. Imports: what files are read and how")
rd = read("crates/lc-io/src/read.rs")
consts = re.findall(r"const (\w+): &\[&str\] = &\[(.*?)\];", rd, re.S)
table(["Column kind", "Header names recognised (any case)"], [[e(n), e(", ".join(re.findall(r'"([^"]*)"', v)))] for n, v in consts])
ul([
    "Tally: TallyPrime XML server (Help > Settings > Connectivity, port 9000). Fetches companies, trial balance with groups, last year's balances, and the day book month by month.",
    "Zoho Books: trial balance Excel export (account type column mapped to Tally-like groups) or API v3 with OAuth.",
    "BUSY: trial balance Excel with its 'List of Accounts' as the account master for groups.",
    "Excel/CSV: LedgerCraft template; hand-made sheets with title lines, text amounts with Indian commas, Debit/Credit columns, blank lines and Total rows are accepted. A file that cannot be read is refused with the reason.",
    "Day book (CSV/Excel), fixed asset register (Excel), bank statements (Excel/CSV), GSTR-2B (JSON/Excel), Form 26AS (text/Excel), branch trial balances and day books.",
])

# ---------------------------------------------------------------- 13 exports
h(2, "13. Exports: what files are produced")
table(["File", "What it is"], [
    ["Financial_Statements.pdf", "Signing-ready statements (cover, contents, Balance Sheet, P&L, Cash Flow, notes, schedules, annexures, optional charts)"],
    ["Financial_Statements.xlsx", "Same statements in Excel"],
    ["Financial_Statements.docx", "Same statements in Word (optional)"],
    ["Financial_Statements_preview.html", "Preview"],
    ["Auditor_Reference_Workbook.xlsx", "Findings, loan register, adjustments, reconciliations"],
    ["Tax_Audit_Helper_Form_3CD.xlsx / _Form_26.xlsx", "Clause-wise supporting data (optional)"],
    ["analysis.json", "All figures and findings, machine readable"],
    ["export-manifest.json", "SHA-256 fingerprint of every file"],
])
p("Folder: Documents\\LedgerCraft Data\\Exports\\&lt;client&gt;\\FY &lt;year&gt;\\Draft-&lt;date&gt;_v&lt;n&gt; (or Final). A final copy is refused while blockers are open.")

# ---------------------------------------------------------------- 14 architecture
h(2, "14. Technical architecture")
loc = sh("cd ledgercraft && for d in crates/*; do n=$(find $d -name '*.rs' -o -name '*.js' -o -name '*.css' -o -name '*.html' -o -name '*.typ' | xargs cat 2>/dev/null | wc -l); echo \"$d $n\"; done")
crate_role = {"lc-core": "Engine: money, groups, mapping, checks, figures, rounding, ratios, ageing, capital, FAR, adjustments, consolidation, reconciliations, report model, charts", "lc-io": "Readers and renderers (PDF via Typst, Excel, Word, HTML), export, tax audit helper, portal files", "lc-ai": "Ollama client and guarded assistant", "lc-app": "Desktop app: local web server on 127.0.0.1 with a session token, storage, audit trail, screens", "lc-cli": "Command line (analyse books and export, list Tally companies, write practice data, benchmark)", "lc-testdata": "Practice-book generator with expected answers"}
table(["Part", "Role", "Lines of code"], [[e(Path(l.split()[0]).name), e(crate_role.get(Path(l.split()[0]).name, "")), l.split()[1]] for l in loc.strip().splitlines()])
ul([
    "Language: Rust (one .exe, no installation of other software needed). Screens in plain HTML/CSS/JavaScript served by the program to your browser on this PC only.",
    "Storage: Documents\\LedgerCraft Data, one folder per client and year: inputs, settings, mapping memory with context, pinned rule and format packs, audit log, exports, Recycle Bin.",
    "Each request runs on its own thread; changes are written one at a time so two clicks cannot corrupt a file.",
    "Considered and not used: Tauri desktop shell with auto-updater (needs code signing); a bundled AI model inside the program (too large).",
])
srv = read("crates/lc-app/src/server.rs")
routes = sorted(set(re.findall(r'\("(GET|POST)", \[([^\]]*)\]\)', srv)))
table(["Method", "Address"], [[m_, "/" + "/".join(x.strip().strip('"') for x in a.split(",")) ] for m_, a in routes])

# ---------------------------------------------------------------- 15 AI
h(2, "15. Local AI")
ul([
    "Ollama on the same PC (ollama.com, free). Models offered: qwen2.5:3b (recommended; English, Hindi, Marathi; about 2 GB; 8 GB RAM PC), llama3.2:3b, qwen2.5:1.5b (older PCs).",
    "Uses: explain a finding in simple language; suggest a placement for a ledger (must be one of the allowed heads, otherwise rejected); draft a reason for a ratio change; answer questions about the engagement.",
    "Never changes a figure. Every answer is labelled as an AI suggestion with the model name and recorded in the audit trail. The program works fully without it.",
])

# ---------------------------------------------------------------- 16 testing
h(2, "16. Testing and verification")
tests = sh("cd ledgercraft && grep -rn '#\\[test\\]' -A1 crates --include=*.rs | grep -o 'fn [a-z0-9_]*' | sed 's/fn //'").split()
p(f"{len(tests)} automatic tests (Rust), run before every push together with formatting and lint checks. Names:")
parts.append("<p style='font-size:9pt'>" + ", ".join(e(t) for t in tests) + "</p>")
pr = []
for d in sorted((LC / "samples" / "practice").iterdir()):
    j = d / "expected.json"
    if j.exists():
        x = json.loads(j.read_text())
        pr.append([e(d.name), e(x.get("description", "")), str(len(x.get("expected_findings", []))), e(x.get("expected_profit", x.get("expected_profit_combined", "")))])
table(["Practice book", "What it tests", "Planted problems", "Expected profit (₹)"], pr)
ul([
    "Ground truth: every planted mistake must be found, and nothing else; clean books must raise nothing (also 60 random clean books).",
    "Casting: printed statements must add up in 6 units across 33 books.",
    "Round trip: Excel/CSV, Tally XML (mock server) and the app must give identical results.",
    "Browser walkthrough of every screen at 1440, 1280 and 1024 px (create client, import, check, map, adjust, disclose, present, export, audit, delete/restore, search).",
    "Performance: about 1 million vouchers checked in about 5 seconds (benchmark).",
    "<b>Not yet done:</b> real client books; real TallyPrime, Zoho, BUSY, bank, GST and 26AS files; Microsoft Word; installing on a real Windows PC.",
])

# ---------------------------------------------------------------- 17 install
h(2, "17. Installation and distribution")
ul([
    "Installer LedgerCraft-Setup-&lt;version&gt;.exe (NSIS): per-user, no administrator rights; Start menu (LedgerCraft, User guides, Practice books, Uninstall) and desktop icon; uninstall never touches Documents\\LedgerCraft Data.",
    "Not code-signed: Windows shows 'Unknown publisher' (More info → Run anyway). Free signing for open-source projects (SignPath Foundation) needs an open-source licence, which conflicts with 'no licence'.",
    "GitHub Actions builds and tests on Ubuntu and Windows and makes the installer; Actions have not run since 3 October (they may need enabling in the repository's Actions tab).",
    "User guides: README (English), docs/GUIDE-hi.md (Hindi), docs/GUIDE-mr.md (Marathi).",
])

# ---------------------------------------------------------------- 18 sources
h(2, "18. Sources consulted")
p("Web searches made during the build (as typed). Results were read as search summaries; where a page itself was opened it is listed in 18.2.")
if HIST and HIST.exists():
    hist = json.loads(HIST.read_text())
    parts.append("<ol style='font-size:9pt'>" + "".join(f"<li>{e(q)}</li>" for q in hist["searches"]) + "</ol>")
    h(3, "18.2 Pages opened or attempted")
    p("Official government and several tax websites refused connection from the build environment; for those, the content rests on search-result text. The list shows every address attempted.")
    parts.append("<ol style='font-size:9pt'>" + "".join(f"<li>{e(u)}</li>" for u in hist["fetch"]) + "</ol>")
h(3, "18.3 Official sources to be read before final use")
ul([
    "Companies Act, 2013, Schedule III (as amended 24-03-2021) and Schedule II — mca.gov.in / indiacode.nic.in",
    "ICAI Guidance Notes: Division I (Non Ind AS) Schedule III (Jan 2022); Non-Corporate Entities (Aug 2023) and its applicability announcement (31-03-2026); LLPs — icai.org",
    "Income-tax Act, 2025 and Income-tax Rules, 2026 (sections 33/34, 36, 63, 185, 186, 188; Rule 47; Form 26; depreciation table) — incometaxindia.gov.in, including the utility mapping 1961 sections to 2025 sections",
    "Companies (Accounts) Rules, 2014, rule 3(1) proviso; MSMED Act, 2006 s.22",
])

# ---------------------------------------------------------------- 19 verification register
h(2, "19. Verification register (facts and their status)")
bp = (ROOT / "docs/financial-statements-tool/BLUEPRINT.md").read_text()
m = re.search(r"## 17\. Verification status of facts used\n\n(.*?)\n\n", bp, re.S)
if m:
    lines = [l for l in m.group(1).splitlines() if l.startswith("|")][2:]
    rows = [[re.sub(r"\*\*(.*?)\*\*", r"<b>\1</b>", e(c.strip())) for c in l.strip("|").split("|")] for l in lines]
    table(["Fact", "Status", "Source"], rows)
p("Additional: Form 3CD clause 31 sub-clauses (a), (b), (ba), (bb), (bc), (bd), (c), (d), (e) confirmed from search results; Form 26 clause numbers not mapped; the counterpart of s.43(1) 6th proviso not confirmed.")

# ---------------------------------------------------------------- 20 pending
h(2, "20. Pending, unverified and known limitations")
h(3, "20.1 To verify against official texts")
ul([
    "Every line of the three format packs against Schedule III Division I and the two ICAI Guidance Notes (wording, order, sub-classification).",
    "Income-tax Act 2025 section numbers: s.36 (cash payments) and its sub-section; counterpart of s.43(1) 6th proviso; depreciation s.33 vs s.34; ss.185, 186, 188; s.63.",
    "Form 26 clause numbering and content; map the helper workbook to it.",
    "Depreciation tables (Schedule II lives; Income-tax block rates under the 2025 Act and Rules 2026).",
    "Applicability dates of the Non-Corporate Guidance Note (phases).",
    "Standard accounting-policy wording per entity type.",
    "Hindi and Marathi help text and guides (native-speaker review).",
])
h(3, "20.2 Not built")
ul([
    "Ind AS formats (Schedule III Division II/III); XBRL (MCA AOC-4).",
    "ITR schedule export; deferred tax computation (AS 22) — only the policy text exists.",
    "Inventory valuation (stock is taken from the books, not computed); trusts and societies.",
    "Monthly comparison with last year's months (last year's day book is not imported).",
    "Code signing; automatic updates.",
])
h(3, "20.3 Known limitations")
ul([
    "Not yet run on any real client's books or real exports from Tally, Zoho, BUSY, banks, GST portal or TRACES.",
    "Zoho API path untested against a live account; Tally tested against a mock server.",
    "Party matching for GSTR-2B/26AS uses names when GSTIN/TAN is not in the books; similar names need your check.",
    "Bank matching window is 10 days and needs equal amounts; split or combined bank entries stay as open items.",
    "'Points to look at' uses fixed thresholds (25%, 15 points, 2 percentage points, 0.5% materiality); they are a starting point for questions, not conclusions.",
    "The installer has not been tried on a Windows PC by us; GitHub Actions are not running at present.",
    "A lowercase 'ledgercraft' branch still exists on GitHub and clashes with 'LedgerCraft' on Windows; please delete it.",
])

# ---------------------------------------------------------------- 21 changelog
h(2, "21. Change log (every commit)")
log = sh("git -C ledgercraft log --reverse --format='%h|%ad|%s' --date=short").strip().splitlines()
table(["Commit", "Date", "Summary"], [[e(a), e(b), e(c)] for a, b, c in (l.split("|", 2) for l in log)])

# ---------------------------------------------------------------- appendices
h(2, "Appendix A: Truth model (full text)")
parts.append("<pre style='white-space:pre-wrap;font-size:9pt'>" + e(read("TRUTH-MODEL.md")) + "</pre>")
if HIST and HIST.exists():
    h(2, "Appendix B: Your messages (verbatim)")
    p("Long tool instructions and screenshots are left out.")
    msgs = [u for u in hist["users"] if not u.startswith(("Base directory", "[Image", "Stop hook", "[Request interrupted"))]
    parts.append("<ol style='font-size:9pt'>" + "".join(f"<li>{e(u)}</li>" for u in msgs) + "</ol>")

style = """<style>
body{font-family:'Liberation Sans',Arial,sans-serif;font-size:10.5pt;line-height:1.4;color:#1d2420}
h1{font-size:22pt;color:#14402c} h2{font-size:15pt;color:#14402c;border-bottom:1px solid #9fb8aa;padding-bottom:3px;margin-top:22pt}
h3{font-size:12pt;color:#1d5a3e} h4{font-size:10.5pt}
table{border-collapse:collapse;width:100%;font-size:9pt;margin:6pt 0 10pt} th{background:#e3efe8;text-align:left} td,th{vertical-align:top}
code{font-family:'Liberation Mono',monospace;font-size:8.5pt} blockquote{border-left:3px solid #9fb8aa;margin:6pt 0;padding-left:10pt;color:#333}
</style>"""
OUT.mkdir(parents=True, exist_ok=True)
html_path = OUT / "LedgerCraft-Build-Report.html"
html_path.write_text(f"<!doctype html><html><head><meta charset='utf-8'><title>LedgerCraft build report</title>{style}</head><body>{''.join(parts)}</body></html>", encoding="utf-8")
print(html_path)
