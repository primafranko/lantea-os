# 0002 — Base: NixOS 26.05, then 26.11

**Status:** Accepted · **Date:** 2026-10-09

## Context

The brief says "current NixOS stable". In October 2026 that is NixOS 26.05 "Yarara" (Plasma 6.6, Linux 6.18, systemd-based initrd by default), supported until 31 December 2026. NixOS 26.11 is due around the end of November 2026.

## Decision

Pin `nixpkgs` to `nixos-26.05` now. Move to `nixos-26.11` once it comes out, as a deliberate, reviewed change.

## Consequences

- The move to 26.11 is the first real-world exercise of the Offerings idea, even before Phase 5 exists.
- The scripted initrd is deprecated in 26.05 and removed in 26.11: Lantea uses the systemd initrd from the start (needed for TPM2 unlock anyway).
