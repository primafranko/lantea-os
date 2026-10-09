# 0006 — Tended Store at /home; Held is XDG Trash inside it

**Status:** Accepted · **Date:** 2026-10-09

## Context

The 1.0 brief put Held in its own Btrfs subvolume `@held` and mounted `@tended` at `/tended`, bind-mounted into `/home`. Btrfs refuses to rename across subvolumes, so every set-down would be a full copy, not a move, and not atomic. The desktop's file manager also has its own Trash, so there would be two parallel "deleted files" concepts.

## Decision

- `@tended` is mounted directly at `/home`. There is no `/tended` path.
- Held is each user's XDG Trash (`~/.local/share/Trash`, with `.trashinfo` metadata: original path and deletion time), inside the same subvolume. There is no `@held` subvolume.
- `lantea set-down` is a well-behaved XDG Trash client that also writes a Record entry. Dolphin's "Move to Trash", `gio trash` and `trash-cli` are set-down too.

## Consequences

- Set-down is an atomic rename and costs nothing for large files.
- Held items are included in `@tended` snapshots. That is honest: a snapshot is the state at that moment.
- Set-downs made by other tools have no Record entry at the time; `lantea held` shows them, and Condition can note un-recorded set-downs.
- Subvolume list becomes `@root`, `@nix`, `@tended`, `@record`, `@snapshots`.
