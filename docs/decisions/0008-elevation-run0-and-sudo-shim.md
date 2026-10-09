# 0008 — Elevation via run0, with a sudo shim that explains itself

**Status:** Accepted · **Date:** 2026-10-09

## Context

The brief chooses run0 and "no sudo". It also promises that an experienced Linux administrator needs to learn nothing new — and every administrator types `sudo`.

## Decision

- Elevation is run0 (systemd + polkit). The Keeper is the polkit admin identity; the Steward is not.
- `sudo` exists as a small shim: it prints one line ("Lantea uses run0 — running: run0 …") and runs the same command through run0.
- The real `sudo` package is not installed.

## Consequences

- Habits and scripts keep working; nothing is hidden (interpretation, not deception).
- run0 has no credential caching by default; repeated elevation asks again unless polkit's `auth_admin_keep` is chosen. That choice is Phase 6's.
