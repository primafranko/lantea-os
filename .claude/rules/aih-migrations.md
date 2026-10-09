---
paths:
  - "**/migrations/**"
  - "**/drizzle/**"
  - "**/database/**"
---
# Database migrations — core rules

<!-- Managed by AiHarness (aih sync). Do not edit in a project. Stack-specific
     migration rules live in the stack packs. -->

- WHEN a migration has been applied anywhere (merged, deployed, or run on a shared database) → never edit, reorder or delete it; write a new migration. <!-- id=U-5 n=0 since=2026-10 check=none -->
- WHEN renaming or dropping a column or table that running code uses → expand/contract: add the new one, ship code that writes both and reads the new one, backfill, drop the old one in a later release. <!-- id=U-6 n=0 since=2026-10 check=none -->
- WHEN a migration loses data or can't be reversed → say so in the spec's Rollback section with the restore path (backup, export) before writing it. <!-- id=U-7 n=0 since=2026-10 check=none -->
- WHEN adding a NOT NULL column to a table that has rows → give it a default or backfill it first, then add the constraint. <!-- id=U-8 n=0 since=2026-10 check=none -->
- WHEN changing data in bulk → keep the backfill separate from the schema change, run it in batches, and make it safe to re-run. <!-- id=U-9 n=0 since=2026-10 check=none -->
- WHEN adding an index to a large live table → build it without blocking writes (PostgreSQL: `CREATE INDEX CONCURRENTLY`, outside a transaction) or schedule a window in the spec. <!-- id=U-10 n=0 since=2026-10 check=none -->
- WHEN a tool generates migration SQL → read the generated SQL before committing; DROP, type changes and other data loss need the spec's explicit approval. <!-- id=U-11 n=0 since=2026-10 check=none -->
- WHEN running migrations or resets from a session → only against the local or test database; never point the tool at a shared or production database. <!-- id=U-12 n=0 since=2026-10 check=hook:guard -->
