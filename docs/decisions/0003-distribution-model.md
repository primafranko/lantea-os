# 0003 — Lantea is a flake that each machine's own flake imports

**Status:** Accepted · **Date:** 2026-10-09

## Context

There are two ways to ship a NixOS layer: a configuration Lantea owns, or a set of modules that the user's own configuration imports. Offerings (updating the lock file) and `lantea inherit` (telling defaults from user choices) both depend on which one it is.

## Decision

Lantea exposes `nixosModules` (core, experience, default), packages and checks. Each machine has its own flake, owned by its user, that imports Lantea as an input. Lantea sets its single-value defaults with `lib.mkDefault`; what it adds to lists and attribute sets (packages, users, units) merges normally and sits behind `lantea.*.enable` options.

## Consequences

- The user's own settings win without `mkForce`.
- `lantea inherit` can use the module system's definition locations and priorities to report provenance.
- Offerings update the machine flake's `flake.lock` (both `nixpkgs` and `lantea` inputs), not a lock Lantea owns.
- The development hosts (`vm-dev`, `vm-dev-aarch64`) live in this repo for testing only.
