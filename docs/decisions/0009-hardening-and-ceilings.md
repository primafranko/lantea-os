# 0009 — systemd sandboxing first; MemoryHigh as the ceiling

**Status:** Accepted · **Date:** 2026-10-09

## Context

AppArmor on NixOS has few maintained profiles, and store paths make writing them awkward. systemd's own sandboxing (`ProtectSystem`, `PrivateTmp`, `NoNewPrivileges`, `SystemCallFilter`, …) works the same everywhere and fits the Watch, where every governed service already declares purpose and limits. For memory, `MemoryMax` kills a service at the limit, while `MemoryHigh` slows it down and reclaims memory first.

## Decision

- Every service on watch declares a sandbox alongside its purpose and ceilings; `lantea watch` shows its `systemd-analyze security` exposure.
- The declared memory ceiling maps to `MemoryHigh`; `MemoryMax` is set above it as a backstop.
- AppArmor is added later, only where maintained profiles exist.

## Consequences

- "Take no more than needed" degrades gracefully instead of killing services at the limit.
- Exposure scores give Condition and the user a concrete hardening measure.
