# 0010 — The Record is tamper-evident, not tamper-proof

**Status:** Accepted · **Date:** 2026-10-09

## Context

The 1.0 brief says the Record "cannot be edited by the Steward". That is easy. Against root on the same machine, no local log can be tamper-proof; claiming otherwise would break "speak plainly".

## Decision

- The Steward cannot write to the Record except through Lantea's own commands.
- The Record is stored in journald with Forward Secure Sealing, plus auditd for system-level events, so tampering by root is *detectable* (`lantea record --verify`).
- Documentation and user-facing text say "tamper-evident", never "tamper-proof". Forwarding to another machine is the way to protect it from local root, and is optional.

## Consequences

- Phase 4 includes key setup for sealing and a verification test with a deliberately tampered journal.
