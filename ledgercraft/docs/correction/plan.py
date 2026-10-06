import html
e = html.escape
P = []
def h(n, t): P.append(f"<h{n}>{e(t)}</h{n}>")
def p(t): P.append(f"<p>{t}</p>")
def ul(xs): P.append("<ul>" + "".join(f"<li>{x}</li>" for x in xs) + "</ul>")
def table(hd, rows):
    P.append('<table border="1" cellspacing="0" cellpadding="3"><thead><tr>' + "".join(f"<th>{e(x)}</th>" for x in hd) + "</tr></thead><tbody>" + "".join("<tr>" + "".join(f"<td>{c}</td>" for c in r) + "</tr>" for r in rows) + "</tbody></table>")

C = {"CONFIRMED": "#a3262a", "PARTLY CONFIRMED": "#9a5b00", "ALREADY FIXED": "#1d6b46", "NEEDS LEGAL VERIFICATION": "#5b3a9a"}
def st(s): return " + ".join(f'<b style="color:{C.get(x.strip(), "#333")}">{e(x.strip())}</b>' for x in s.split("+"))

P.append("<h1>LedgerCraft: response to the Master Correction Specification</h1>")
p("<b>Stage:</b> plan only (section M of the specification: 'Before coding, return the implementation plan'). No code has been changed for this specification. Repository state inspected: branch <code>LedgerCraft</code>, commit <code>02b3dc8</code>.")
p("<b>Method:</b> every issue was checked against the source (file, function and line quoted) and classified CONFIRMED, PARTLY CONFIRMED, ALREADY FIXED, or NEEDS LEGAL VERIFICATION.")

h(2, "1. Facts that shape this plan")
ul([
    "<b>Official legal sources cannot be reached from this build environment.</b> Tested today: incometaxindia.gov.in, mca.gov.in, icai.org, resource.cdn.icai.org, indiacode.nic.in, egazette.gov.in and cbic.gov.in all fail to connect. I therefore <b>cannot</b> perform primary-source verification of any legal item. I will build the machinery for it and keep every item unverified; verification must be recorded by a person (you or a CA you appoint) from the official text.",
    "16 of 40 rules are marked 'unverified'; the other 24 are internal accounting logic (arithmetic, integrity). All three format packs and the depreciation pack are marked draft.",
    "Consequence of the specification as written: once the legal-readiness gate (LC-P0-001 / P0-013) is in place, <b>no final signing copy can be produced for any client</b> until the applicable items have been verified and recorded. Drafts remain available. This is a product decision for you (Decision D1 below).",
    "Real-world validation needs real (anonymised) books and exports from you; Windows VM, Microsoft Word and code-signing work need a Windows PC and a certificate. These are listed as items I cannot complete here.",
])

h(2, "2. Decisions needed from you before coding")
table(["#", "Decision", "Options", "My recommendation"], [
    ["D1", "Signing while legal items are unverified", "(a) Block final copy until every applicable item is verified (spec preference). (b) Allow a final copy labelled 'non-authoritative: legal content not verified' on every page.", "(a) Block. Drafts stay available and say exactly which items are pending."],
    ["D2", "Who records legal verification, and where", "(a) An in-app 'Legal verification register' (expert screen): a person records authority, document, provision, official URL/identifier, effective dates, verified on, verified by. Stored outside the client folders; each record is bound to a hash of the exact item text, so any later change to the item voids the verification. (b) Verification only by editing pack files in the repository.", "(a), with (b) also possible for shipping pre-verified packs later. The app will never mark anything verified by itself."],
    ["D3", "Client folder IDs (LC-P1-009)", "Move to a stable internal ID with the display name kept separately, migrating existing folders (with a backup copy first).", "Yes, with backup and a one-time migration log entry."],
    ["D4", "Order of work", "Batches 1 to 9 below, P0 first.", "As listed; each batch ends with the full test suite and a commit."],
])

h(2, "3. Issue-by-issue findings")
rows = [
 ["LC-P0-001", "Legal packs draft/unverified", "CONFIRMED + NEEDS LEGAL VERIFICATION", "packs/rules.json (version 2026.10-draft; 16 rules 'unverified'); company_div1.json, llp.json, non_corporate.json ('status: draft'); depreciation.json ('verify against official text'). Provenance today is per rule (authority, document, section, verification) but has no official URL, verified_on, verified_by or content hash; format lines and depreciation rows have none.", "Item-level provenance for every rule, format line, depreciation row, section mapping and tax-audit clause; verification overlay (D2); readiness function; signing gate (P0-013)."],
 ["LC-P0-002", "Form 26 is a renamed 3CD helper", "CONFIRMED", "lc-io/src/tax_audit.rs: comment 'Form 26 clause numbers are not yet mapped; the sheets carry the Form 3CD subject'; same sheet builders used for both forms, only file name and labels change.", "TaxAuditSchema abstraction: schemas/form3cd.json and schemas/form26.json (Form 26 starts with zero mapped clauses). Helper for Form 26 periods titled 'Form 26 helper, not mapped'; 3CD clause numbers never shown as Form 26 clauses."],
 ["LC-P0-003", "New-Act section numbers hardcoded", "CONFIRMED + NEEDS LEGAL VERIFICATION", "rules.json old_ref/new_ref strings; app.js TAX_ITEMS (s.36, s.186, s.185, s.188, 'actual-cost rule'); tax_audit.rs sheet titles; depreciation s.33 vs s.34 conflict noted only in docs.", "One data file packs/sections.json (old provision, new provision, status, source). Unresolved entries render as 'Statutory reference pending primary-source verification'. All UI/report/workbook text reads from it. Effective-date tests."],
 ["LC-P0-004", "Company 'Managerial remuneration' uses PARTNERS_REMUNERATION head", "CONFIRMED", "company_div1.json lines 210 and 226 use head PARTNERS_REMUNERATION.", "New canonical head MANAGERIAL_REMUNERATION for companies; mapping by entity type; ratios, cash flow and capital note updated; whether the line belongs on the face or in notes goes to the legal verification list (not assumed)."],
 ["LC-P0-005", "Formats share one generic skeleton", "CONFIRMED (design)", "The three packs differ mainly in labels over the same internal heads.", "Staged: (1) separate presentation schema per entity with its own line provenance and semantic heads (Batch 3); (2) richer canonical facts with dimensions (LC-P1-023) after correctness batches. Full rewrite avoided."],
 ["LC-P0-006", "Cash wording too conclusive", "CONFIRMED", "checks/cash.rs line 105 and 162: '...attracts penalty'; line 149: 'Exclude this amount from the asset's cost for tax depreciation.'", "Detection wording ('Potential threshold event detected; review scope, aggregation, exceptions and facts'). New finding fields: detection_basis, unknown_facts, possible_exceptions, legal_source, verification_status, professional_review_required."],
 ["LC-P0-007", "Loan checks encode legal interpretation; 3CD wording in Form 26 periods", "CONFIRMED + NEEDS LEGAL VERIFICATION", "checks/loans.rs lines 125, 147: 'Report in Form 3CD clause 31(a)/(b)' emitted regardless of year.", "Regime-aware text from the tax-audit schema; predicates (acceptance, repayment, principal vs interest/TDS, journal, exemptions) moved to versioned rule data with status; each listed for verification."],
 ["LC-P0-008", "Depreciation pack unverified", "CONFIRMED + NEEDS LEGAL VERIFICATION", "depreciation.json status text; 13 book classes and 12 tax blocks have no per-row source or effective period.", "Per-row provenance and effective periods; table chosen by period; regression test per row once verified."],
 ["LC-P0-009", "Income-tax rates as book basis", "CONFIRMED", "far.rs: BookBasis::IncomeTaxRates (line 26) and used as a default in places (line 361).", "Option kept but gated: explicit confirmation, shown in policies/FAR/statements, warning when it conflicts with the framework (once applicability is verified)."],
 ["LC-P0-010", "Cash flow without applicability", "CONFIRMED", "Cash flow is a plain on/off option.", "Applicability result REQUIRED / NOT_REQUIRED / VOLUNTARY / REQUIRES_REVIEW / NOT_SUPPORTED with reason, source and period. Until rules are verified the result is REQUIRES_REVIEW; the UI never says 'required'."],
 ["LC-P0-011", "Cash flow from balances and name heuristics", "CONFIRMED", "report/cashflow.rs line 18-23: interest income by ledger name containing 'interest'; PPE purchases from balance movement plus depreciation; tax paid = tax expense.", "When a day book exists, derive from cash/bank transactions classified by the counter-ledger; otherwise balance method marked 'derived/estimated'. Tax paid from tax asset/provision movements; finance cost paid separate from expense; non-cash items flagged for review. Golden tests with known answers."],
 ["LC-P0-012", "Ratio heuristics", "CONFIRMED + NEEDS LEGAL VERIFICATION", "ratios.rs lines 86-90: investment income by names (dividend, mutual fund, ' fd'); COGS simplified.", "Formula provenance per ratio; numerator/denominator drill-down in UI; heuristic components flagged 'review'; zero/negative denominator and sign tests; 25% test unchanged in logic, formula to be verified."],
 ["LC-P0-013", "Signing only checks accounting blockers", "CONFIRMED", "lc-io/src/export.rs lines 118-124: Signing refused only on 'printable' and report blockers.", "LegalContentReadiness gate (depends on P0-001, P0-002, P0-008). Per D1."],
 ["LC-P1-001", "Weak session token", "CONFIRMED", "server.rs line 169: RandomState + time + PID.", "OS random (getrandom), 256 bits, constant-time comparison, never logged."],
 ["LC-P1-002", "Token accepted in URL", "CONFIRMED", "server.rs: token_ok accepts url.contains('t=token').", "Header only for /api. Preview/download links that need a URL get a short-lived single-use ticket instead. Referrer-Policy: no-referrer."],
 ["LC-P1-003", "1 GiB body limit for every request", "CONFIRMED", "server.rs line 1248: take(1 << 30).", "Per-endpoint limits (JSON 2 MB; uploads 200 MB, streamed to a temp file); 413 on excess; tests with chunked bodies."],
 ["LC-P1-004", "Origin/CSRF/DNS-rebinding", "PARTLY CONFIRMED", "Binds 127.0.0.1; Host checked with starts_with ('127.0.0.1:' / 'localhost:'); token required; nosniff already sent (line 1287).", "Exact Host parse with port match; Origin check on POST; CSP, frame-ancestors none, Referrer-Policy; hostile Host/Origin tests."],
 ["LC-P1-005", "open-folder opens any directory", "CONFIRMED", "server.rs open_folder(path) line 132: any existing directory.", "Canonicalise; allow only the data folder and its sub-folders; audit the action."],
 ["LC-P1-006", "PID-based temp file", "CONFIRMED", "store.rs write_atomic line 41.", "Unique temp name per call, same directory; fsync file and (Unix) directory; Windows replace via rename-over with retry; crash tests."],
 ["LC-P1-007", "Audit log truncation", "PARTLY CONFIRMED", "Chain detects edits in the middle; whole-file deletion or truncation at the end is not detected.", "Head hash and count stored in a checkpoint file and in every export manifest; warn if log is shorter than the checkpoint or missing; never start a fresh chain silently."],
 ["LC-P1-008", "audit() drops bad lines", "CONFIRMED", "store.rs line 366: filter_map(...ok()).", "audit() returns events plus corruption status; UI shows 'chain broken' with line numbers."],
 ["LC-P1-009", "Slug collisions", "CONFIRMED", "store.rs slug(): different names can give the same folder.", "Stable internal ID per client (D3), migration with backup, collision test."],
 ["LC-P1-010", "Adversarial import tests", "CONFIRMED (gap)", "Readers use calamine 0.26; no adversarial suite.", "Fixture suite: zip bombs, malformed workbooks, huge sheets, duplicate headers, formulas, hidden sheets, odd Unicode, bad dates, extreme values, malformed/deep XML, bad encodings, duplicate vouchers, CSV formula strings, broken zip."],
 ["LC-P1-011", "Tally parser limits", "CONFIRMED", "lc-io/src/tally.rs has no size/depth/node limits.", "Limits on bytes, depth, nodes, ledgers, vouchers; graceful errors; property/fuzz tests."],
 ["LC-P1-012", "Formula injection", "PARTLY CONFIRMED", "Excel outputs use write_string (stored as text, not formulas). CSV is written only for practice data; no CSV user export.", "Regression tests with '=cmd|...' style names in every Excel output; prefix-quote in any CSV writer."],
 ["LC-P1-013", "HTML output escaping", "PARTLY CONFIRMED", "render/html.rs uses esc() 21 times; not audited exhaustively.", "Audit every interpolation; XSS fixture test on the preview."],
 ["LC-P1-014", "innerHTML in UI", "PARTLY CONFIRMED", "app.js has 64 innerHTML assignments; esc() used for user text, not proven everywhere.", "Inventory and classify all 64; replace risky ones; browser test with payloads in entity, ledger, narration, signatory and disclosure text."],
 ["LC-P1-015", "Cache invalidation", "PARTLY CONFIRMED", "server.rs keeps an analysis cache per client; 15 explicit invalidate() calls; a missed call would serve stale figures.", "Key the cache by a hash of everything that affects analysis (inputs, settings, mapping, tags, adjustments, disclosures, period, branches, FAR, pins). Mutation-by-mutation tests."],
 ["LC-P1-016", "GSTR-2B/26AS name matching", "CONFIRMED", "recon.rs line 109: name score >= 0.6 accepted.", "Identifier match = deterministic; exact name = suggested; fuzzy = review only with score and competing candidates; confirmations stored."],
 ["LC-P1-017", "Greedy matching", "CONFIRMED", "recon.rs picks best unmatched portal party in order.", "Tie/near-tie detection; global assignment for suggestions; confirmation for non-identifier matches; similar-name tests."],
 ["LC-P1-018", "Bank matching too limited", "CONFIRMED", "bankrec.rs: same amount, date window (10 days), cheque number.", "Tier 2 reference/UTR; Tier 3 one-to-many and many-to-one within limits (suggested only); Tier 4 manual matching; nothing ambiguous auto-confirmed."],
 ["LC-P1-019", "Zero difference treated as proof", "PARTLY CONFIRMED", "Unmatched lists exist; no method per match, stale items, duplicates, opening balance check.", "Add those, plus stored confirmations."],
 ["LC-P1-020", "Mock import tests not enough", "CONFIRMED", "As stated in the report.", "Needs real anonymised exports from you; status stays PARTLY until then. I will write the anonymisation procedure and fixture format."],
 ["LC-P1-021", "Import provenance", "PARTLY CONFIRMED", "Upload SHA-256 computed (server.rs line 421); no parser version, counts, warnings, rejected rows stored.", "Import record per file with all listed fields; shown on Import screen; included in manifest."],
 ["LC-P1-022", "Silent row drops", "PARTLY CONFIRMED", "Readers skip total/blank rows by design; skipped rows are not reported.", "Every reader returns accepted / skipped-with-reason / review rows; import diagnostic report."],
 ["LC-P1-023", "Canonical fact granularity", "CONFIRMED (design)", "Heads in mapping.rs; dimensions only partly (MSME tag, ageing).", "Typed dimensions (current/non-current, related party, secured, MSME, disputed, ageing, nature, counterparty, tax character, cash/non-cash, branch). After Batch 3."],
 ["LC-P1-024", "Current/non-current inference", "CONFIRMED", "Split inferred from group/name.", "Classification evidence; Suggested state; remembered confirmation; Review when context changes."],
 ["LC-P1-025", "Ageing basis", "CONFIRMED", "ageing.rs: FIFO, aged from bill (transaction) date; due dates not used.", "Disclose the method in the note and dashboard; warning that it is not due-date ageing; due-date ageing when bill-wise due dates are imported."],
 ["LC-P1-026", "Inventory is not valuation", "CONFIRMED (wording)", "Stock taken from books.", "Label 'book balance'; remove any wording implying valuation."],
 ["LC-P1-027", "Deferred tax wording", "CONFIRMED", "report/policies.rs line 59: 'Deferred tax is recognised ... in accordance with AS 22' though nothing is computed.", "Policy shown only when a deferred tax figure exists, otherwise a marked placeholder for the preparer."],
 ["LC-P1-028", "Ind AS / XBRL not selectable", "ALREADY FIXED", "No such options in the UI or entity list.", "Add an explicit 'Not supported' line where formats are listed."],
 ["LC-P2-001", "Four-axis status", "CONFIRMED", "Report uses one status tag.", "Implementation / tests / legal verification / real-world validation, in the app's About screen and the generated report."],
 ["LC-P2-002", "Why a rule fired", "PARTLY CONFIRMED", "Findings carry ledger, voucher, date, amount and a suggestion; not threshold, aggregation, period, legal status, unknown facts.", "Add the missing fields (shared with P0-006) and show them under each finding."],
 ["LC-P2-003", "Severity names", "PARTLY CONFIRMED", "Three levels: Must fix / Check / Note. Cash and loan items are 'Check' (warning), not blockers.", "Five categories: Blocker (objective), Review (judgement), Warning, Info, Unverified law."],
 ["LC-P2-004", "Self-correction", "ALREADY FIXED", "No silent changes exist; every change is a user action (truth model).", "Keep; add a test that analysis never mutates inputs."],
 ["LC-P2-005", "Final copy wording", "PARTLY CONFIRMED", "'Final signing copy' used when accounting blockers close.", "Use only when all five conditions hold; add 'This is not an audit report' wording."],
 ["LC-P2-006", "UDIN", "ALREADY FIXED", "Paste-only, no validation, no authentication claim.", "Keep."],
 ["LC-P2-007", "AI privacy", "PARTLY CONFIRMED", "Default endpoint 127.0.0.1:11434, but --ollama accepts any URL silently (main.rs line 48).", "Show endpoint and what is sent; refuse non-local endpoints unless explicitly confirmed; switch to disable."],
 ["LC-P2-008", "Manifest content", "CONFIRMED", "export-manifest.json holds file hashes only.", "Add all listed items (inputs, mapping, adjustments, disclosures, packs, app version, audit head, mode, time, readiness)."],
 ["LC-P2-009", "Manifest attestation", "CONFIRMED", "", "Canonical JSON plus manifest.sha256."],
 ["LC-P2-010", "Draft marking in all formats", "PARTLY CONFIRMED", "PDF: DRAFT watermark (report.typ line 52); Excel: 'DRAFT' header; Word: footer 'Draft – for discussion only'; HTML: draft banner.", "Add a visible DRAFT heading/watermark in Word and HTML body and Excel first rows; tests per format."],
 ["LC-P2-011", "Word in Microsoft Word", "CONFIRMED (gap)", "Checked in LibreOffice only.", "Structural golden tests here; Microsoft Word check needs a Windows PC (you)."],
 ["LC-P2-012", "Excel print setup", "PARTLY CONFIRMED", "Not reviewed.", "Print areas, fit-to-width, repeated headers; tests."],
 ["LC-P2-013", "Installer validation", "CONFIRMED (gap)", "Not run on Windows.", "Test checklist written; execution needs Windows 10/11 machines."],
 ["LC-P2-014", "Code signing", "CONFIRMED (gap)", "Unsigned.", "No promise of a provider; document options and costs."],
 ["LC-P2-015", "CI", "PARTLY CONFIRMED", "Workflow exists (fmt, clippy, tests, bench, Windows build, installer) but has not run since 3 October.", "Add pack schema validation, readiness report, artifact hashes, cargo audit; you enable Actions."],
 ["LC-P2-016", "Case-clashing branch", "CONFIRMED", "Lowercase 'ledgercraft' branch exists; my deletion attempt was refused by GitHub permissions.", "You delete it on GitHub (Branches page)."],
]
table(["ID", "Issue", "Classification", "Evidence in source", "Planned change"], [[r[0], e(r[1]), st(r[2]), e(r[3]), e(r[4])] for r in rows])

h(2, "4. Dependencies between issues")
ul([
    "P0-013 (signing gate) depends on P0-001 (item provenance), P0-002 (tax-audit schema), P0-003 (section data), P0-008 (depreciation rows).",
    "P0-006, P0-007 and P2-002 share the new finding fields; P2-003 severities change UI and tests together.",
    "P0-004 needs the semantic-head change before P0-005 schemas are split.",
    "P0-010 applicability and P0-009 basis gating both need the applicability engine (Batch 2).",
    "P1-015 cache key and P2-008 manifest share one 'inputs fingerprint'.",
    "P1-009 ID migration must run before any other storage format change (Batch 5 starts with it).",
])

h(2, "5. Implementation batches")
table(["Batch", "Issues", "Main files", "Tests added"], [
    ["1. Legal provenance and readiness", "P0-001, P0-003, P0-013, P0-006, P0-007 (wording + fields), P2-001, P2-002, P2-003, P2-005, P1-027, P1-028", "new lc-core/src/legal.rs; packs/sections.json; rules.json schema; checks/cash.rs, loans.rs; rules.rs (Severity); export.rs; report/policies.rs; app UI (Legal verification register, readiness on Sign and export and dashboard)", "every applicable item enumerated per entity and year; signing blocked with any unverified item; verification voided when item text changes; no 'Form 3CD' text in Form 26 periods; unresolved section shows neutral text"],
    ["2. Accounting correctness", "P0-004, P0-009, P0-010, P0-011, P0-012, P1-025, P1-026", "mapping.rs (MANAGERIAL_REMUNERATION), packs, far.rs, report/cashflow.rs, ratios.rs, ageing.rs, new applicability.rs", "golden cash-flow books with known answers; ratio zero/negative/sign tests; basis confirmation required; ageing method disclosed"],
    ["3. Schemas", "P0-002, P0-005 (stage 1)", "lc-io/src/tax_audit.rs → schema-driven; schemas/form3cd.json, form26.json; per-entity presentation schemas with line provenance", "schema chosen by period; no 3CD number as Form 26 clause; each entity pack enumerable line by line"],
    ["4. Security", "P1-001 to P1-005, P1-012 to P1-014, P2-007", "lc-app/src/server.rs, main.rs, app.js, render/html.rs", "token randomness and header-only; 413; hostile Host/Origin; folder outside data refused; XSS payload browser test; formula strings in Excel"],
    ["5. Storage and audit", "P1-009 (migration), P1-006, P1-007, P1-008, P1-015, P2-008, P2-009", "lc-app/src/store.rs, server.rs, lc-io/src/export.rs", "slug collision; atomic write crash simulation; truncation detected; corrupt line reported; cache key per mutation; manifest fields"],
    ["6. Imports", "P1-010, P1-011, P1-021, P1-022", "lc-io read.rs, table.rs, tally.rs, portal.rs, zoho.rs; store import records", "adversarial fixture suite; parser limits; diagnostic report"],
    ["7. Reconciliation", "P1-016 to P1-019", "lc-core/src/recon.rs, bankrec.rs; UI", "identifier vs name tiers; ties; one-to-many suggestions never auto-confirmed; duplicate statement lines"],
    ["8. Outputs and release", "P2-010, P2-011 (structural), P2-012, P2-015", "renderers, CI workflow", "draft marking per format; Excel page setup; CI pack validation"],
    ["9. Test corpus and report", "J01 to J05, P1-023/024 groundwork; new four-axis build report", "lc-testdata; docs/report", "golden books for all 7 entity types; adversarial books; property tests; performance metrics"],
])

h(2, "6. Schema and migration changes")
ul([
    "rules.json, format packs, depreciation.json: new per-item provenance fields; pack version bumped. Pinned client years keep their old pack (reproducible); migration only by explicit user action with a diff and audit entry.",
    "New packs/sections.json and schemas/form3cd.json, schemas/form26.json.",
    "New user-level file legal-verifications.json (outside client folders), each record bound to an item id and content hash.",
    "Client folders: stable internal ID (D3) with a backup copy before migration and an audit entry; old paths still readable.",
    "Settings: book-depreciation basis confirmation flag; cash-flow applicability override (voluntary).",
    "Finding JSON gains fields; old saved analyses are recomputed, not migrated.",
    "Severity gains Review and Unverified-law categories; mapping of old to new is fixed and tested.",
])

h(2, "7. Items requiring official legal verification (by a person, from primary text)")
table(["Area", "Items"], [
    ["Income-tax Act 2025 / Rules 2026", "s.36 (cash payments) and sub-section; counterpart of old s.43(1) 6th proviso; depreciation section (33 or 34) and rate table; ss.185, 186, 188 (loans, receipts, repayments) including exceptions; s.63 and Rule 47; Form 26 clauses and fields"],
    ["Income-tax Act 1961 (legacy years)", "ss.40A(3), 43(1) proviso, 269SS, 269ST, 269T, Rule 6DD; Form 3CD clauses 18, 21(d), 26, 31(a)-(e), 34, 40, 44"],
    ["Companies Act 2013", "Schedule III Division I every line used, general instructions on rounding, ageing and ratio disclosures; Schedule II useful lives and residual value; cash-flow exemption (s.2(40)); managerial remuneration placement"],
    ["ICAI", "Guidance Note on Division I; Guidance Note on Non-Corporate Entities (every line) and its applicability announcement; Guidance Note on LLPs (every line); AS 3, AS 22 references"],
    ["Other", "Companies (Accounts) Rules rule 3(1) proviso; MSMED Act s.22 items; ratio formulas"],
])
p("For each, the register will hold: authority, document title, provision/paragraph/table, official source identifier or URL, effective from/until, verified on, verified by, and the exact text it was compared with.")

h(2, "8. Items I will intentionally not change")
ul([
    "Integer-paise money; immutable imports; overlay adjustments; mapping states; Given/Nil/Unknown disclosures; final-copy blockers; pinned packs; AI outside the trust boundary; hash-chained audit trail (hardened, not replaced); Rust workspace layout.",
    "No new scope: Ind AS, XBRL, ITR export, deferred tax computation, inventory valuation, auto-updater, more AI features.",
    "No legal item marked verified by me.",
])

h(2, "9. What I cannot complete in this environment")
ul([
    "Primary-source legal verification (official sites unreachable; and the spec requires a human verifier record).",
    "Real-world validation with real Tally, BUSY, Zoho, bank, GST and TRACES files (needs anonymised samples from you).",
    "Windows 10/11 installer matrix, Microsoft Word check, SmartScreen behaviour (needs Windows PCs).",
    "Code signing (needs a certificate and a decision).",
    "Deleting the lowercase GitHub branch and enabling GitHub Actions (needs your GitHub access).",
])

h(2, "10. After coding, you will receive")
ul(["complete diff summary per batch", "tests run and results, unresolved failures", "legal items still unverified", "migrations performed", "screenshots of changed screens", "a new source-generated report with the four status axes", "updated verification register and known limitations", "no status upgraded without evidence"])

style = "<style>body{font-family:'Liberation Sans',Arial,sans-serif;font-size:10pt;line-height:1.35;color:#1d2420}h1{font-size:19pt;color:#14402c}h2{font-size:14pt;color:#14402c;border-bottom:1px solid #9fb8aa;margin-top:18pt}table{border-collapse:collapse;width:100%;font-size:8.5pt;margin:6pt 0}th{background:#e3efe8;text-align:left}td,th{vertical-align:top}code{font-size:8.5pt}</style>"
import pathlib
out = pathlib.Path(__file__).parent / "LedgerCraft-Correction-Plan.html"
out.write_text(f"<!doctype html><html><head><meta charset='utf-8'><title>LedgerCraft correction plan</title>{style}</head><body>{''.join(P)}</body></html>", encoding="utf-8")
print(out)
