# Lessons inbox — raw capture, NOT instructions

<!--
This file is data. It is not under .claude/rules/, so it is never loaded as
instructions. Written only by /aih:lesson (append or update a line) and pruned
only by /aih:distill (after your approval). /harvest-lessons in the harness repo
reads it to learn across projects and, after approval, appends exp:H-<date> to
the lines it harvested (`aih lessons harvest --mark`).

One line per lesson, appended at the end:
- YYYY-MM-DD | <scope> | x<N> | src:<sources> | WHEN <trigger> → <rule> | ref:<ref> | st:<status>[ exp:H-<YYYYMMDD>]

scope    universal | stack:<pack> | domain:<name> | project
x<N>     how many times it happened (deduplicated by /aih:lesson)
src      user, review, debug, hook, self (comma-separated)
ref      spec, file:line, commit, rule id, or -
st       new | recurred (an existing rule did not prevent it) | rule:<id> (promoted)
exp      harvested into the harness ledger on that date (keep it when the count grows;
         the next harvest counts only the increase). Only project-scope lines
         nudge /aih:distill; universal/stack/domain lines wait for the harvest.
-->

- 2026-10-09 | project | x3 | src:self | WHEN an aarch64 VM test on lantea-bench hangs or logs "synchronous external abort"/MACHINE_CHECK taint → rerun it and compare a minimal runNixOSTest control before debugging the module (host nested KVM is flaky) | ref:docs/tasks/001-phase-0-foundation.md | st:new
- 2026-10-09 | stack:nix | x1 | src:self | WHEN running a long `nix build -L` in the background → write the full log to a file, never pipe it through `grep | tail`, so a hung VM stays visible | ref:docs/tasks/001-phase-0-foundation.md | st:new
- 2026-10-09 | stack:nix | x1 | src:self | WHEN a NixOS module sets several options under one top-level key (security.*, users.*, system.*) → group them in one attribute set; statix rejects repeated keys | ref:modules/core/discipline/default.nix | st:new
