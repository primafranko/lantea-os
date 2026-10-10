# Task 002 — record-writer-and-held

| Field | Value |
|---|---|
| Area / owner | tended-store + record (`pkgs/lantea/**`, `tests/**`) · Primož |
| Risk | high (data deletion: `release` permanently deletes user files) |
| Depends on | 001 (merged) |
| Contract | `docs/brief.md` §2, §5 (`set-down`, `held`, `restore`, `release`), §6 Phase 1 "Done when" · decisions 0005, 0006, 0010, 0015, **0017**, **0018**, **0019** |
| Model hint | opus (XDG Trash edge cases, directory-handle file handling and the journald native protocol are subtle) |

## Goal
The Steward can set a file down, see what is held, restore it, and release it, with `lantea`; files trashed by other tools (`gio trash`) show up in `lantea held`; and every set-down, restore and release done by `lantea` leaves a structured entry in the Record (journald, identifier `lantea`). This is the whole Phase 1 "Done when" from the brief; the disk layout and snapshots follow in tasks 003 and 004.

## Context
- Decision 0006: Held is each user's XDG Trash (`$XDG_DATA_HOME/Trash`, default `~/.local/share/Trash`) inside `@tended`, mounted at `/home`. `lantea set-down` is a well-behaved XDG Trash client that also writes a Record entry.
- Decision 0005: Phase 1 brings a minimal Record writer, structured journald entries under the `lantea` identifier.
- **Decision 0017** (Accepted) fixes the Record entry format. **Decision 0018** (Accepted) fixes Held ids, the Tended Store boundary, symlinks, restore and the release question. Both are written; this task implements them and does not change them.
- Decision 0015: `--explain` wins over `--json` and runs nothing; `--yes` is a global flag.
- Decision 0019: VM tests are booted in CI. Locally, `aih verify`'s `check` builds the VM test's driver without booting it.
- Files that matter: `pkgs/lantea/cli/src/main.rs`, `pkgs/lantea/cli/tests/cli.rs`, `pkgs/lantea/cli/Cargo.toml`, `pkgs/lantea/default.nix`, `tests/default.nix`, `tests/discipline.nix` (the style to follow).
- XDG Trash specification 1.0 (freedesktop.org): `.trashinfo` holds `[Trash Info]`, `Path=` (absolute, percent-encoded) and `DeletionDate=YYYY-MM-DDThh:mm:ss` in local time. A client reserves a name by creating `info/<name>.trashinfo` with `O_EXCL`, then moves the file to `files/<name>`.

## Assumptions
**Record writer**
- The Record writer lives in the CLI crate as a module (`pkgs/lantea/cli/src/record.rs`); no shared lib crate yet. It sends one datagram per entry to `/run/systemd/journal/socket` using journald's native protocol (`KEY=value\n`; values containing a newline use the binary form `KEY\n<u64 little-endian length><value>\n`). No libsystemd, no C linking.
- Fields per entry are exactly those in decision 0017. Who acted is journald's trusted `_UID`, which journald adds itself; Lantea writes no user field.
- **`LANTEA_JOURNAL_SOCKET` exists only in test builds.** It is behind a Cargo feature `test-journal-socket` of the `lantea` crate, off by default. Without the feature the socket path is the fixed constant and no environment variable is read. `cli/tests/held.rs` starts with `#[cfg(not(feature = "test-journal-socket"))] compile_error!(…)`, so it cannot be skipped silently. The gates and the Nix package's test phase enable the feature (`--all-features`, `cargoTestFlags`); the installed binary is built without it.

**Held items and the Tended Store** (decision 0018)
- **A held id is one file name from `Trash/files`.** An id that is empty, is `.` or `..`, or contains `/` is refused with the unknown-id message, before anything is opened. When `a.txt` is taken, set-down tries `a.txt.2`, `a.txt.3`, … until creating `info/<id>.trashinfo` with `O_EXCL` succeeds.
- **Directory handles, never joined strings.** `<T>`, `<T>/files` and `<T>/info` are opened once as directory handles (`O_DIRECTORY | O_NOFOLLOW`, created with mode 700 if missing). Every later step works relative to a handle: `openat`, `renameat2`, `unlinkat`, `mkdirat`, `fstatat(AT_SYMLINK_NOFOLLOW)`. Recursive deletion for `release` walks directory handles and never follows a symlink. Paths built as strings are used only for display, `--json`, the Record and `--explain`.
- **"Outside the Tended Store" means outside the user's home directory, on any file system.** A path is resolved like this:
  1. Make it absolute against the current directory.
  2. Canonicalise its parent.
  3. Keep the last component as given.
  4. Open the parent as a directory handle.
  5. Decide on the handle's real path (`/proc/self/fd/<n>`): it must be the home or inside it. A parent symlink that points outside the home is refused.
- **Symlinks are acted on themselves, never followed.** Setting down a symlink moves the link; its target is untouched. Its size is the link's own size (`lstat`), whether it is set down alone or sits inside a set-down directory.
- **Refused targets:** the home directory itself, `<T>`, `<T>/files`, `<T>/info`, anything inside `<T>`, and any directory that contains `<T>` (such as `~/.local`).
- **Another file system inside the home** (for example a mount at `~/mnt`) cannot be set down, because a rename cannot cross file systems. It is refused before anything changes.
- **Moves never overwrite.** Moves use `renameat2(…, RENAME_NOREPLACE)` (`rustix`) for both set-down and restore, so "never overwrites" holds even in a race. `EEXIST` from the rename is reported as the "already exists" refusal.

**Restore**
- **The `Path=` in a `.trashinfo` is untrusted.** It is percent-decoded and must be absolute, without `.` or `..` components. Its destination is then checked with the same boundary rule as set-down: the deepest existing ancestor's real path must be inside the home. Otherwise the restore is refused and `--to` is offered.
- **`--to <path>` may be relative** (Primož, 2026-10-10). It is made absolute against the current directory, `.` parts are dropped, and a `..` part is refused. Then the same checks as `Path=` apply: the home boundary, "inside your Held items", and the mount check. `--explain`, `--json`, the output and the Record all show that absolute path. `Path=` in an info file stays absolute-only.
- Missing parent directories of the destination are recreated (`mkdirat` from the deepest existing ancestor's handle, mode 755 minus the umask). The output says so.

**Release**
- `set-down` and `restore` are not destructive and never ask.
- `release` asks unless `--yes`. The question goes to stderr and the answer is read from stdin (one line). `y` or `yes`, case-insensitive, confirms. Anything else, or EOF, declines.
- **When stdin is not a terminal** (`std::io::IsTerminal`) and `--yes` is not given, `release` refuses at once with exit 1. It never waits or guesses.

**Errors and Record failures**
- Errors and refusals are always plain text on stderr with exit 1, also under `--json`. `--json` changes only successful stdout.
- **Order of action and Record entry** (decision 0017, amended 2026-10-10). `set-down` and `restore` act first, then write the entry. If that write fails, they say so on stderr and exit 1; the action is not undone. **`release` writes its entry first and deletes only if the write succeeded.** If deletion then fails partway, it writes a second entry, `release-incomplete`.
- The VM test reads the Record as root with `journalctl -t lantea -o json`. A `lantea record` command is Phase 4.

**Dependencies** (approved by Primož)
- New Cargo dependencies: `rustix` (features fs, process — process for geteuid, no extra crate), `percent-encoding`, `chrono` (`default-features = false`, features `clock`), and `tempfile` as a dev-dependency only.
- The matching `Cargo.lock` change adds those crates and what they pull in. There are no other `Cargo.lock` changes and no `cargo update`.

## Files in scope
- `pkgs/lantea/cli/src/main.rs` — new subcommands `set-down`, `held`, `restore`, `release`
- `pkgs/lantea/cli/src/record.rs` (new) — the minimal Record writer
- `pkgs/lantea/cli/src/held.rs` (new) — XDG Trash client: list, set down, restore, release
- `docs/decisions/0017-record-entry-format.md` — amended by Primož on 2026-10-10 (release writes its entry first; the `release-incomplete` action). Already done before implementation; change it only if the code shows the amendment is wrong, and say so.
- `pkgs/lantea/cli/Cargo.toml` — the dependencies above and the `test-journal-socket` feature
- `pkgs/lantea/Cargo.lock` — added entries for those dependencies only
- `pkgs/lantea/cli/tests/held.rs` (new) — CLI tests with a temporary `HOME` and a bound datagram socket
- `pkgs/lantea/default.nix` — enable `test-journal-socket` for the test phase only (`cargoTestFlags`)
- `.claude/harness.json` — the `rust` package's `lint` and `test` commands add `--all-features`; nothing else
- `tests/default.nix` — wire `tended-store`
- `tests/tended-store.nix` (new) — the Phase 1 acceptance VM test
- `modules/core/record/README.md`, `modules/core/tended-store/README.md` — say what now exists and where
- `docs/decisions/0018-held-ids-and-refusals.md` — amended in place on 2026-10-10 (mounts, control characters); it is not on `main` yet

**Approved by Primož on 2026-10-10, outside this task's code but on its branch:**
- the build-machine change, commit `ab4c3ad`: `docs/decisions/0019-vm-tests-boot-in-ci.md`, the pointer in `docs/decisions/0012-architectures.md`, `tests/check-without-boot.sh`, and the `check`/`check.vm` gates in `.claude/harness.json`;
- decisions `0017` and `0018`, and their rows in `docs/decisions/README.md`.

Accepting decision 0014 is **not** part of this branch; it goes into task 003's pull request.

**Added by Primož on 2026-10-10 (third review round):**
- `.github/workflows/ci.yml`: a step before the build that lifts Ubuntu's AppArmor restriction on unprivileged user namespaces, for the `unshare -Urm` tests;
- `docs/decisions/0019-vm-tests-boot-in-ci.md`: a note under Consequences about that step (0019 is not on `main` yet, so it is amended in place);
- specs `docs/tasks/003-disk-layout.md` and `docs/tasks/004-tended-snapshots.md` stay staged in this PR, as Phase 1 planning.

## Acceptance criteria
In the strings below, `<H>` is the user's home (`/home/steward` in the VM) and `<T>` is `<H>/.local/share/Trash`. Every stdout or stderr line ends with one newline unless marked "no newline".

**set-down**
- [ ] **Basic set-down.** Given `<H>/a.txt` (12 bytes), `lantea set-down <H>/a.txt` exits 0 and prints `Set down <H>/a.txt. It is held as a.txt; to bring it back: lantea restore a.txt`. Afterwards:
  - `<H>/a.txt` is gone;
  - `<T>/files/a.txt` has the same content and inode;
  - `<T>/info/a.txt.trashinfo` is exactly `[Trash Info]\nPath=<H>/a.txt\nDeletionDate=<YYYY-MM-DDThh:mm:ss local>\n`.
- [ ] `Path=` is percent-encoded: `<H>/two words.txt` is written as `Path=<H>/two%20words.txt`. Unreserved characters and `/` stay literal.
- [ ] If `a.txt` is already held, a second `<H>/a.txt` is held as `a.txt.2`, and the hint names `a.txt.2`. If the id has characters outside `[A-Za-z0-9._+/-]`, it is single-quoted in the hint (for example `lantea restore 'two words.txt'`).
- [ ] A directory can be set down. **Size** (everywhere: set-down, `held`, release, `--json`, the Record) is exactly what `du -s --apparent-size --block-size=1` reports. That is the `lstat` size of the item and of every entry under it, counting symlinks and directories themselves, and hard links once. Symlinks are never followed.
- [ ] A symlink is set down as itself: `<H>/link` → `/etc/hostname` is moved to `<T>/files/link`, is still a symlink with the same target, and `/etc/hostname` is unchanged.
- [ ] `--json` prints `{"action":"set-down","id":"a.txt","path":"<H>/a.txt","size":12}`.
- [ ] Refusals exit 1, print to stderr, and change nothing. The Trash directories may be created.
  - missing path → `<H>/missing.txt does not exist. Nothing was changed.`
  - outside the home, on any file system (VM: `/dev/shm/x` and `/tmp/y`), or through a parent symlink that leads outside → `/dev/shm/x is outside the Tended Store, so it cannot be set down. Nothing was changed.`
  - on a separate mount inside the home (VM: a tmpfs at `<H>/mnt`; also a bind mount on the same file system) → `<H>/mnt/z is on a separate mount from your Held items, so it cannot be set down. Nothing was changed.` (adjusted by Primož, 2026-10-10)
  - `<H>`, `<T>`, anything inside `<T>`, or any path that contains `<T>` → `<path> contains your Held items, so it cannot be set down. Nothing was changed.`
- [ ] `--explain` exits 0, changes nothing, and prints these lines in order (paths absolute, shell-quoted when needed):
  1. `mkdir -p -m 700 -- <T>/files <T>/info`
  2. `printf '[Trash Info]\nPath=%s\nDeletionDate=%s\n' '<encoded path>' '<now>' > <T>/info/a.txt.trashinfo`
  3. `mv -n -- <H>/a.txt <T>/files/a.txt`
  4. `logger --journald <<'EOF'`, then one `FIELD=value` line per Record field (decision 0017), then `EOF`.

**held**
- [ ] With nothing held, `lantea held` prints `Nothing is held.` and exits 0; `--json` prints `{"items":[]}`.
- [ ] With items, it prints `Held · 1 item` (or `Held · N items`), then one line per item, newest first (ties by id): two spaces, then `<id>  <original path>  <size>  set down <YYYY-MM-DD HH:MM>`, with fields separated by two spaces.
- [ ] Sizes: `<n> B` below 1024. Otherwise one decimal place in `KiB`, `MiB`, `GiB` or `TiB` (1536 → `1.5 KiB`).
- [ ] `--json` prints `{"items":[{"id":"a.txt","path":"<H>/a.txt","deleted_at":"<DeletionDate as written>","size":12}]}` in the same order.
- [ ] Items trashed by other tools are listed. In the VM, `gio trash <H>/c.txt` makes `c.txt` appear, with path `<H>/c.txt`.
- [ ] These are skipped and not printed: an info file without a matching `files/<id>`, a file without an info file, and an unparsable info file. `held` lists a `Path=` outside the home as written; only `restore` refuses it.
- [ ] `--explain` prints exactly `cat -- <T>/info/*.trashinfo` and `du -s --apparent-size --block-size=1 -- <T>/files/*` (two lines) and exits 0.

**restore**
- [ ] `lantea restore a.txt` moves `<T>/files/a.txt` back to its original path, removes `<T>/info/a.txt.trashinfo`, prints `Restored a.txt to <H>/a.txt.`, and exits 0. `--json` prints `{"action":"restore","id":"a.txt","path":"<H>/a.txt","created":[]}`.
- [ ] `lantea restore a.txt --to <path>` restores to `<path>` instead, with the same messages naming `<path>`.
- [ ] **Missing parents are recreated.** When the destination's parents are missing (say `<H>/gone/deeper/` for `<H>/gone/deeper/a.txt`):
  - they are created;
  - stdout is first `Recreated <H>/gone/deeper, which no longer existed.` (naming the deepest created directory), then the `Restored …` line;
  - `--json` lists every created directory, top first: `"created":["<H>/gone","<H>/gone/deeper"]`.
- [ ] Refusals exit 1, print to stderr, and leave both the held item and the destination untouched:
  - destination exists (checked up front, and `EEXIST` from `renameat2`) → `<H>/a.txt already exists, so a.txt was not restored. To restore it elsewhere: lantea restore a.txt --to <path>`
  - the info file's `Path=` is not absolute, has `.`/`..` components, or leads outside the home → `a.txt was set down from /tmp/evil, which is outside the Tended Store, so it was not restored there. To restore it elsewhere: lantea restore a.txt --to <path>`
  - `--to` has a `..` part, or (after being made absolute against the current directory, with `.` parts dropped) leads outside the home → `<path> is outside the Tended Store, so a.txt was not restored. Nothing was changed.` A relative `--to` inside the home is accepted, and every output shows its absolute form.
  - unknown id, or an id that is empty, `.`, `..` or contains `/` → `Nothing is held as <id>. To see what is held: lantea held`
- [ ] `--explain` prints `mkdir -p -- <missing parent>` (only when a parent is missing), then `mv -n -- <T>/files/a.txt <H>/a.txt` and `rm -- <T>/info/a.txt.trashinfo`, then the `logger --journald` block. It changes nothing.

**release**
- [ ] **Confirmed release.** Without `--yes`, on a terminal, `lantea release b.txt` writes `Release b.txt (<H>/b.txt, 4 B)? It will be permanently deleted. [y/N] ` (no newline) to stderr and reads one line. On `y` or `yes`:
  - it deletes `<T>/files/b.txt` (recursively for directories, through directory handles, never following symlinks) and `<T>/info/b.txt.trashinfo`;
  - it prints `Released b.txt. <H>/b.txt has been permanently deleted.` and exits 0.
- [ ] On any other answer, or EOF, it prints `Nothing was released.` to stderr, exits 1, and the item is still held.
- [ ] Without `--yes` and with stdin not a terminal, it asks nothing and reads nothing. It prints `Release asks before deleting anything, but there is no terminal to ask on. To release without being asked: lantea release b.txt --yes` to stderr, exits 1, and the item is still held.
- [ ] `--yes` skips the question. `--json` prints `{"action":"release","id":"b.txt","path":"<H>/b.txt","size":4}`. An unknown or invalid id gives the same refusal as `restore`.
- [ ] `--explain` prints the `logger --journald` block first (the entry is written before deleting), then `rm -rf -- <T>/files/b.txt <T>/info/b.txt.trashinfo`. It asks nothing and changes nothing.
- [ ] When an item was restored or released but its `.trashinfo` could not be removed, the message ends with one more sentence: `It may still be listed by lantea held.` For example: `a.txt was restored to <H>/a.txt, but <T>/info/a.txt.trashinfo could not be removed: <reason>. It may still be listed by lantea held.`

**The Record** (decision 0017)
- [ ] Each successful `set-down`, `restore` and `release` writes exactly one journald entry with exactly these fields:
  - `MESSAGE=` — the plain stdout sentence without its newline. For a restore that recreated directories, it is the `Restored …` sentence.
  - `MESSAGE_ID=` — the action's fixed id from 0017.
  - `PRIORITY=5`
  - `SYSLOG_IDENTIFIER=lantea`
  - `LANTEA_RECORD_VERSION=1`
  - `LANTEA_ACTION=set-down|restore|release`
  - `LANTEA_HELD_ID=<id>`
  - `LANTEA_PATH=<absolute path>` — the original path for set-down and release, the destination for restore.
  - `LANTEA_SIZE=<bytes>` — set-down and release only.

  No field names the user: the VM test reads journald's `_UID`.
- [ ] A refusal, a declined or refused release, `--explain`, and `held` write no entry.
- [ ] For `set-down` and `restore`: if the entry cannot be sent, the command still completes the action, prints `The <action> was done, but the Record could not be written: <reason>.` to stderr, and exits 1. Here `<action>` is `set-down` or `restore`.
- [ ] For `release`: the entry is sent **before** anything is deleted. If it cannot be sent, nothing is deleted, the item is still held, and `release` prints `The Record could not be written, so <id> was not released: <reason>. Nothing was deleted.` to stderr and exits 1 (Rust test).
- [ ] If deletion fails partway after the release entry was written, a second entry is written with `LANTEA_ACTION=release-incomplete`, `MESSAGE_ID=a2a943364ecc42288bf2ab9c44ae4db8`, `MESSAGE=<id> could not be released completely: <reason>. Some of it may already be deleted. To see what is held: lantea held` (the same sentence the command prints), `PRIORITY=4`, and the same `LANTEA_HELD_ID`, `LANTEA_PATH` and `LANTEA_SIZE`. The command prints the existing "could not be released completely" sentence and exits 1. If that second entry cannot be sent either, it also prints `The Record could not be written: <reason>.` (Rust test, for example with an unremovable entry inside a held directory).
- [ ] A value containing a newline is sent in journald's binary form, and arrives intact (Rust test).
- [ ] The installed binary does not read `LANTEA_JOURNAL_SOCKET`: the string does not occur in `$(readlink -f $(command -v lantea))` (VM test, `grep -c` returns 0).

**Wiring and gates**
- [ ] `tests/tended-store.nix` runs the scenario under Test data on a headless node with `nixosModules.core`, `lantea.enable = true` and `pkgs.glib` (for `gio`, test node only). It is wired as `checks.<system>.tended-store` for both systems.
- [ ] `nix develop -c cargo test --workspace --locked --all-features` passes, including the new `cli/tests/held.rs`. Those tests use a temporary `HOME`/`XDG_DATA_HOME` and a test-bound `LANTEA_JOURNAL_SOCKET`, whose datagrams they assert. They cover: id validation, symlinks, the boundary checks, `Path=` outside the home, `--to`, recreated parents, the non-terminal release refusal and `--yes`, and every Record field.
- [ ] The `rust` package's `lint` and `test` commands in `.claude/harness.json` pass `--all-features`. `pkgs/lantea/default.nix` sets `cargoTestFlags = [ "--features" "test-journal-socket" ]` so `nix build .#lantea` runs the tests with the feature and installs a binary built without it.
- [ ] `aih verify` prints `VERIFY: PASS` (its `check` builds the `tended-store` driver without booting it, decision 0019).
- [ ] **The `tended-store` VM test passes in CI** on this task's pull request. Locally it is optional (`aih run check.vm`).

**Review fixes (Primož, 2026-10-10)** — these take precedence over earlier criteria where they differ.

*Mounts.* A mount is identified by its mount ID: `statx(…, AT_SYMLINK_NOFOLLOW, STATX_MNT_ID)` through `rustix`. Never compare `st_dev`, which misses bind mounts on the same file system and differs for every Btrfs subvolume.
- [ ] **The top item.** The top-item refusal uses the mount ID: the item's mount ID differs from that of `<T>/files`. That also catches bind mounts on the same file system, so it uses the "separate mount" wording: `<path> is on a separate mount from your Held items, so it cannot be set down. Nothing was changed.` Restore across mounts says the same: `<path> is on a separate mount from your Held items, so <id> was not restored. Nothing was changed.`
- [ ] **A mount at or under the item.** `set-down` refuses an item when any entry at or under it, walked through directory handles without following symlinks, has a mount ID different from the item's parent directory. It prints `<path> contains <mount point>, which is a separate mount, so it cannot be set down. Nothing was changed.`, naming the first mount point found, and exits 1.
- [ ] **No walk crosses a mount.** `tree_size` and `remove_at` never cross into a different mount. A directory entry whose mount ID differs from the held item's own is neither counted nor entered.
- [ ] **A release that meets a mount** stops there and leaves that part in place, together with the directories above it that are therefore not empty. It keeps the `.trashinfo`, so the item is still held. It writes the `release-incomplete` entry and prints the existing message, naming the path that was left: `<id> could not be released completely: <mount point> is a separate mount, so it was left in place. The release is in the Record, marked as incomplete. Some of it may already be deleted. To see what is held: lantea held`. It exits 1.
- [ ] **Tests.** The Rust tests cover what they can without root. The VM test covers the rest (Test data steps 11 and 12).

*Untrusted text on the terminal.*
- [ ] Every string from the file system or a `.trashinfo` (ids, paths, `Path=`) is escaped before it reaches stdout or stderr: in `held`, every message, the release prompt and `--explain`.
- [ ] Each control character (U+0000–U+001F, U+007F, U+0080–U+009F) **and the backslash itself** is shown as `\xHH`, two lowercase hex digits: a newline becomes `\x0a`, a backslash `\x5c`. So the display is unambiguous, also in the release prompt. Everything else is unchanged.
- [ ] `--json` is unaffected: JSON escaping already handles these.
- [ ] The Record keeps the raw bytes, with journald's binary form for newlines.
- [ ] Because of this, a value can no longer break the `logger --journald` block in `--explain`. The "unescaped logger block" known limitation is gone; remove it from the Implementer notes.

*Info files.*
- [ ] A `.trashinfo` is opened with `O_NOFOLLOW | O_NONBLOCK`, must be a regular file (`fstat`), and is read up to 64 KiB. Anything else is treated as unparsable and skipped, so a FIFO cannot make `held` hang.
- [ ] A `Path=` that is not absolute is also unparsable: the home Trash requires absolute paths. The item is skipped by `held`, and `restore`/`release` say "Nothing is held as <id>". So `LANTEA_PATH` is always absolute.

- [ ] **`held` never hides skipped items silently** (Primož, 2026-10-10).
  - **What counts:** info files that are unreadable, not a regular file, over 64 KiB, unparsable, or have a non-absolute `Path=`. An info file without its `files/<id>`, or a file without an info file, still doesn't count.
  - **The extra line:** when any such files exist, the listing ends with one more line: `1 item in Held could not be read and is not shown.` or `<N> items in Held could not be read and are not shown.`
  - **With no readable items,** it prints `Nothing is held.` and then that line.
  - **`--json`** adds a count field, `"unreadable":<N>`, always present (0 when there are none): `{"items":[…],"unreadable":0}`.
  - **Exit code:** stays 0.

*Sizes.*
- [ ] **An item whose size cannot be measured** (for example an unreadable subdirectory) is still listed:
  - `held` shows `size unknown` in place of the size;
  - `--json` has `"size":null`;
  - the release prompt says `(<path>, size unknown)`;
  - the Record entry omits `LANTEA_SIZE` (decision 0017).

  `restore` and `release` work on it as usual.
- [ ] **`human_size` picks the unit after rounding:** 1048575 bytes prints `1.0 MiB`, not `1024.0 KiB`.

**Second review fixes (Primož, 2026-10-10)** — these take precedence over earlier criteria where they differ.

*One shared mount check.*
- [ ] **One helper** answers "is this on the same mount as `<T>/files`?", by comparing `statx` mount IDs.
  - **Every path uses it:** `set-down` (the top item and every entry under it), `restore`, `release`, `held` and the size walk.
  - **The base:** the mount of `<T>/files` is the base everywhere. The held item's own mount is no longer used as the base.
  - **No other comparison:** no code path compares mounts any other way.
- [ ] **A held item that is itself a mount point** (its mount ID differs from `<T>/files`):
  - **release refuses up front,** before any Record entry is written, so nothing is deleted. It prints `<id> is a separate mount, so it cannot be released. Nothing was changed.` and exits 1.
  - **`held`** lists it with `size unknown`.
  - **`restore`** refuses it with the existing "separate mount" restore sentence.
- [ ] **Below the item, release and the size walk** neither enter nor count anything whose mount ID differs from `<T>/files`. The existing "left in place" behaviour is unchanged.
- [ ] **One test table** in `cli/tests/held.rs` runs each command against each mount situation.
  - **Commands:** `set-down`, `held`, `held --json`, `restore`, `release --yes`.
  - **Situations:** the item itself a mount, a mount below the item, and a bind mount on the same file system.
  - **Without root:** every combination that can be set up without root is in the table, using `/dev/shm` or a user namespace (`unshare -Urm`) where the build sandbox allows one.
  - **Needing root:** each combination that needs root is named in a comment in the table and covered by the VM test (Test data step 12). None is skipped silently.

*Restore leaves nothing behind when it fails.*
- [ ] **Only directories `mkdirat` actually created are tracked.** `EEXIST` doesn't count as created and is never reported as "Recreated".
- [ ] **When restore fails after creating directories,** it removes them again, deepest first, with `unlinkat(AT_REMOVEDIR)`, so `Nothing was changed.` stays true.
- [ ] **If that cleanup fails,** the message instead ends with `The directories <dir>, … were created and are still there.` in place of `Nothing was changed.` (Rust test).

*Record failure on the second entry.*
- [ ] **A Rust test covers a failed `release-incomplete` send.** The test socket stops accepting after the first datagram, so the `release-incomplete` entry fails and the command also prints `The Record could not be written: <reason>.`

*Ownership of Held.*
- [ ] **The check.** Before acting, every command (`set-down`, `held`, `restore`, `release`) checks that `<T>` and every directory leading to it below the home (`<H>/.local`, `<H>/.local/share`) are owned by the caller's uid. It opens each one with `O_NOFOLLOW` through directory handles from the home, and checks with `fstat`. Directories that don't exist yet are created by `set-down` as before.
- [ ] **When one belongs to another user,** the command prints `Held at <T> belongs to another user, so lantea will not act on it. Run lantea as that user instead. Nothing was changed.` and exits 1. This also stops `root` with `HOME=/home/<user>`.
- [ ] **Tests.** A Rust test where possible. The VM test runs `lantea held` as root with `HOME=/home/steward` and expects the line and exit 1.

**Third review fixes (Primož, 2026-10-10)** — these take precedence over earlier criteria where they differ.

*Ownership starts at the home.*
- [ ] **The home comes first.** Every Held command first checks that `$HOME` exists, is a directory and is owned by the effective uid, using `fstat` on a handle opened with `O_DIRECTORY`. Only then does it check each existing directory below it (`.local`, `.local/share`, `Trash`) as before.
- [ ] **Missing below an owned home:** a missing directory below an owned home is fine; `set-down` creates it.
- [ ] **Missing or foreign home:** this is a refusal, and nothing is created.
  - foreign home: `Your home directory <H> belongs to another user, so lantea will not act on it. Run lantea as that user instead. Nothing was changed.`
  - missing home: `Your home directory <H> does not exist, so nothing was changed.`
- [ ] **Test:** `HOME` points at a directory owned by another user (a user namespace, or `/nix/store`-style): exit 1, and nothing is created under it.

*Restore never writes into Held.*
- [ ] **Refused destinations:** `restore` and `--to` refuse any destination that resolves inside the Trash directory: `<T>` itself, `files/` or `info/`, or anything below them.
- [ ] **Message:** `<path> is inside your Held items, so <id> was not restored there. Nothing was changed. To restore it elsewhere: lantea restore <id> --to <path>`
- [ ] **Test:** a Rust test covers this.

*Relative HOME.*
- [ ] **When `HOME` is set but relative,** every command prints `HOME is set to a relative path (<value>). Held needs an absolute path, so nothing was changed.` and exits 1. Test it.

*Symlinked `.local` (known limitation in Phase 1).*
- [ ] **Still refused, with a better message.** A symlink at `<H>/.local`, `<H>/.local/share` or `<T>` still stops every command. The message now names the link and says what to do: `<path> is a symbolic link, so lantea will not use it for your Held items. Nothing was changed. Replace the link with a folder to use lantea.` Test it.

*Restore with a mount below the item.*
- [ ] **Allowed.** It is a rename: nothing is copied or deleted, and it moves the item out of Held. This is unchanged and is recorded in decision 0018.

*"Could not be released" messages.*
- [ ] **When to say "Nothing was changed.":** a release failure ends with `Nothing was changed.` only when nothing was removed and no Record entry was written. Example: `<id> could not be released: <reason>. Nothing was changed.`
- [ ] **When the `release` entry was already written** but deletion then failed (decision 0017's `release-incomplete` path), the message says so and what to do. For example: `<id> could not be released completely: <reason>. The release is in the Record, marked as incomplete. Some of it may already be deleted. To see what is held: lantea held`
- [ ] **Remedies:** where the reason has a known remedy, the message ends with it as one final sentence. Examples: permission denied → `Check that you own <path>.`; the kernel does not report mount IDs → `Lantea needs Linux 5.8 or later.`. List every remedy you add in the Implementer notes.
- [ ] **The Record entry:** the `release-incomplete` entry's `MESSAGE` stays the exact sentence printed.

*Implementer notes.*
- [ ] **Bring them up to date.** In particular, the mount-under-item and release-meets-a-mount cases are covered by `mount_situations_table`, and the double Record failure by `a_failed_second_entry_is_reported_too`.

## Follow-ups (not in this task)
- Names of 246 bytes or more can't be set down, because `<name>.trashinfo` exceeds `NAME_MAX`. They are refused safely ("File name too long") and nothing changes. `gio` shortens such names; Lantea could do the same.
- A symlinked `~/.local` or `~/.local/share` (a Phase 1 limitation, decision 0018): accept it when the link's target is owned by the caller.

## Test data
VM scenario (as the Steward, `su - steward -c …`; the VM's time zone is UTC). A terminal for the release question comes from `script -qec '<command>' /dev/null`. Its output echoes the typed answer, so assert the exit code, the prompt, and the result lines, not the whole output.
1. **Setup.** `printf 'twelve bytes' > ~/a.txt`, then `printf 'four' > ~/b.txt`.
2. **Explain changes nothing.** `lantea set-down ~/a.txt --explain`: `~/a.txt` still exists and `<T>/files` is empty or missing.
3. **Set down.** `lantea set-down ~/a.txt`: assert the exact line and the info file. `lantea held` matches `^Held · 1 item\n  a\.txt  /home/steward/a\.txt  12 B  set down \d{4}-\d{2}-\d{2} \d{2}:\d{2}\n$`.
4. **Restore.** `lantea restore a.txt`: `~/a.txt` contains `twelve bytes`, and `lantea held` prints `Nothing is held.`.
5. **Release.**
   - `lantea set-down ~/b.txt`.
   - `echo y | lantea release b.txt` (stdin is a pipe): exit 1 with the exact "no terminal" line, and still held.
   - `echo n | script -qec 'lantea release b.txt' /dev/null`: exit 1, and still held.
   - `echo y | script -qec 'lantea release b.txt' /dev/null`: exit 0, and both Trash entries are gone.
6. **Other tools.** `printf 'gio' > ~/c.txt; gio trash ~/c.txt`: `lantea held --json` lists `{"id":"c.txt","path":"/home/steward/c.txt",…,"size":3}`. Then `lantea release c.txt --yes`.
7. **Never overwrite, and `--to`.** `lantea set-down ~/a.txt`, then `printf 'new' > ~/a.txt`, then `lantea restore a.txt`: exit 1 with the exact "already exists" line, and `~/a.txt` still contains `new`. Then `lantea restore a.txt --to /home/steward/a-restored.txt`: exit 0.
8. **Recreated parents.** `mkdir -p ~/gone/deeper; printf 'd' > ~/gone/deeper/d.txt; lantea set-down ~/gone/deeper/d.txt; rm -r ~/gone; lantea restore d.txt`: exit 0, the exact `Recreated /home/steward/gone/deeper, which no longer existed.` line, and `~/gone/deeper/d.txt` contains `d`.
9. **Symlink.** `ln -s /etc/hostname ~/link; lantea set-down ~/link`: `<T>/files/link` is a symlink to `/etc/hostname`, and `/etc/hostname` is unchanged. Then `lantea release link --yes`: `/etc/hostname` still exists.
10. **Untrusted `Path=`.**
    - As the Steward, write `<T>/files/evil` (content `e`) and `<T>/info/evil.trashinfo` with `Path=/tmp/evil`.
    - `lantea restore evil`: exit 1 with the exact "set down from /tmp/evil, which is outside the Tended Store" line, and `/tmp/evil` does not exist.
    - `lantea restore evil --to /home/steward/evil.txt`: exit 0.
11. **Refusals.** Each exits 1 with its exact stderr line:
    - `lantea set-down ~/missing.txt`
    - `printf x > /dev/shm/x; lantea set-down /dev/shm/x`
    - `printf y > /tmp/y; lantea set-down /tmp/y`
    - `ln -s /tmp ~/tmplink; lantea set-down ~/tmplink/y` (refused as outside the Tended Store)
    - `lantea set-down ~`
    - `lantea set-down ~/.local/share/Trash/files`
    - `lantea restore nope`
    - `lantea restore ../a.txt`
    - `lantea release . --yes`
    - As root first: `mkdir /home/steward/mnt && mount -t tmpfs tmpfs /home/steward/mnt && chown steward /home/steward/mnt`. Then as the Steward: `printf z > ~/mnt/z; lantea set-down ~/mnt/z` → the "separate mount" line.
    - **A directory with a mount under it.** As root first: `mkdir -p /home/steward/withmount/inner && mount -t tmpfs tmpfs /home/steward/withmount/inner && chown -R steward /home/steward/withmount`. Then as the Steward, `lantea set-down ~/withmount` exits 1 with `/home/steward/withmount contains /home/steward/withmount/inner, which is a separate mount, so it cannot be set down. Nothing was changed.` Afterwards `~/withmount` is still in place.
12. **Release meets a mount.**
    - **Plant the item.** As the Steward: `mkdir -p <T>/files/m/sub && printf 'm' > <T>/files/m/top.txt`, then write `<T>/info/m.trashinfo` with `Path=/home/steward/m`.
    - **Mount under it.** As root: `mount -t tmpfs tmpfs <T>/files/m/sub && printf 'keep' > <T>/files/m/sub/keep.txt && chown -R steward <T>/files/m/sub`.
    - **Release.** As the Steward, `lantea release m --yes` exits 1 with `m could not be released completely: <T>/files/m/sub is a separate mount, so it was left in place. The release is in the Record, marked as incomplete. Some of it may already be deleted. To see what is held: lantea held`.
    - **Afterwards:**
      - `<T>/files/m/sub/keep.txt` still contains `keep`;
      - `<T>/files/m/top.txt` is gone;
      - `<T>/info/m.trashinfo` still exists;
      - `lantea held` still lists `m`.
    - **Clean up.** As root, `umount <T>/files/m/sub`.
13. **The installed binary.** `grep -c LANTEA_JOURNAL_SOCKET "$(readlink -f "$(command -v lantea)")"` prints `0`.
14. **The Record.** As root: `journalctl -t lantea -o json` (use `wait_until_succeeds`, since the journal is asynchronous) holds exactly 14 entries, in this order:
    1. set-down a.txt
    2. restore a.txt
    3. set-down b.txt
    4. release b.txt
    5. release c.txt
    6. set-down a.txt
    7. restore a.txt (path `/home/steward/a-restored.txt`)
    8. set-down d.txt
    9. restore d.txt
    10. set-down link
    11. release link
    12. restore evil (path `/home/steward/evil.txt`)
    13. release m
    14. release-incomplete m (`PRIORITY=4`, `MESSAGE` naming the mount point)

    Each has exactly the fields above, including the action's `MESSAGE_ID`, and `_UID` equals the Steward's uid.

Exact strings: see Acceptance criteria. Sizes in the scenario: `a.txt` 12 B, `b.txt` 4 B, `c.txt` 3 B.

## Non-functional
- Security: everything runs as the calling user, on that user's own Trash and home only. No setuid, no elevation, no new system service. Untrusted input (ids, `Path=`, `--to`, symlinks) never leads outside the home.
- Logging: the Record holds paths and file names that the user acted on, in their own journal. Never file contents.
- Compatibility: Trash written by `lantea` is readable by `gio`/Dolphin/trash-cli, and the other way round (same naming, `.trashinfo` format, percent-encoding).
- Builds and tests on `x86_64-linux` and `aarch64-linux` (L-9). Pure-Rust dependencies only.

## Data & migrations
No schema. The on-disk formats are XDG Trash (existing standard) and the Record entry (decision 0017; Phase 4 must read and keep it). Existing Trash content on a machine is adopted as Held without change.

## Rollback
Revert the PR. Items held by `lantea` stay valid XDG Trash, so `gio`, Dolphin and trash-cli can still restore them. Released files cannot be recovered, except from a `@tended` snapshot (task 004) taken before the release. That is why `release` asks.

## Verification
Success check — run on the build machine; exits 0 when the local part of this task is done:
```
nix develop -c cargo test --manifest-path pkgs/lantea/Cargo.toml --workspace --locked --all-features && nix flake check --no-build
```
Also required:
- `aih verify` prints `VERIFY: PASS`.
- **The `tended-store` VM test passes in CI** (decision 0019). The task is not done until it does.

## Known pitfalls
- Flakes only see files tracked by git: `git add` new files (`tests/tended-store.nix`, `src/held.rs`, `src/record.rs`) before `nix build` or `nix flake check`, or they are silently missing.
- Integration tests in `cli/tests/` do not see `cfg(test)` of the binary; that is why the socket override is a Cargo feature. Clippy with `--all-targets` compiles `held.rs` too, so it needs `--all-features` as well.
- `script` forwards the piped answer through a pseudo-terminal and echoes it. If the answer arrives before the prompt is read, that is fine: the line is buffered.
- `gio trash` may print D-Bus warnings to stderr without a session bus. Assert its exit code and the Trash contents, not its silence. If it actually fails, run it under `dbus-run-session` and say so in Implementer notes.
- Don't boot the VM test locally to "prove" it. lantea-bench crashes aarch64 guests (decision 0019, inbox lesson). CI is the verdict.
- L-5: every new subcommand supports `--json` and `--explain` (and `--explain` runs nothing, decision 0015); exit 0/1/2; `release` asks unless `--yes`.
- L-1 / L-2: user text is calm and specific. "release" means only permanent removal, and "Held"/"set down" are used exactly as in the glossary. Don't add strings beyond those listed here without noting them in Implementer notes.

## Docs & changelog
- `modules/core/tended-store/README.md`: Held and set-down/restore/release live in `pkgs/lantea/cli/src/held.rs`; the Btrfs layout arrives in task 003.
- `modules/core/record/README.md`: the Phase 1 writer is `pkgs/lantea/cli/src/record.rs`, its format is decision 0017; sealing, retention and auditd come in Phase 4.
- No README Status change (that comes with task 004).

## Out of scope
- The disko layout, `@root-blank` (task 003), and btrbk snapshots (task 004). No `workstation` host yet.
- `lantea record` and Record hardening (Phase 4). Restricting who may write `lantea` entries (Phase 4).
- Trash on other mounts (`$topdir/.Trash-$uid`), the `directorysizes` cache, and emptying all of Held at once.
- Marking which held items have no Record entry (Condition, Phase 3).
- Any Nix module code for `tended-store` or `record`: nothing in this task needs it.
- Changing decisions 0017 or 0018. If the implementation shows one is wrong, stop and report it.

## Implementer notes
- 2026-10-10 (implementer, updated after the third review round)
  - **Paths in messages.** Refusals name the path as given, made absolute, with `.` components and trailing slashes dropped (so `~/tmplink/y` is refused as `/home/steward/tmplink/y`). Success output, `Path=`, `--json` and the Record use the resolved path: the parent's real path plus the last component as given. In the VM scenario the two are the same. A path ending in `..` is canonicalised before it is split.
  - **Home and ownership.** Every command first runs `place()`:
    - `HOME` unset → the "HOME is not set" refusal. `HOME` relative → the relative-path refusal.
    - The home is opened with `O_DIRECTORY`, following a symlink for the home itself, and checked with `fstat` against the effective uid (`rustix::process::geteuid`; rustix's `process` feature, which adds no crate). A missing home, or a home path that is not a directory, gets the "does not exist" refusal. A home owned by another uid gets the "belongs to another user" refusal. Nothing is created in either case.
    - Then each existing component of `<T>` below the home (`.local`, `share`, `Trash`) is opened with `O_NOFOLLOW` from the previous handle and checked with `fstat`. A missing component ends the check, and `set-down` creates it later. A foreign one gets the "Held at <T> belongs to another user" refusal.
    - A symlink there gets the "is a symbolic link" refusal, naming the link (a known Phase 1 limitation, decision 0018).
    - When `<T>` is not below the home (`XDG_DATA_HOME` elsewhere), only `<T>` itself is checked.
    - This runs under `--explain` too.
  - **Size.** Size is what `du -s --apparent-size --block-size=1` reports: the `lstat` size of the item and of every entry under it. That counts directories and symlinks themselves, counts hard links once (by device and inode), and never follows a symlink.
    - Entries on another mount than `<T>/files` are neither counted nor entered.
    - Set-down, `held`, release, `--json` and the Record all use this one function. The Rust test computes the expected value from the file system instead of hard-coding it.
    - For a held item whose size can't be measured (it is itself a mount, or a directory under it can't be read): `size unknown` on the terminal, `null` in `--json`, and no `LANTEA_SIZE` in the Record.
    - `set-down` still refuses an item it can't measure or walk ("could not be set down"), because it can't check that item for mounts.
  - **Mounts.** One check, `HeldMount::holds`, answers "is this on the same mount as `<T>/files`?" by `statx` mount ID. There is no other mount comparison. It is used by:
    - `set-down`: the top item, then every entry under it;
    - `restore`: the item and the destination's deepest existing ancestor;
    - `release`, `held` and the size walk.

    Details:
    - `set-down --explain`, before `<T>/files` exists, takes the base from `<T>`'s deepest existing ancestor.
    - `release` refuses an item that is itself a mount before `--explain` output, the question and the Record entry.
    - Restore of an item with a mount below it is a rename and is allowed (decision 0018).
    - A release that meets a mount below the item deletes the item's other entries and leaves the mount and the directories above it. It keeps the `.trashinfo` and writes `release-incomplete`.
    - If the kernel doesn't report mount IDs, the reason is `the kernel does not report mount IDs`.
  - **Restore never writes into Held.** The destination is resolved as the real path of its deepest existing ancestor, plus the missing components and the name. If that is `<T>` (resolved) or below it, restore refuses with the "inside your Held items" sentence. This check comes after the outside-the-home check and before the "already exists" check, and applies to `Path=` and `--to` alike.
  - **Restore cleanup.**
    - Only directories that `mkdirat` created are tracked and reported; `EEXIST` is passed over.
    - On any failure after creating some (a later `mkdirat`, an `openat`, or the rename), they are removed deepest first.
    - If any remain, `Nothing was changed.` is replaced by `The directories <dir>, … were created and are still there.`. A refusal without that ending (the "already exists" race) gets the sentence appended instead.
  - **Release failure messages.**
    - A failure before the Record entry ends with `Nothing was changed.`: `<id> could not be released: <reason>. Nothing was changed.`
    - The Record-write refusal keeps its spec wording, `… Nothing was deleted.`
    - After the `release` entry, a failed deletion prints `<id> could not be released completely: <reason>. The release is in the Record, marked as incomplete. Some of it may already be deleted. To see what is held: lantea held`, plus a remedy when there is one. That exact sentence is the `release-incomplete` `MESSAGE`.
    - VM step 12 expects this new wording; its Test data text in the spec still shows the earlier sentence.
  - **Remedies** (release messages only), appended as one final sentence. After `lantea held` the remedy is joined with `. `, so the message reads `… lantea held. Check that you own <path>.`
    - permission denied (`EACCES`) → `Check that you own <path>.`, where `<path>` is the directory whose permissions matter:
      - the parent, for looking up or unlinking an entry;
      - the directory itself, for opening or reading it;
      - `<T>/files` when the held item can't be examined;
      - `<T>/info` when the info file can't be removed.
    - no mount IDs (`ErrorKind::Unsupported`) → `Lantea needs Linux 5.8 or later.`
  - **Creating the Trash.** Only `set-down` creates it, and only after the home and ownership checks pass. A refusal later in `set-down` can leave the Trash directories created. `set-down --explain`, `held`, `restore` and `release` never create it. `set-down --explain` runs the same refusal checks and refuses the same way.
  - **Choosing an id** also skips a candidate whose `files/<candidate>` already exists, even with no info file. If the move fails, the reserved info file is removed again.
  - **Record failure.** For `set-down` and `restore`, the success stdout is printed first, then the stderr sentence, and the command exits 1. For `release`, the entry is sent before anything is deleted. If the `release-incomplete` entry also fails, `The Record could not be written: <reason>.` goes on a second stderr line.
  - **Info file left behind** (restore or release). The action has happened, so the success stdout and, for `restore`, the Record entry are still produced. Then the ".trashinfo could not be removed" sentence goes to stderr and the command exits 1. If the restore's Record entry also fails, both sentences are printed, one per line.
  - **`--to` paths.** `--to` is made absolute against the current directory and `.` parts are dropped. A `..` part is refused as outside the Tended Store. For `Path=`, an empty segment (`//`) is allowed; `.` and `..` segments are refused.
  - **Info files.**
    - The first `Path=` and `DeletionDate=` inside `[Trash Info]` are used. `DeletionDate` must parse as `%Y-%m-%dT%H:%M:%S`, and the decoded `Path=` must start with `/`.
    - An info file larger than 64 KiB is treated as unparsable rather than cut short.
    - Ids and paths that are not valid UTF-8 are shown and written to JSON lossily. `Path=` percent-encodes the raw bytes.
    - `held` counts an info file as unreadable only when its id is valid and its `files/<id>` exists.
  - **Escaping.**
    - Messages are built raw and escaped once when printed; control characters and the backslash become `\xHH`.
    - Lantea's own `printf` format line in `set-down --explain` is the one place where only the file name is escaped, so its `\n` stays readable.
    - Shell quoting writes a single quote as `'"'"'`, so the quoting contains no backslash.
    - stderr lines are kept as a list, so two sentences stay on two lines even when a path contains a newline.
    - In `--explain`, each `logger` value is escaped on its own line.
    - The Record `MESSAGE` is the raw sentence (lossy UTF-8).
  - **Release prompt.** A read error on stdin counts as declining.
  - **Test coverage.**
    - **Mounts:** `mount_situations_table` runs `set-down`, `held`, `held --json`, `restore` and `release --yes` against two situations: an item that is itself a mount (tmpfs and bind), and a mount below the item (tmpfs and bind). That includes the release that meets a mount and the restore with a mount below. It runs in `unshare -Urm`, so no combination needs real root. `pkgs/lantea/default.nix` adds util-linux to the check inputs. VM steps 11 and 12 repeat the cases as real root.
    - **Namespaces denied:** if `unshare -Urm` is denied, the namespace tests fail with the cause (AppArmor's `kernel.apparmor_restrict_unprivileged_userns`, `user.max_user_namespaces`, `kernel.unprivileged_userns_clone`). They are never skipped.
    - **Double Record failure:** `a_failed_second_entry_is_reported_too` fills the socket queue, so the `release-incomplete` send waits until the test closes the socket and then fails.
    - **Ownership:**
      - `held_owned_by_another_user_is_refused` bind-mounts `/nix/store` (`--rbind`) over `~/.local` in a user namespace;
      - `a_home_owned_by_another_user_is_refused` uses `HOME=/nix/store`;
      - `a_missing_home_is_refused_and_not_created`, `a_relative_home_is_refused` and `a_symlink_on_the_way_to_held_is_refused` cover the other home cases.
    - **Restore into Held:** `restore_never_writes_into_held`.
    - **Restore cleanup:** `restore_removes_the_directories_it_created_when_it_fails` uses umask 222. A unit test in `src/held.rs` covers a cleanup that fails.
    - **Other mount:** the "separate mount" set-down refusal is also tested by putting Held on `/dev/shm`.
    - **VM only:** the release question on a terminal, because it needs a pty.
    - **Root assumption:** the tests that use read-only directories assume they don't run as root. That holds for Nix build users and the dev shell.
  - **Strings added beyond the spec:**
    - `Your home directory is not known, because HOME is not set. Nothing was changed.`
    - `Your home directory <H> could not be opened: <reason>. Nothing was changed.`
    - `Your Held items at <T> could not be opened: <reason>. Nothing was changed.`
    - `Your Held items at <T> could not be read: <reason>.`
    - `<path> could not be set down: <reason>. Nothing was changed.`
    - `<id> could not be restored to <path>: <reason>. Nothing was changed.`
    - `<id> could not be released: <reason>. Nothing was changed.` (when the held item can't be examined before release)
    - The reason `the kernel does not report mount IDs`.
    - The remedies `Check that you own <path>.` and `Lantea needs Linux 5.8 or later.` (named in the spec as examples).
    - The clap help text for the four new subcommands and their arguments.
    - Spec strings used as written: the separate-mount sentences; `The directories <dir>, … were created and are still there.`; the "inside your Held items", relative-HOME, missing-home, foreign-home, foreign-Held and symbolic-link refusals; the ".trashinfo could not be removed … It may still be listed by lantea held." sentences; and the "could not be released completely … marked as incomplete …" sentence.
