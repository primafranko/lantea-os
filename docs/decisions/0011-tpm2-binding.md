# 0011 — TPM2 unlock binds to Secure Boot state plus PIN

**Status:** Proposed · **Date:** 2026-10-09

## Context

TPM2 unlock that is sealed to PCRs measuring the kernel and initrd breaks on every new generation, which in Lantea means every accepted Offering. Lanzaboote now supports measured boot through systemd-pcrlock.

## Decision

Proposed: bind the LUKS key to PCR 7 (Secure Boot state) plus a PIN, with FIDO2 as an alternative unlock and a recovery key printed at install. Re-evaluate pcrlock during Phase 6.

## Consequences

- Accepting an Offering never breaks unlock.
- Decide before Phase 1's installable disk layout is final, because enrolment happens at install.
