# 0004 — Glossary: services are "on watch"; versions are "editions"

**Status:** Accepted · **Date:** 2026-10-09

## Context

The 1.0 glossary promised one meaning per term but used two terms twice: *Standing* was the stable track and also the state of a running service ("23 services standing"); *release* was permanent deletion and also the word for versions ("release names").

## Decision

- A governed service that is running as declared is **on watch** ("23 on watch · 0 failing"). *Standing* means only the stable track.
- A version of Lantea is an **edition** (1.0 *Harbour*). *Release* means only permanent removal from Held.
- A governed service that has failed is simply **failing** ("23 on watch · 0 failing"). *Distressed* is only a Condition state.

## Consequences

- `glossary.md` and the brief are updated together.
- User-facing text, CLI output and tests use the new words; the project rules flag the old usage.
