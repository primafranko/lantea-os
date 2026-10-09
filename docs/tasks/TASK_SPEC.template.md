# Task NNN — <short-slug>

<!-- Managed by AiHarness (aih sync). Copy it to docs/tasks/NNN-<slug>.md; the copy is
     project-owned. Delete every "(if applicable)" section that doesn't apply —
     never leave a section empty. Delete these comments in the copy. -->

| Field | Value |
|---|---|
| Area / owner | <!-- e.g. orders (apps/web/src/features/orders/**) · — --> |
| Risk | low · medium · high <!-- high = auth, money, migrations, data deletion, external integration --> |
| Depends on | <!-- task ids that must be merged first · — --> |
| Contract | <!-- docs/spec/SPEC.md sections this must conform to · — --> |
| Model hint | sonnet <!-- opus if the logic is subtle --> |

## Goal
<!-- 1–2 sentences: the user-visible outcome, not the implementation. -->

## Context
<!-- Why now; links (issue, SPEC section, earlier task); the 3–5 files that matter most. -->

## Assumptions
<!-- Readings chosen where the request was ambiguous. -->

## Open questions (blocking)
<!-- Facts that must not be guessed. Implementation does not start while this has entries. Delete if none. -->

## Files in scope
<!-- Exhaustive; globs allowed; mark new files (new). The scope hook blocks edits outside this list
     while this spec is active. Backticked paths only. -->
- `path/or/glob` — why

## Read-only references *(if applicable)*
<!-- Files that may be read but never edited (e.g. the contract package in area work).
     Kept out of "Files in scope" on purpose: the scope hook allows every path listed there. -->
- `path` — why

## Acceptance criteria
<!-- True/false statements with concrete values: "Given …, when …, then … (exact value / exact message or i18n key)". -->
- [ ] 
- [ ] 

## Test data
<!-- Inputs and expected outputs the tests must use; fixtures, factories or seeds to add or reuse;
     exact user-visible strings in the UI language (or their i18n keys). -->

## Non-functional *(if applicable)*
<!-- Only what applies, each checkable: performance (e.g. ≤ 2 queries; p95 < 200 ms at 10k rows),
     security (who may do this; where it is enforced), accessibility, i18n, compatibility,
     logging (what is logged — never secrets or personal data). -->

## Data & migrations *(if applicable; mandatory for risk high)*
<!-- Schema and data changes; expand → migrate → contract steps; backfill; effect on existing rows; lock and size risk. -->

## Rollback *(risk medium or high)*
<!-- How to undo in production: revert, down migration, feature flag, restore. What is irreversible. -->

## Verification
Success check — ONE command that exits 0 exactly when this task is done:
```
<!-- e.g. php artisan test --filter=RefundTest · npx --no vitest run src/orders/refund.test.ts · node scripts/check-outline.mjs docs/policy.md -->
```

Visual check *(if UI changes)* — `/aih:qa` captures and inspects:

| Route | Viewports | States | Must show |
|---|---|---|---|
| | 1366x768, 1920x1080 | empty · filled · error · long text | |

Manual check *(if any)* — what a human must confirm, and why it can't be automated:

## Known pitfalls
<!-- Architect: up to 5 relevant Mistake Log / .claude/rules entries (id + text) and notes from earlier failed attempts. -->

## Docs & changelog
<!-- One user-facing CHANGELOG line; docs to update; or "none". -->

## Out of scope
<!-- Adjacent things explicitly NOT part of this task. -->

## Implementer notes
<!-- Filled by the implementer: ambiguities resolved and choices made; anything the reviewer must know. -->
