# LedgerCraft – Accounting Truth Model

LedgerCraft is a **financial-statement compiler**: accounting data and
disclosure data go in; deterministic, traceable statements come out. This
document fixes what the program may decide by itself, what needs source
evidence, what needs professional judgement, and what blocks a final copy.
The code follows this document; where they differ, the code is wrong.

## 1. Two kinds of input

| Accounting data (from the books) | Disclosure data (from the entity / the CA) |
|---|---|
| Trial balance, ledgers and groups | Share capital particulars, shareholders, promoters |
| Vouchers (day book) | Contingent liabilities and commitments |
| Last year's trial balance | Related parties and their transactions |
| Fixed asset register | MSME status of suppliers and interest under the MSMED Act |
| | Accounting policy wording, other notes |
| | Manual adjustments (journal entries passed by the CA) |

Disclosure data can never be inferred from a trial balance. Its absence is
never treated as "nil".

## 2. Three states for every disclosure

| State | Meaning | Printed as |
|---|---|---|
| **Not answered** (default) | Nobody has said anything yet | Draft: a visible "Information not yet provided" line. Final: **blocked** |
| **Explicitly nil** | The user confirmed there is nothing to disclose | A nil statement ("There are no contingent liabilities…") |
| **Entered** | Particulars given | The particulars |

Required sections: share capital (companies), contingent liabilities and
commitments, related parties, MSME dues (all entities).

## 3. What LedgerCraft decides by itself (deterministic)

* Arithmetic: trial-balance totals, opening = last year's closing, voucher
  totals = ledger movement, statements balance, rounding that casts exactly.
* Placement of a ledger whose standard group has **one** possible line in
  the format (status **Rule**): Capital, Reserves, Sundry Debtors, Sundry
  Creditors, Cash, Bank, Bank OD, Duties and Taxes, Stock-in-hand, Sales,
  Purchases, Indirect Incomes, Direct and Indirect Expenses, Profit & Loss A/c.
* Cash-limit and cash-loan tests (they test facts in the vouchers; whether a
  penalty or disallowance finally applies is the CA's judgement, so they are
  reported as "Check", never decided).
* Cash flow statement (indirect method) from two balance sheets and the P&L.
* Depreciation from the fixed asset register using the chosen basis.

## 4. What needs confirmation (status **Suggested**)

LedgerCraft proposes; the user confirms. Until confirmed, a final copy is
locked.

* A group with more than one possible line: loans (long-term vs short-term),
  loans and advances and deposits (non-current vs current), investments,
  provisions, fixed assets (PPE / intangibles / work in progress), current
  liabilities, current assets, direct incomes, suspense, branch accounts.
* Placement found from the ledger **name** (salary, interest, depreciation,
  partners' remuneration …).
* A ledger moved to the other side because of its balance (debtor in credit,
  creditor in debit, bank overdrawn).
* Any AI suggestion.

## 5. When a remembered choice is re-opened (status **Review**)

A choice the user made for a client is reused every year, but it is
re-opened when the context changed:

* the ledger's group in the books is different from when it was confirmed;
* the balance changed side (Dr ↔ Cr).

## 6. What is always professional judgement (never automated)

* Whether a disclosure is required beyond the format's standard lines.
* Classification as current / non-current where the books do not say.
* Whether a cash payment falls in an exception (Rule 6DD and similar).
* Whether an unresolved difference is acceptable.
* Accounting policy wording and the audit opinion.

The local AI may explain and suggest in these areas; it never decides, never
changes a figure and never confirms a mapping. Every AI answer is labelled and
recorded.

## 7. What blocks a final (signing) copy

Draft copies are always allowed and say on the first page how many problems
are unresolved. A final copy is refused while any of these is true:

1. A "Must fix" finding is open (trial balance does not tally, unknown group,
   unmapped ledger, statements do not balance, …).
2. A ledger's placement is **Suggested** or **Review** and not confirmed.
3. A required disclosure is **Not answered**.
4. Share capital entered does not agree with the books (companies).

## 8. Rules and formats are versioned and pinned

* Each rule carries: its id, the law or standard it comes from (authority,
  document, section), effective-from and effective-until dates, the entity
  types it applies to, and a verification status (`verified` against the
  official text, or `unverified`).
* Each client year is **pinned** to the rule pack and format pack it started
  with. A newer pack is never applied silently: LedgerCraft shows what changed
  and the user chooses to move the year to it. The move is recorded in the
  audit trail. Earlier exports can therefore always be reproduced.
* Every export records the pack versions it used.

## 9. Audit trail

The audit trail is **tamper-evident**, not tamper-proof: every event is sealed
with the fingerprint of the one before, so editing or removing an event in the
middle is detected. Someone with full control of the files could still delete
the whole history; export the trail with the statements to keep an outside
copy.

## 10. Status of the legal content

Format packs and several legal references are marked `unverified` until each
line is compared with the official text (see DESIGN.md §7). Verification is
recorded in the pack, not in this document.
