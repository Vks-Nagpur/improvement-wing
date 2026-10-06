# Golden book corpus (J01)

One clean, synthetic book per supported entity type. Each `.txt` file holds,
in plain text, what LedgerCraft produces for that book after every ledger
placement is confirmed:

- findings, blockers and warnings
- ledger mapping (ledger, group, head, closing balance, status)
- Balance Sheet, Statement of Profit and Loss, Cash Flow (company), notes
- the tax audit helper workbook, read back from Excel
- the legal items the book relies on, with their status as shipped

`cargo test --test golden` fails when any line changes. After an intended
change, run `UPDATE_GOLDEN=1 cargo test -p lc-io --test golden` and review
the diff before committing it.

## Status

| Axis | Status |
|---|---|
| Real-world validation | **synthetic**: generated books, not real client data |
| Professional review | **not yet reviewed**: the expected output is what the program produced, not what a CA confirmed |
| Legal verification | **unverified**: every legal item is listed as shipped (secondary/draft/unknown) |

A passing golden test means only "nothing changed unexpectedly". It does not
mean the output is correct. A Chartered Accountant should review each file
and record the review (who, date, file hash) here.

## Not covered yet

Manual adjustments, disclosure answers, the fixed asset register and the
audit trail are app-level state; they are covered by `lc-app/tests/api.rs`,
not by these files.

## Points for the reviewer (found while building the corpus)

1. **AOP / BOI members' remuneration and interest** are placed under
   *Other expenses* / *Finance costs*. A firm's partners' remuneration goes to
   the separate *Remuneration to owners* line. Which line the ICAI Guidance
   Note for non-corporate entities expects for AOP/BOI members has not been
   checked against the official text, so the program was not changed.
2. **HUF: salary to Karta** is placed under *Employee benefits expense*.
   Confirm the intended presentation.
3. **Company: Directors' remuneration** is placed under *Other expenses*
   after confirmation. Schedule III disclosure of managerial remuneration
   needs review.
4. **Non-company books** show the blocker "Book depreciation is computed at
   income-tax rates" until the user confirms the policy (Batch 2, by design).
