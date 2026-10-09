# 0016 — os-release names the Lantea edition; the NixOS base stays visible

**Status:** Accepted · **Date:** 2026-10-09

## Context

With only `system.nixos.distroId` and `distroName` set, NixOS fills the version keys of `/etc/os-release` from its own version: `PRETTY_NAME="Lantea OS 26.05 (Yarara)"`. That pairs Lantea's name with NixOS's version and codename, while Lantea's versions are editions (decision 0004; edition 1.0 *Harbour*). The NixOS base underneath must still be easy to see, and `nixos-version` must keep working.

## Decision

The `discipline` Strand sets, through `system.nixos.extraOSReleaseArgs` and each with `lib.mkDefault`, derived from `lantea.edition`:

```
NAME="Lantea OS"
VERSION="1.0 (Harbour)"
VERSION_ID="1.0"
VERSION_CODENAME=harbour
PRETTY_NAME="Lantea OS 1.0 (Harbour)"
LANTEA_BASE="NixOS 26.05 (Yarara)"
HOME_URL="https://github.com/primafranko/lantea-os"
BUG_REPORT_URL="https://github.com/primafranko/lantea-os/issues"
VENDOR_NAME=""
SUPPORT_URL=""
```

- `BUILD_ID` is left as NixOS sets it (the full NixOS version, e.g. `26.05.20261008.7c8764b`).
- `LANTEA_BASE` is a vendor-prefixed key, which os-release(5) allows. It is built from the NixOS version and codename (`system.nixos.release`, `system.nixos.codeName`), so it follows the base without edits.
- `CPE_NAME` keeps NixOS's own form, `cpe:/o:nixos:nixos:<NixOS version>` (built from `system.nixos.release`), because vulnerability scanners match on it and the packages are NixOS's.
- `SUPPORT_END` is left as NixOS sets it. It is the NixOS base's end of security support, not a promise about how long the Lantea edition is supported.
- `VENDOR_NAME` and `SUPPORT_URL` are not set. NixOS's generator writes every key it knows and marks an unset one with an empty value, so they read `""`; dropping the lines entirely would mean replacing the whole file with `mkForce`, which Lantea does not use.
- `LOGO` stays NixOS's `nix-snowflake` until Lantea's visual identity arrives in Phase 7.
- `nixos-version` is untouched: it takes its version and codename from `system.nixos` at build time, not from os-release.

## Consequences

- Tools that read `VERSION_ID` see the edition (`1.0`), not the NixOS version. Anything that needs the NixOS version reads `BUILD_ID`, `LANTEA_BASE` or `nixos-version`.
- Once the base's `SUPPORT_END` date has passed, newer systemd warns about it at boot; that warning is about the NixOS base, and moving to the next NixOS version clears it.
- The initrd's `initrd-release` is generated from the same contents, so it shows the edition too.
- `/etc/lsb-release` is not changed here; it still carries NixOS's version and codename.
- `system.nixos.extraOSReleaseArgs` is marked internal in nixpkgs. If it is renamed or removed in a later NixOS version, this decision is revisited then; the `discipline` VM test asserts every key above, so the change will not go unnoticed.
