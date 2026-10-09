# 0014 — Take a blank @root snapshot at install

**Status:** Proposed · **Date:** 2026-10-09

## Context

"Leave no mark" (root erased at every boot) is optional and comes late. Enabling it later on an installed machine requires a blank snapshot of `@root` taken when the system was empty; without it, enabling the profile would mean reinstalling.

## Decision

Proposed: the Phase 1 disko layout creates `@root-blank` (a read-only snapshot of the empty `@root`) at install, even though the profile stays off.

## Consequences

- Costs nothing at install; keeps the later profile a configuration change instead of a reinstall.
