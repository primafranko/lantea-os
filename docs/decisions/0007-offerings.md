# 0007 — An Offering is one generation; accepting it updates the declared lock

**Status:** Accepted · **Date:** 2026-10-09

## Context

A NixOS update is not a set of independent package updates: it is a new `flake.lock` that produces a new system generation. A per-package checklist cannot be honoured without pinning packages by overlay. If an accepted update is activated but the lock file in the user's configuration stays old, the next rebuild silently reverts it; if a rollback leaves the new lock in place, the next rebuild silently re-applies it.

## Decision

- An Offering is one built-but-not-activated generation together with the `flake.lock` that produced it. Its diff (nvd) is shown per package; acceptance is all or nothing, at most per flake input.
- `lantea accept` activates the offering at the **next restart** by default (`switch-to-configuration boot`); `--now` switches immediately.
- Accepting writes the offering's `flake.lock` into the machine's flake. `lantea return` activates a previous generation and restores its lock.
- If the running generation and the declared lock ever disagree, Condition says so in plain words.

## Consequences

- No surprise reboots and no silent reversion.
- Lock files must be stored alongside generations (or recoverable from the generation's metadata) so `return` can restore them.
