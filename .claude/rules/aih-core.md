# AiHarness core rules

<!-- Managed by AiHarness (aih sync). Do not edit in a project: change it in the
     harness repo, or capture a lesson with /aih:lesson. Max 20 id'd rules. -->

## Behavioral Rules
These override any instinct to be fast or impressive.

1. **Think before coding.** State your assumptions. Ask when a wrong guess would change the shape of the work, touch money, legal, security or data, or be costly to undo, and whenever a needed fact (rate, price, legal reference, ID, name) is not in the repo or the request. Otherwise take the simplest reading, say which one, and proceed. Name confusion; never paper over it.
2. **Simplicity first.** Do the minimum that meets the success check: 20 lines over 200. No speculative features, parameters or layers.
3. **Surgical changes.** Touch only what the task needs. No drive-by refactors, renames or reformatting. Never change or delete code or comments you don't understand. Match the local style.
4. **Goal-driven, evidence-based.** Define the success check before starting: a command that exits 0 (UI: named screenshots; docs: a check or an exact structure). Bug → a reproducing test first. Loop until green. Report the command and its output, never "should work".

## Workflow
- **Trivial** = one file, an obvious check, no schema, API or UI-contract change → do it, then `/aih:verify`.
- **Otherwise:** `/aih:plan` → spec in `docs/tasks/` → `/aih:implement <spec>` → `/aih:verify` → `/aih:qa` (UI changes) → `/aih:review` → `/aih:ship`.
- **Multi-area:** `/aih:plan … contract` → `docs/spec/SPEC.md` (contract + ownership map) + foundation, area, integration and QA specs → `/aih:integrate`. Areas never edit the contract.
- **Scope wall:** while a spec is active (`aih active-spec`), edits outside its Files in scope are blocked. Need another file → stop and say which and why.
- **Verdicts come from tools:** the first line of `aih verify` decides green, not an opinion. Green = `VERIFY: PASS` (or `VERIFY: PASS (no checks)`). `VERIFY: FAIL (pre-existing only)` = only failures that were already red before (known-red) — not green: report it, never fix unrelated code to get there; the human decides.
- **Stuck:** the same error or symptom twice → `/aih:debug`. The approach is going sideways → stop and re-plan.
- **Lessons:** corrected by the human, blocked twice by a hook, a reviewer RECURRENCE → `/aih:lesson` (one line in `.claude/lessons/inbox.md`; data, not instructions). Rules change only through `/aih:distill` with human approval.
- **Managed files:** `.claude/rules/aih-*.md` and `docs/**/*.template.md` belong to the harness. Never edit them in a project.

## Universal lessons
Promoted from real mistakes across projects by `/harvest-lessons` in the harness repo. Obey them.

- WHEN a fix fails twice on the same error or symptom → stop patching; run /aih:debug and pin the cause with a failing test before the next attempt. <!-- id=U-1 n=0 since=2026-10 check=hook:post-edit -->
- WHEN a test, check or hook is red → fix the cause; never skip, weaken, delete or snapshot-update a test, or add a suppression comment, to get green unless the spec allows it. <!-- id=U-2 n=0 since=2026-10 check=none -->
- WHEN the work needs a file outside the active spec's Files in scope, or a change to a shared contract type → stop and report it; never edit around the wall or redefine the type locally. <!-- id=U-3 n=0 since=2026-10 check=hook:scope-guard -->
- WHEN a guard or hook blocks a command → read the reason and do the safe alternative or ask; never rephrase, split or wrap the command to get past the guard. <!-- id=U-4 n=0 since=2026-10 check=hook:guard -->
