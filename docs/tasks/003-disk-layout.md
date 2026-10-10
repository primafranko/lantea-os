# Task 003 — disk-layout

| Field | Value |
|---|---|
| Area / owner | tended-store (`modules/core/tended-store/**`, `flake.nix`, `tests/**`) · Primož |
| Risk | high (disk layout and encryption: wrong choices are fixed at install and costly to undo) |
| Depends on | 002 (merged) |
| Contract | `docs/brief.md` §4 "Disk layout", §6 Phase 1 · decisions 0003, 0006, 0011 (Proposed), 0012, 0014 (accepted in this task), 0019 |
| Model hint | opus |

## Goal
Lantea declares its installable disk layout with disko: GPT, an EFI system partition, LUKS2, and Btrfs inside with `@root`, `@nix`, `@tended` (at `/home`), `@record` and `@snapshots`, plus a read-only, empty `@root-blank` taken when the layout is created. A VM test formats a blank disk with it, installs, boots through the LUKS prompt, and shows every subvolume mounted where it belongs.

## Context
- Brief §4: disko, LUKS2 full-disk encryption, Btrfs subvolumes `@root`, `@nix`, `@tended`, `@record`, `@snapshots`. Decision 0006: `@tended` is mounted at `/home`, and there is no `@held`. Decision 0014 (Proposed; this task accepts it, per Primož 2026-10-10): `@root-blank` at install. Decision 0019: VM tests are booted in CI; locally the driver is built without booting.
- Decision 0011 (Proposed) says the TPM2 binding must be decided before the installable layout is final, because enrolment happens at install. This task only declares LUKS2 with a passphrase. Enrolment belongs to the installer (Phase 8) and Secure Boot (Phase 6), so the layout does not depend on the PCR choice.
- Decision 0003: Lantea is imported by each machine's flake, so the layout is a Core option a host turns on, not a host file.
- Files that matter: `flake.nix`, `modules/core/default.nix`, `modules/core/tended-store/README.md`, `tests/default.nix`, `hosts/vm-dev/default.nix` (unchanged: it keeps qemu-vm's own disk).

## Assumptions
Primož answered the questions on 2026-10-10 (see "Resolved questions"); these hold.
- `disko` becomes the second flake input, `github:nix-community/disko/<latest release tag>` with `inputs.nixpkgs.follows = "nixpkgs"`. `flake.lock` gains only the `disko` node. The `nixpkgs` revision does not change.
- `nixosModules.core` imports `disko.nixosModules.disko` next to `./modules/core`. The disko module does nothing until `disko.devices` is set.
- New options, each with a type, a default and a description (L-3):
  - `lantea.tendedStore.layout.enable` (bool, default `false`).
  - `lantea.tendedStore.layout.device` (str, no default, required when enabled; for example `/dev/nvme0n1`).
  - `lantea.tendedStore.layout.passwordFile` (null or str, default `null`, meaning disko asks for the passphrase when formatting; tests set a file).
- The layout is a pure function in `modules/core/tended-store/layout.nix` (new), `{ device, passwordFile }: <disko.devices attrset>`. The module sets `disko.devices` from it under `mkIf enable`. The test reuses the same function, so test and module cannot drift.
- Layout: GPT; ESP 1 GiB, vfat, at `/boot`, mount options `umask=0077`; the rest is LUKS2, mapper name `lantea`, `allowDiscards = true`. Inside it, Btrfs with mount options `compress=zstd` and `noatime`:
  - `@root` → `/`
  - `@nix` → `/nix`
  - `@tended` → `/home`
  - `@record` → `/var/log`, with `neededForBoot = true` so journald finds it early
  - `@snapshots` → `/.snapshots`
- No swap partition.
- `@root-blank` is made in the Btrfs content's `postCreateHook`: mount the top level (`subvolid=5`), `btrfs subvolume snapshot -r <top>/@root <top>/@root-blank`, unmount. It is never mounted.
- `vm-dev` stays on qemu-vm's disk. The layout is exercised only by the VM test, with disko's `makeDiskoTest`, which formats, installs, and boots from the layout. There is no `workstation` host in this task (answer 3).

## Open questions (blocking)
None.

## Resolved questions (Primož, 2026-10-10)
1. **`disko` as a flake input:** yes. It follows `nixpkgs`, is pinned to disko's latest release tag, and `flake.lock` gains only the disko node.
2. **Decisions 0014 and 0011:** accept 0014 in this task's pull request (its Status line and index row only, per 0001). 0011 stays Proposed. Phase 1 uses a passphrase-only LUKS2; TPM2/FIDO2 enrolment comes in Phases 6 and 8.
3. **Workstation host:** none yet. It comes when Primož names the hardware. The layout is proven in a VM only.
4. **Layout:** as listed above: `@record` at `/var/log`, `@snapshots` at `/.snapshots`, ESP 1 GiB at `/boot`, LUKS mapper `lantea`, `compress=zstd,noatime`, no swap.

## Files in scope
- `flake.nix` — `disko` input; `nixosModules.core` imports the disko module; `checks` gets the layout test
- `flake.lock` — the added `disko` node only
- `modules/core/default.nix` — import `./tended-store`
- `modules/core/tended-store/default.nix` (new) — the `lantea.tendedStore.layout.*` options
- `modules/core/tended-store/layout.nix` (new) — the disko layout function
- `modules/core/tended-store/README.md` — mention the layout, or remove the file if `default.nix` now documents the Strand (match how `discipline` is done)
- `tests/default.nix` — wire `tended-store-layout`
- `tests/tended-store-layout.nix` (new) — the `makeDiskoTest` VM test
- `docs/decisions/0014-impermanence-ready-layout.md` — Status → Accepted (Status line only, per 0001)
- `docs/decisions/0020-disk-layout.md` (new), `docs/decisions/README.md` — index rows (0020 added, 0014 Accepted)
- `docs/architecture.md` — note in the Core modules table that `tended-store` now has the layout

## Acceptance criteria
- [ ] `nix flake metadata --json | jq -r '.locks.nodes | keys[]'` lists exactly `disko`, `nixpkgs`, `root`. The `nixpkgs` node's `rev` is unchanged (`7c8764b7c7b09b34f632464276218ef9090eaa11`).
- [ ] The three options exist with descriptions. Enabling the layout without a `device` fails evaluation with a message that names `lantea.tendedStore.layout.device`.
- [ ] `nix eval .#nixosConfigurations.vm-dev-aarch64.config.disko.devices --json` is `{}` or has no `disk` entries: `vm-dev` is unchanged and still builds with `nixos-rebuild build-vm --flake .#vm-dev-aarch64`.
- [ ] In the booted test machine (after the LUKS passphrase is entered on the console):
  - `findmnt -no FSTYPE,OPTIONS /` contains `btrfs` and `subvol=/@root`.
  - Likewise `/nix` has `subvol=/@nix`, `/home` has `subvol=/@tended`, `/var/log` has `subvol=/@record`, and `/.snapshots` has `subvol=/@snapshots`. All five have `compress=zstd` and `noatime`.
  - `findmnt -no SOURCE /` is `/dev/mapper/lantea[/@root]`.
  - `cryptsetup luksDump` of the LUKS partition shows `Version:` `2`.
  - `/boot` is vfat.
- [ ] With the top level mounted at `/mnt` (`mount -o subvolid=5 /dev/mapper/lantea /mnt`):
  - `btrfs subvolume list /mnt` lists `@root`, `@nix`, `@tended`, `@record`, `@snapshots` and `@root-blank`, and no other subvolume.
  - `btrfs property get -ts /mnt/@root-blank ro` prints `ro=true`.
  - `ls -A /mnt/@root-blank` prints nothing.
- [ ] `/var/log/journal` exists on `@record`, and `journalctl --list-boots` works after boot.
- [ ] The test is `checks.<system>.tended-store-layout` for both systems. `nix flake check --no-build` evaluates it on both.
- [ ] Decision 0020 records the layout (partitions, mapper name, mount points, mount options, no swap, `@root-blank`, why `vm-dev` keeps qemu-vm's disk) and is indexed in `docs/decisions/README.md`. Decision 0014's Status is Accepted, in the record and the index.
- [ ] `aih verify` prints `VERIFY: PASS`.

## Test data
- Test passphrase file: `/tmp/secret.key`, holding a fixed test passphrase. It is a test fixture, never a real secret. Follow disko's own `tests/luks-btrfs-subvolumes.nix` for how the passphrase file is provided and typed at boot.
- Disk: the one disko's test library provides (it rewrites the device paths). Size ≥ 4 GiB.

## Non-functional
- Evaluates on `x86_64-linux` and `aarch64-linux`. The test boots with EFI on both, as `makeDiskoTest` does by default (L-9).
- No real disk is touched. Formatting happens only inside the test VM. The guard denies `disko` and `mkfs` on the build machine, and that is correct.

## Data & migrations
- No machine runs Lantea's layout yet, so nothing is migrated. From the first install onwards, the layout is fixed per machine: changing subvolume names or mount points later needs a written migration (a new decision and task), never an edit of this layout for installed machines.
- `@root-blank` costs only metadata.

## Rollback
Revert the PR. Machines formatted with this layout keep working (their `fileSystems` are in their own generation), but a revert removes the declarations they were built from, so do not revert after a real install. Write a superseding decision and a migration task instead.

## Verification
Success check — run on the build machine; exits 0 when the local part of this task is done (decision 0019):
```
nix flake check --no-build && nix build --no-link -L .#checks.aarch64-linux.tended-store-layout.driver
```
Also required: `aih verify` prints `VERIFY: PASS`, and **the `tended-store-layout` VM test passes in CI** on this task's pull request. The task is not done until it does.

## Known pitfalls
- `makeDiskoTest`'s exact attribute path and arguments (`disko-config`, `extraTestScript`, `bootCommands`, `extraSystemConfig`, `efi`) must be read from the pinned disko source, not guessed. Write what you used in Implementer notes.
- Flakes only see tracked files: `git add` new files before `nix build`.
- Don't boot the VM test locally to "prove" it. lantea-bench crashes aarch64 guests (decision 0019, inbox lesson). CI is the verdict; `aih run check.vm` only on request.
- Inbox (stack:nix): group options under one top-level key per module (`disko.*`, `fileSystems.*`); statix rejects repeated keys.
- L-3: options under `lantea.*` with type, default and description; `mkDefault` for single values, never `mkForce`. CLAUDE.md "Never guess": no real disk device names, and no `flake.lock` change beyond the disko node.

## Docs & changelog
- Decision 0020 (new); 0014 → Accepted; `docs/architecture.md` Core table note.
- No README Status change (task 004 does it).

## Out of scope
- btrbk snapshots (task 004).
- TPM2/FIDO2 enrolment, lanzaboote, Secure Boot (Phases 6 and 8); the decision on 0011.
- The `workstation` host and any real hardware: it waits until Primož names the hardware (answer 3).
- Switching `vm-dev` to the disko layout.
- The "Leave no mark" rollback to `@root-blank` at boot (later).

## Implementer notes
