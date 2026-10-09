# 0005 — A minimal Record and the accounts arrive early

**Status:** Accepted · **Date:** 2026-10-09

## Context

In the 1.0 plan, Phase 1's test needs the Record (Phase 4), and Phase 2's `stand-down` needs Steward/Keeper and polkit (Phase 6). Each phase's acceptance test would depend on a later phase.

## Decision

- Phase 0 creates the Steward and Keeper accounts (Steward unprivileged, Keeper administrative) in the `discipline` module.
- Phase 1 starts with a minimal Record writer: structured journald entries under the `lantea` identifier.
- Phase 4 *hardens* the Record (sealing, retention, auditd, verification); Phase 6 *hardens* elevation (run0, sudo shim, polkit rules).

## Consequences

- Every phase's acceptance test can be written against what already exists.
- The Record's on-disk format is fixed early; Phase 4 must stay compatible with Phase 1's entries.
