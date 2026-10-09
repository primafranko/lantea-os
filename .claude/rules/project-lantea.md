---
paths:
  - "**/*.nix"
  - "pkgs/**"
  - "tests/**"
  - "modules/**"
  - "hosts/**"
  - "docs/**"
---
# Lantea project rules

<!-- Project-owned. Seeded from the brief and decision records; /aih:distill may add to it. -->

- WHEN writing user-facing text (CLI output, messages, notifications, user docs) → plain, calm and specific: what happened, why it matters, what to do. No unexplained codes, no percentages for health, no "tamper-proof", no exclamation marks, no marketing voice. <!-- id=L-1 n=0 since=2026-10 check=none -->
- WHEN using an Atlantean term → use it exactly as defined in `docs/glossary.md`; a new concept goes into the glossary and `docs/brief.md` §2 in the same change. Services are "on watch" (or "failing"), versions are "editions", "release" only means permanent removal. <!-- id=L-2 n=0 since=2026-10 check=none -->
- WHEN defining a NixOS option → put it under `lantea.*` with a type, a default and a `description`; set single-value defaults with `lib.mkDefault` (never `mkForce`); additions to lists and attribute sets merge normally, behind an enable option. <!-- id=L-3 n=0 since=2026-10 check=none -->
- WHEN adding or changing a Core module → add or update its VM test in `tests/` in the same change and wire it into `checks`; assert behaviour (the Record entry, the exact message, the refusal), not only that a unit started. <!-- id=L-4 n=0 since=2026-10 check=none -->
- WHEN adding a `lantea` subcommand → support `--json` and `--explain` (print the exact underlying commands); exit `0` success, `1` failed or refused, `2` usage error; destructive actions ask unless `--yes`. <!-- id=L-5 n=0 since=2026-10 check=none -->
- WHEN a command would change the build machine (`nixos-rebuild switch|boot|test`, disk, boot or TPM tools, `systemctl` on system units, `sudo`/`run0`) → don't run it; do it inside a test VM, or stop and ask Primož. <!-- id=L-6 n=0 since=2026-10 check=hook:guard -->
- WHEN Core seems to need the desktop → it doesn't; put presentation in `modules/experience/` and read Core's public outputs (`/run/lantea/condition.json`, `lantea --json`, the Record). <!-- id=L-7 n=0 since=2026-10 check=none -->
- WHEN making a choice the brief and `docs/decisions/` don't cover → add a decision record `docs/decisions/NNNN-<slug>.md` (Status, Context, Decision, Consequences) in the same change; never contradict an Accepted record without proposing one that supersedes it. <!-- id=L-8 n=0 since=2026-10 check=none -->
- WHEN writing Nix that builds packages or tests → don't assume an architecture; it must evaluate and build on both `x86_64-linux` and `aarch64-linux`. <!-- id=L-9 n=0 since=2026-10 check=none -->
