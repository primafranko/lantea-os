# 0018 — Held ids, the Tended Store boundary, and how restore and release refuse

**Status:** Accepted · **Date:** 2026-10-10 · Amended 2026-10-10 (mounts, control characters, ownership, symlinked `.local`, restore with a mount below)

## Context

Decision 0006 makes Held each user's XDG Trash. `lantea set-down`, `held`, `restore` and `release` act on it. They handle names and paths that come from the user and from `.trashinfo` files any program can write, and `release` deletes permanently. The brief does not say what a held item is called, where the Tended Store ends, or how these commands refuse.

## Decision

- **A held id is one file name from `Trash/files`**, the same name `gio`, Dolphin and trash-cli use (`a.txt`, then `a.txt.2`, `a.txt.3`, … on a collision). An empty id, `.`, `..`, or an id containing `/` is refused as "Nothing is held as …".
- **Only the user's home Trash** (`$XDG_DATA_HOME/Trash`) is Held. Per-mount `.Trash-$uid` directories are not used.
- **Held must belong to the caller.** The home must exist and be owned by the effective uid; then, if the Trash directory or a directory leading to it below the home is owned by another uid, every command refuses: "Held at <T> belongs to another user, so lantea will not act on it. Run lantea as that user instead. Nothing was changed." This also stops root running with another user's `HOME`.
- **Directory handles, not joined paths.** The Trash directories are opened once as directory handles, and every operation works relative to a handle (`openat`, `renameat2`, `unlinkat`, `mkdirat`) without following symlinks. Paths built as strings are only shown to the user and recorded.
- **"Outside the Tended Store" means outside the user's home directory, on any file system.** A path counts as inside when its parent directory's real path, taken from the opened handle, is the home or inside it. A path on another file system inside the home cannot be set down, because a rename cannot cross file systems.
- **Symlinks are acted on themselves**, never followed. Setting down a link moves the link; its target is untouched.
- **Mounts are recognised by mount ID** (`statx` `STATX_MNT_ID`), not by `st_dev`, which misses bind mounts and differs for every Btrfs subvolume. One check, "is this on the same mount as `Trash/files`?", is used everywhere; a held item that is itself a mount point is refused by `release` before anything is recorded or deleted. An item on another mount than the Held items, a bind mount included, is refused as "on a separate mount". `set-down` also refuses an item that has any other mount at or under it, naming the mount point. Walks for size and for `release` never cross into another mount: a release that meets one leaves it in place, keeps the item held, and is recorded as `release-incomplete`, naming the path that was left.
- **Never set down:** the home directory itself, the Trash directories, anything inside them, or any directory that contains them.
- **Restore never overwrites.** Both `set-down` and `restore` move with `renameat2(RENAME_NOREPLACE)`, so this holds even in a race. When the destination exists, `restore` refuses and offers `--to <path>`.
- **The `Path=` in a `.trashinfo` is untrusted.** If it is not absolute, has `.` or `..` components, or leads outside the home, `restore` refuses and offers `--to`. `--to` follows the same rule.
- **Missing parent directories are recreated** on restore, and the output names them.
- **Restore never writes into Held.** A destination inside the Trash directory (itself, `files/` or `info/`) is refused like set-down refuses it.
- **Restore with a mount below the item is allowed.** It is a rename: nothing is copied or deleted, and the mount moves out of Held with the item. `release`, which deletes, never enters that mount.
- **The release question.** `release` asks `[y/N]` on stderr and reads one line from stdin unless `--yes` is given. `y` or `yes` confirms; anything else or EOF declines. When stdin is not a terminal and `--yes` is not given, `release` refuses with exit 1 and a plain message. It never waits or guesses.
- **Errors and refusals** are plain text on stderr with exit 1, also under `--json`. `--json` changes only successful output.
- **Untrusted text is escaped on the terminal.** Ids, paths and `Path=` values are printed with every control character and the backslash shown as `\xHH` (a backslash as `\x5c`), in normal output, messages, the release prompt and `--explain`. `--json` uses JSON escaping, and the Record keeps the raw bytes.
- **A `.trashinfo` must be a regular file** of at most 64 KiB with an absolute `Path=`; anything else is unparsable and not shown, and `held` says how many items it could not read.

## Consequences

- Trash written by `lantea` and by other tools are the same thing; Held adopts existing Trash content unchanged.
- A script that releases must say `--yes`; piping an answer does not count as consent.
- A `.trashinfo` written by another program cannot make `restore` write outside the home.
- Files on a separate mount inside the home are outside Held until per-mount Trash is supported (not planned).

## Known limitations (Phase 1)

- **A symlinked `~/.local`, `~/.local/share` or Trash directory is refused.** Every command stops with a message that names the link and says to replace it with a folder. Lantea opens these directories without following symlinks so that the ownership check means something; accepting a link whose target the caller owns is a follow-up.
