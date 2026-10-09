# SPEC — <system or feature>

<!-- Managed by AiHarness (aih sync). Copy it to docs/spec/SPEC.md; the copy is
     project-owned and written by the architect (/aih:plan … contract). -->

Status: draft · frozen (after the foundation merges; changes only through a contract-change task written by the architect)

## Goal & users

## Glossary
| UI term | Code term | Meaning |
|---|---|---|

## Ownership map
| Area | Owns (globs) | Depends on |
|---|---|---|
| foundation | <!-- e.g. packages/contracts/**, packages/ui/**, migrations --> | — |
| <area> | <!-- e.g. apps/web/src/features/<area>/**, apps/api/src/routes/<area>/** --> | foundation |

No area may edit: <!-- shared files -->. Read-only for all areas: <!-- e.g. packages/contracts/** -->.

## Contract
- Types: <!-- path --> — the source of truth; areas import, never redefine.
- API:

  | Method | Path | Request schema | Response | Errors | Who may call |
  |---|---|---|---|---|---|

- UI: design-system components to use; screens and routes; i18n key namespace per area.
- Data: tables and which area owns them; who writes migrations.

## Cross-cutting rules
- Auth: <!-- e.g. Entra ID — token validated server-side; roles from the roles claim -->
- Errors, logging, language: <!-- … -->

## Integration & QA
- Merge order: foundation → <!-- areas --> → integration.
- Integration success check: <!-- one command -->
- QA: routes × viewports × states (see the QA spec).

## Open questions
