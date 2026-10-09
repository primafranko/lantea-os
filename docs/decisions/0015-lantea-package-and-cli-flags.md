# 0015 — How Core installs `lantea`, and how its global flags combine

**Status:** Proposed · **Date:** 2026-10-09

## Context

Phase 0 (task 001) puts `lantea` on every user's PATH from the `discipline` Strand, and makes `--json`, `--explain` and `--yes` global flags. The brief does not say where the module gets the package from, nor what happens when `--json` and `--explain` are given together.

## Decision

- The `discipline` Strand builds `lantea` from the flake's own source with `pkgs.callPackage ../../../pkgs/lantea { }`. There is no `lantea.package` option and no overlay yet: a machine's flake that imports `nixosModules.default` gets the `lantea` of the same Lantea edition, built for its own architecture.
- `--explain` takes precedence over `--json`: when both are given, the command prints the real commands it would run, as plain text, and runs nothing.
- `--yes` is accepted by every command; commands with nothing to confirm ignore it.

## Consequences

- The CLI and the modules always come from the same Lantea revision; a user cannot swap in another build without an overlay of their own. A `lantea.package` option can be added later if that is needed.
- A machine-readable form of `--explain` (for example a JSON list of commands) would need its own decision.
