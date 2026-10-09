# 0001 — Record architecture decisions

**Status:** Accepted · **Date:** 2026-10-09

## Context

Lantea is built over many phases by a human and coding agents in turn. Choices made in one session are invisible to the next unless written down, and an agent will otherwise re-argue or silently reverse them.

## Decision

Every significant decision is recorded here as a short file: `NNNN-slug.md` with Status, Context, Decision and Consequences. Status is one of Proposed, Accepted, Superseded by NNNN. A choice not covered by the brief gets a record in the same change that makes it.

## Consequences

- Agents read `docs/decisions/` before planning and must not contradict an Accepted record without proposing a superseding one.
- Records are never edited after acceptance except to change Status; a change of mind is a new record.
