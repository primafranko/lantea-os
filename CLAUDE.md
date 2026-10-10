# Lantea OS

An opinionated layer of NixOS modules, packages and themes whose architecture embodies the Atlantean "Discipline" from the novel *Aria Sum*: declared, recoverable, governed, plainly spoken, recorded, never forced. Built by Primož Franko with coding agents. "Done well" means: every Atlantean term maps to a real Linux/Nix concept that stays visible, every Core module has a passing VM test, and every user-facing sentence is calm and specific. The full brief is `docs/brief.md`; read it before planning.

## Stack
- Language / runtime: Nix (flakes, NixOS 26.05 modules) · Rust stable (clap, serde) for `pkgs/lantea` · Python only inside NixOS test scripts
- Frameworks: NixOS module system · `pkgs.testers.runNixOSTest` · disko, lanzaboote, btrbk, restic (from later phases)
- Data: none (state lives in the system: Btrfs, journald, `/run/lantea`)
- Auth: Steward (unprivileged) / Keeper (admin, elevation via run0) — see `docs/glossary.md`
- Package manager / monorepo: Nix flake; one Cargo workspace in `pkgs/lantea`
- Harness: packs and domains are listed in `.claude/harness.json`.

## Map
| Path | What |
|---|---|
| `flake.nix` | Outputs: nixosModules, nixosConfigurations (vm-dev, vm-dev-aarch64), packages, checks, formatter, devShells |
| `modules/core/<strand>/` | Core Strands — headless, required. Never import Experience |
| `modules/experience/` | Desktop presentation only; reads Core's public outputs |
| `hosts/vm-dev/` | The development VM host module (both architectures) |
| `pkgs/lantea/` | Rust workspace: `lantea` CLI, later the condition daemon and a shared lib |
| `tests/` | One `runNixOSTest` per Strand, wired into `checks` |
| `docs/brief.md`, `docs/glossary.md` | The brief and the fixed vocabulary |
| `docs/decisions/` | Decision records — read before planning; don't contradict an Accepted one |
| `docs/tasks/` | Task specs (AiHarness) |
| `docs/notes/` | Proposals not yet decided (e.g. Phase 7 Experience) |

## Commands
The gates are defined in `.claude/harness.json`; hooks and `aih verify` run them. This table is for humans.

| Purpose | Command |
|---|---|
| Verify (all gates) | `aih verify` |
| One named gate | `aih run <name>` (e.g. `aih run check`) |
| All checks incl. VM tests | `nix flake check -L` |
| One VM test | `nix build .#checks.aarch64-linux.<name> -L` |
| Dev VM (build machine) | `nixos-rebuild build-vm --flake .#vm-dev-aarch64` then `./result/bin/run-*-vm` (headless: `QEMU_OPTS=-nographic`) |
| Dev shell | `nix develop` |
| Format | `nix fmt` |

## Language
- Code, identifiers, comments, commits, specs: English.
- UI text (CLI output, messages, docs for users): English, plain and calm — see "Speak plainly" in `docs/brief.md` §1.
- Documents: English.

## Conventions
- The glossary in `docs/glossary.md` is fixed. A new term goes into the glossary and the brief first, in the same change. Services are "on watch" (or "failing"); versions are "editions"; "release" only means permanent removal.
- Every NixOS option Lantea defines lives under `lantea.*` and has a `description`.
- Lantea sets single-value defaults with `lib.mkDefault` (never `mkForce`) so user settings win and `lantea inherit` can trace provenance; additions to lists and attribute sets merge normally behind a `lantea.*.enable` option.
- Every Core module ships with a VM test in `tests/`, written with the feature, and wired into `checks`.
- Core never imports Experience; Experience reads only `/run/lantea/condition.json`, `lantea --json` and the Record.
- Every `lantea` command supports `--json` and `--explain`; exit codes `0` success, `1` failed or refused, `2` usage error; destructive actions ask unless `--yes`.
- A choice the brief doesn't cover → a decision record in `docs/decisions/` in the same change.
- Never patch nixpkgs; use modules and overlays.
- Code must work on both `x86_64-linux` and `aarch64-linux`.

## Never guess — ask
- Atlantean vocabulary, edition names, Plymouth text or any wording taken from *Aria Sum* that is not already in `docs/`.
- Visual identity beyond `docs/brief.md` §9 (palette, fonts, imagery).
- Licence of the project and of any asset.
- Anything about real hardware (disks, TPM, firmware) and GitHub settings or secrets.
- Bumping `flake.lock` or the NixOS version.

## Forbidden
- Disabling, skipping or snapshot-updating a failing test to get green.
- Secrets, tokens or personal data in code, logs, fixtures or screenshots.
- Changing the build machine itself: `nixos-rebuild switch|boot|test`, disk, boot or TPM tools, `systemctl` on system units, `sudo`/`run0`. All testing happens in VMs.
- Updating `flake.lock` without a task that says so.
- "Tamper-proof", percentages for health, or alarmist wording in user-facing text.

## Mistake Log
Project-wide rules distilled from real mistakes here. Obey them. Changed only by
`/aih:distill` after human approval; new lessons go to `.claude/lessons/inbox.md`
via `/aih:lesson`. Max 15; path-specific rules live in `.claude/rules/project-*.md`.
Line format: `- WHEN <trigger> → <rule>.` followed by `<!-- id=P-<n> n=<count> since=YYYY-MM check=<…> -->`.

<!-- distilled rules below -->
- WHEN a local VM test dies with a kernel crash ("synchronous external abort", `dc zva` in `clear_page`) → it is lantea-bench, not the module: rerun it once at most, then let CI decide (decision 0019). A failed assertion is a real failure and is debugged as usual. <!-- id=P-1 n=3 since=2026-10 check=none retire-with=0019 -->
