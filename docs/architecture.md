# Architecture

Lantea is a flake of NixOS modules, packages and themes on NixOS 26.05. It has two layers, and the line between them is the most important rule in the codebase.

## Two layers

```
┌──────────────────────────────────────────────────────────────┐
│ Experience (optional, replaceable)                           │
│ Plasma config · theme · sounds · Plymouth · Condition widget │
│ reads Core's outputs; never the other way round              │
├──────────────────────────────────────────────────────────────┤
│ Core (required, headless)                                    │
│ discipline · tended-store · watch · condition · record ·     │
│ security · inheritance · offerings · backups · impermanence  │
├──────────────────────────────────────────────────────────────┤
│ NixOS 26.05 · systemd · Btrfs · LUKS · journald · nftables   │
└──────────────────────────────────────────────────────────────┘
```

- **Core never imports Experience.** A headless machine runs all of Core.
- **Experience reads Core's public outputs only:** `/run/lantea/condition.json`, the `lantea` CLI with `--json`, and the Record. It never reaches into a Core module's internals.

## How a machine uses Lantea

Each machine has its own flake. It imports Lantea as an input and enables what it wants ([ADR 0003](decisions/0003-distribution-model.md)):

```nix
# the machine's own flake.nix (sketch)
inputs.lantea.url = "github:primafranko/lantea-os";
# ...
modules = [ lantea.nixosModules.default ./hardware.nix ./my-settings.nix ];
```

Lantea sets its single-value defaults with `lib.mkDefault`; what it adds to lists and attribute sets sits behind `lantea.*.enable` options. The user's own settings win without `mkForce`, and `lantea inherit` can tell which value came from where.

## Flake outputs

| Output | What |
|---|---|
| `nixosModules.core` | All Core Strands |
| `nixosModules.experience` | The Experience layer |
| `nixosModules.default` | Core + Experience |
| `nixosConfigurations.vm-dev` | Development VM, x86_64-linux |
| `nixosConfigurations.vm-dev-aarch64` | Development VM, aarch64-linux |
| `packages.<system>.lantea` | The `lantea` CLI |
| `checks.<system>.*` | One VM test per Strand, plus formatting and lint checks |
| `formatter.<system>` | nixfmt |
| `devShells.<system>.default` | Rust toolchain, nixfmt, statix, deadnix, nvd |

## Core modules

| Module | Responsibility | Phase |
|---|---|---|
| `discipline` | Base policy, the `lantea.*` namespace, os-release, Steward/Keeper accounts | 0 |
| `record` | Record writer (Phase 1), sealing, retention and auditd (Phase 4) | 1, 4 |
| `tended-store` | Btrfs layout, Held (XDG Trash), set-down/restore/release | 1 |
| `watch` | Purpose, ceilings and sandbox per governed service | 2 |
| `condition` | Condition daemon and rules | 3 |
| `offerings` | Fetch, build, diff, offer, accept, return | 5 |
| `security` | Firewall, run0 + sudo shim, hardening, Secure Boot, TPM2 | 6 |
| `inheritance` | Provenance of configuration | 6 |
| `backups` | btrbk snapshots (Phase 1); restic off-site (not yet scheduled) | 1 |
| `impermanence` | "Leave no mark", off by default | later |

## The `lantea` CLI

A Rust workspace in `pkgs/lantea` (clap, serde). Conventions shared by every command:

- plain-language output by default; `--json` for machines; `--explain` prints the real commands instead of (or before) running them;
- exit codes: `0` success, `1` the action failed or was refused, `2` usage error;
- destructive actions ask first unless `--yes`.

## Where things are decided

- The vocabulary: [`glossary.md`](glossary.md).
- Decisions and their reasons: [`decisions/`](decisions/).
- Work in progress: [`tasks/`](tasks/).
