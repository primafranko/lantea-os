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
- 2026-10-10 | stack:nix | x1 | src:self | WHEN a NixOS VM test asserts a `grep -c` count → read it via `machine.execute`, not `machine.succeed`, because `grep -c` exits 1 when the count is 0 | ref:tests/tended-store.nix | st:new
- 2026-10-10 | project | x1 | src:self | WHEN searching a binary for a string from the agent shell → use `command grep -c`; the session's `grep` wrapper skips binary files and prints nothing | ref:docs/tasks/002-record-writer-and-held.md | st:new
- 2026-10-10 | universal | x1 | src:self | WHEN a test needs a deletion or unlink to fail partway → make a directory 0500 inside a temp dir and restore 0700 after; note it assumes tests do not run as root | ref:pkgs/lantea/cli/tests/held.rs | st:new
- 2026-10-10 | project | x1 | src:self | WHEN a spec defines a size "like du" → name the exact du flags (`--apparent-size --block-size=1`), hard links once, symlinks counted, before implementing | ref:docs/tasks/002-record-writer-and-held.md | st:new
- 2026-10-10 | universal | x2 | src:review | WHEN a recursive delete or size walk must not cross mounts → compare mount IDs (statx STATX_MNT_ID, never st_dev), and compare the root item with its container too, or a mount on the item itself is emptied | ref:pkgs/lantea/cli/src/held.rs:1123 | st:new
- 2026-10-10 | universal | x1 | src:review | WHEN printing names or metadata read from files any local program can write (trashinfo, desktop files) → escape control characters before the terminal, above all in a confirmation prompt | ref:pkgs/lantea/cli/src/held.rs:898 | st:new
- 2026-10-10 | stack:rust | x1 | src:self | WHEN a test may block (FIFO, socket, stdin) → run the test binary under `timeout`, and never `pkill -f` a pattern that matches your own shell | ref:pkgs/lantea/cli/tests/held.rs | st:new
- 2026-10-10 | universal | x1 | src:self | WHEN untrusted text reaches a terminal → build messages raw, escape once at the print boundary, and keep multi-sentence errors as a list, not joined with \n | ref:pkgs/lantea/cli/src/held.rs | st:new
- 2026-10-10 | universal | x1 | src:self | WHEN deciding whether two paths are on the same mount → compare `statx` STATX_MNT_ID, never st_dev (misses bind mounts; differs per Btrfs subvolume) | ref:docs/decisions/0018-held-ids-and-refusals.md | st:new
- 2026-10-10 | universal | x1 | src:self | WHEN adding backslash escaping for terminal output → check every fixed string printed through the escaper (format strings, shell quoting) for backslashes of its own | ref:pkgs/lantea/cli/src/held.rs | st:new
- 2026-10-10 | project | x1 | src:self | WHEN editing Rust with sed or heredocs (the edit hook does not run) → run `cargo fmt --all` in pkgs/lantea before `aih verify` | ref:.claude/harness.json | st:new
- 2026-10-10 | stack:rust | x1 | src:self | WHEN a test must fill a Unix datagram socket's queue → send each datagram from a fresh socket; one sender runs out of its own send buffer first | ref:pkgs/lantea/cli/tests/held.rs | st:new
- 2026-10-10 | stack:nix | x1 | src:self | WHEN tests need mounts without root → run them under `unshare -Urm` with util-linux in nativeCheckInputs, and bind trees with submounts using `--rbind` | ref:pkgs/lantea/default.nix | st:new
- 2026-10-10 | universal | x1 | src:review | WHEN a command refuses targets inside a protected directory (set-down into Held) → apply the same refusal to the reverse operation's destination (restore, --to) | ref:pkgs/lantea/cli/src/held.rs:1060 | st:new
- 2026-10-10 | universal | x1 | src:review | WHEN an ownership check skips paths that do not exist yet and the command then creates them → check ownership again after creating, or refuse when euid differs from the home owner | ref:pkgs/lantea/cli/src/held.rs:477 | st:new
- 2026-10-10 | universal | x1 | src:self | WHEN a refusal's wording depends on check order → update every test that runs the earlier situation, VM tests included, in the same change | ref:tests/tended-store.nix | st:new
- 2026-10-10 | stack:rust | x1 | src:self | WHEN a recursive helper must record context on error → use a free fn taking `&mut Option<_>` with `inspect_err`, not a captured FnMut closure that holds the borrow across recursion | ref:pkgs/lantea/cli/src/held.rs | st:new
- 2026-10-10 | universal | x1 | src:review | WHEN Implementer notes relax an acceptance criterion (e.g. accept input the criterion refuses) → amend the criterion itself in the same change, or the spec contradicts itself | ref:pkgs/lantea/cli/src/held.rs:1077 | st:new
