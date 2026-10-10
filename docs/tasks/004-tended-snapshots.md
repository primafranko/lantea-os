# Task 004 — tended-snapshots

| Field | Value |
|---|---|
| Area / owner | backups (`modules/core/backups/**`, `tests/**`) · Primož |
| Risk | high (scheduled deletion of old snapshots by retention) |
| Depends on | 003 (merged) |
| Contract | `docs/brief.md` §4 "Snapshots and backup", §6 Phase 1 · decisions 0006, 0017, 0019, 0020 (from task 003) |
| Model hint | sonnet |

## Goal
On a machine with the Lantea layout, the Tended Store (`@tended`, at `/home`) is snapshotted by btrbk every hour into `@snapshots`, read-only, with a declared retention. Every snapshot that retention deletes leaves an entry in the Record. Phase 1 is complete when this task is done.

## Context
- Brief §4: btrbk for scheduled local snapshots with hourly/daily/weekly retention. Send/receive to an external disk and restic are later.
- Decision 0006: held items are inside `@tended`, so they are in its snapshots. That is honest, and it is also the only way back from a `release` (task 002).
- Task 003 builds the layout and its VM test `tests/tended-store-layout.nix`. This task extends that test instead of formatting another disk.
- Files that matter: `modules/core/default.nix`, `modules/core/backups/README.md`, `modules/core/tended-store/default.nix` (from 003), `tests/tended-store-layout.nix` (from 003), the NixOS `services.btrbk` module.

## Assumptions
Primož answered the question on 2026-10-10 (see "Resolved questions"); these hold.
- New options (L-3):
  - `lantea.backups.snapshots.enable` (bool, default `false`). Snapshots are something the machine's owner turns on once, by principle 5. Retention deletes old snapshots, which is deletion.
  - `lantea.backups.snapshots.onCalendar` (str, default `"hourly"`).
  - `lantea.backups.snapshots.preserveMin` (str, default `"2d"`).
  - `lantea.backups.snapshots.preserve` (str, default `"24h 7d 4w"`).
- Enabling it without `lantea.tendedStore.layout.enable` fails evaluation, with a message naming both options.
- It is one btrbk instance named `lantea`, through `services.btrbk.instances.lantea`. Only `@tended` is snapshotted, into the `@snapshots` subvolume, with snapshot name `tended` (btrbk appends its timestamp, for example `tended.20261009T1400`). `@root` is declared, `@nix` is reproducible, and `@record` is journald's own.
- The btrbk volume is reached through the mounted paths (`/home` as the subvolume, `/.snapshots` as `snapshot_dir`). If btrbk refuses that, mount the top level at a private path (for example `/run/lantea/btrfs-top`, `subvolid=5`) through the module. Record which one worked in decision 0021.
- btrbk's own unit and timer are kept as NixOS creates them (`btrbk-lantea.service`, `btrbk-lantea.timer`).
- **Every snapshot btrbk deletes leaves a Record entry** (answer 1). It uses the decision 0017 format with a new action, recorded in decision 0021:
  - `LANTEA_ACTION=snapshot-removed`
  - `MESSAGE_ID=b7d2b400137d48b6b3febe65d4059d7a`
  - `MESSAGE=Removed snapshot <name> of the Tended Store, following its retention rules.`
  - `LANTEA_PATH=/.snapshots/<name>`
  - plus `PRIORITY=5`, `SYSLOG_IDENTIFIER=lantea` and `LANTEA_RECORD_VERSION=1`

  `_UID` is 0: the system acted. The entries are written after btrbk ran, from btrbk's own account of what it deleted. For example, btrbk's `transaction_log` to a file under `/run/lantea`, read by an `ExecStartPost` of `btrbk-lantea.service` that sends one entry per `delete` line with `logger --journald`. Do not compare directory listings. If btrbk ran but its account cannot be read, the service fails, so the gap is visible. Record the mechanism in decision 0021.

## Open questions (blocking)
None.

## Resolved questions (Primož, 2026-10-10)
1. **Retention and schedule:** yes. Hourly, only `@tended`, `snapshot_preserve_min 2d`, `snapshot_preserve 24h 7d 4w`, off unless the machine's owner turns it on. In addition:
   - every snapshot btrbk deletes leaves a Record entry (above);
   - the Phase 8 installer asks the owner once whether to turn snapshots on (follow-up, see Out of scope).

## Files in scope
- `modules/core/backups/default.nix` (new) — the `lantea.backups.snapshots.*` options and the btrbk instance
- `modules/core/backups/README.md` — remove it, or reduce it to restic "later" (match the `discipline` Strand's style)
- `modules/core/default.nix` — import `./backups`
- `tests/tended-store-layout.nix` — enable snapshots and assert them
- `docs/decisions/0021-tended-snapshots.md` (new), `docs/decisions/README.md` — index row
- `docs/architecture.md` — `backups` row: btrbk done
- `README.md` — Status line for Phase 1

## Acceptance criteria
- [ ] The four options exist with descriptions and the defaults above. Enabling snapshots without the layout fails evaluation, and the message names `lantea.backups.snapshots.enable` and `lantea.tendedStore.layout.enable`.
- [ ] The layout test machine sets `lantea.enable = true`, so the Steward and `lantea` exist. If task 003 did not set it, this task adds it.
- [ ] In the booted layout test machine, with `lantea.backups.snapshots.enable = true`:
  - `systemctl list-timers btrbk-lantea.timer` shows the timer.
  - `/etc/btrbk/lantea.conf` contains `snapshot_preserve_min 2d` and `snapshot_preserve 24h 7d 4w`.
- [ ] After `echo kept > /home/steward/kept.txt` and `systemctl start btrbk-lantea.service`, the service exits successfully, and exactly one new entry matching `^tended\.\d{8}T\d{4}` appears in `/.snapshots`.
- [ ] In that entry, `btrfs property get -ts <snapshot> ro` prints `ro=true`, and `<snapshot>/steward/kept.txt` contains `kept`.
- [ ] A file held with `lantea set-down` before the snapshot is present under `<snapshot>/steward/.local/share/Trash/files/` (decision 0006: Held is in the snapshot).
- [ ] **A snapshot removed by retention leaves a Record entry.**
  - Setup: as root in the test VM, create an old read-only snapshot by hand, `btrfs subvolume snapshot -r /home /.snapshots/tended.20200101T0000`, which is past every retention rule. Then `systemctl start btrbk-lantea.service`.
  - The service exits successfully, and `/.snapshots/tended.20200101T0000` is gone.
  - `journalctl -t lantea -o json` (with `wait_until_succeeds`) holds exactly one `snapshot-removed` entry with every field above: `LANTEA_PATH=/.snapshots/tended.20200101T0000`, the exact `MESSAGE` and `MESSAGE_ID=b7d2b400137d48b6b3febe65d4059d7a`, and `_UID=0`.
  - A btrbk run that deletes nothing writes no `snapshot-removed` entry.
- [ ] With `lantea.backups.snapshots.enable` left at `false`, there is no `btrbk-lantea` unit (`vm-dev` unchanged).
- [ ] Decision 0021 records the retention, the scope (`@tended` only), how btrbk reaches the volume, and the `snapshot-removed` Record action (fields, `MESSAGE_ID`, how entries are produced). `README.md` Status reads `Phase 1 — The Tended Store: done (build machine: aarch64; CI: x86_64).` in addition to the Phase 0 line.
- [ ] `aih verify` prints `VERIFY: PASS`.

## Test data
- `/home/steward/kept.txt` containing `kept\n`.
- One held item made with `su - steward -c 'printf held > ~/held.txt && lantea set-down ~/held.txt'`.

## Non-functional
- Retention only ever deletes btrbk's own snapshots in `@snapshots`, never anything in `@tended`.
- Both architectures (L-9).

## Data & migrations
No existing snapshots anywhere. The snapshot naming (`tended.<timestamp>`) is fixed by decision 0021, and later tooling (`lantea condition`'s "N snapshots retained") reads it.

## Rollback
Set `lantea.backups.snapshots.enable = false` or revert the PR. Existing snapshots stay in `@snapshots` until removed by hand (`btrfs subvolume delete`). Nothing is deleted automatically once btrbk no longer runs.

## Verification
Success check — run on the build machine; exits 0 when the local part of this task is done (decision 0019):
```
nix flake check --no-build && nix build --no-link -L .#checks.aarch64-linux.tended-store-layout.driver
```
Also required: `aih verify` prints `VERIFY: PASS`, and **the `tended-store-layout` VM test passes in CI** on this task's pull request. The task is not done until it does.

## Known pitfalls
- Flakes only see tracked files: `git add` new files before building.
- Don't boot the VM test locally to "prove" it. lantea-bench crashes aarch64 guests (decision 0019). CI is the verdict; `aih run check.vm` only on request.
- U-2: do not weaken task 003's layout assertions to make room for this one; add to them.
- L-3: options under `lantea.*`; the btrbk instance (an addition to an attribute set) sits behind `lantea.backups.snapshots.enable`.
- Starting `btrbk-lantea.service` is done only inside the test VM, never on the build machine (L-6).

## Docs & changelog
- Decision 0021; `docs/architecture.md` backups row; `README.md` Status: Phase 1 done.

## Out of scope
- Send/receive to an external disk, restic off-site (later).
- Snapshot count in `lantea condition` (Phase 3). Restoring from a snapshot with `lantea` (not planned yet).
- Snapshots of `@root`, `@nix` or `@record`.
- Follow-up for Phase 8 (Primož, 2026-10-10): the installer asks the machine's owner once whether to turn snapshots on, and sets `lantea.backups.snapshots.enable` from the answer.

## Implementer notes
