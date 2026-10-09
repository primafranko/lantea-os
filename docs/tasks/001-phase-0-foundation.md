# Task 001 — phase-0-foundation

| Field | Value |
|---|---|
| Area / owner | foundation (whole repository) · Primož |
| Risk | low |
| Depends on | — |
| Contract | `docs/brief.md` §3, §5, §6 (Phase 0), §7 · decision records 0003, 0004, 0005, 0012, 0013 |
| Model hint | opus (Nix module and test wiring is subtle) |

## Goal
A Lantea flake that builds a bootable development VM on x86_64 and aarch64, with the `discipline` module, the Steward and Keeper accounts, a `lantea` CLI skeleton and its first VM test — proven on the build machine (aarch64) and in CI (x86_64).

## Context
This is the first code in the repository. Everything before it is documents: `docs/brief.md` (v1.1), `docs/glossary.md`, `docs/architecture.md` and the decision records. The build machine is `lantea-bench` (NixOS 26.05, aarch64, `/dev/kvm` available; you run as the unprivileged user `dev`). CI is GitHub Actions on x86_64.

## Assumptions
- `nixpkgs` is the only flake input, pinned to `github:NixOS/nixpkgs/nixos-26.05`. No flake-utils or flake-parts: a small `forAllSystems = lib.genAttrs [ "x86_64-linux" "aarch64-linux" ]`.
- The formatter is nixpkgs' official Nix formatter (`pkgs.nixfmt`, RFC style). If the attribute is named differently in 26.05, use the official one and say so in Implementer notes.
- The Rust toolchain comes from nixpkgs (no rust-overlay); edition 2024; the package is built with `rustPlatform.buildRustPackage` and `cargoLock.lockFile`.
- The Phase 0 VM is headless: Core only, no Experience.
- The real `sudo` is switched off now (`security.sudo.enable = lib.mkDefault false`, per decision 0008); the explaining shim arrives in Phase 6. polkit is on so the Keeper can use run0 interactively.
- In `vm-dev` only, the Steward is logged in automatically on the console, and the Keeper has the initial password `keeper`. Both are development conveniences, marked as such in comments, and never part of `nixosModules`.

## Files in scope
- `flake.nix` (new), `flake.lock` (new) — outputs per `docs/architecture.md` "Flake outputs"
- `hosts/vm-dev/default.nix` (new) — one host module used by both `vm-dev` and `vm-dev-aarch64`
- `modules/core/default.nix` (new) — imports the Core Strands that have code (only `discipline` in Phase 0)
- `modules/core/discipline/**` (new) — options, os-release, accounts, `lantea` package installation
- `modules/core/*/README.md` (new) — one per not-yet-built Strand
- `modules/experience/default.nix` (new), `modules/experience/*/README.md` (new)
- `pkgs/lantea/**` (new) — Cargo workspace with the `lantea` CLI crate; `pkgs/lantea/default.nix` for the Nix package
- `tests/default.nix` (new), `tests/discipline.nix` (new)
- `.github/workflows/ci.yml` (new)
- `.claude/harness.json` — add the `rust` package and phases given under Test data
- `README.md` — update the Status section
- `.gitignore`
- `docs/decisions/*.md` (new) — a record for any choice this spec doesn't settle

## Acceptance criteria
- [ ] `nix flake show --all-systems` lists `nixosModules.core`, `nixosModules.experience`, `nixosModules.default`; `nixosConfigurations.vm-dev` and `nixosConfigurations.vm-dev-aarch64`; for both `x86_64-linux` and `aarch64-linux`: `packages.lantea`, `packages.default`, `checks.discipline`, `formatter`, `devShells.default`.
- [ ] `vm-dev` is `x86_64-linux` and `vm-dev-aarch64` is `aarch64-linux`; both use `hosts/vm-dev` and `nixosModules.default`, and set `lantea.enable = true`.
- [ ] On the build machine, `nixos-rebuild build-vm --flake .#vm-dev-aarch64` succeeds, and `QEMU_OPTS=-nographic ./result/bin/run-vm-dev-vm` boots to a shell logged in as the Steward.
- [ ] Options exist, each with a `description`: `lantea.enable` (bool, default `false`); `lantea.track` (enum `"standing"`/`"rising"`, default `"standing"`); `lantea.edition` (read-only, `{ version = "1.0"; name = "Harbour"; }`); `lantea.accounts.steward.name` (default `"steward"`); `lantea.accounts.keeper.name` (default `"keeper"`). Single-value settings Lantea makes in `config` use `lib.mkDefault`; what it adds to lists and attribute sets (packages, users) merges normally behind `lantea.enable`.
- [ ] With `lantea.enable = true`, `/etc/os-release` contains `ID=lantea`, `ID_LIKE=nixos` and `NAME="Lantea OS"`, `PRETTY_NAME` contains `Lantea OS`, and `nixos-version` still works. If NixOS's options cannot express this exactly, record what was done in a decision record.
- [ ] The Steward exists and is not in `wheel`; the Keeper exists and is in `wheel`. `sudo` is not installed (`command -v sudo` fails). polkit's admin identity is `unix-group:wheel`. As the Steward, `run0 --no-ask-password true` fails with an authorization error (not "command not found").
- [ ] With `lantea.enable = true`, `lantea` is on every user's PATH.
- [ ] `lantea --version` prints exactly `lantea 0.1.0 (Harbour)`.
- [ ] Phase 0 has one subcommand, `condition`. A bare `lantea` prints usage to stderr and exits 2. `lantea condition` prints exactly `Condition is not yet known.` followed by a newline and exits 0; `lantea condition --json` prints `{"state":"unknown"}` and exits 0; `lantea condition --explain` prints exactly `No commands are run yet: the condition daemon arrives in Phase 3.` and exits 0.
- [ ] `--json`, `--explain` and `--yes` are global flags (accepted before or after the subcommand); an unknown subcommand exits 2. All exact strings below end with one newline.
- [ ] `tests/discipline.nix` uses `pkgs.testers.runNixOSTest`, boots a node with `nixosModules.core` (Core works headless) and `lantea.enable = true`, and asserts every criterion above that can be checked inside a VM (os-release values, accounts and groups, the Steward's refused elevation, the exact CLI output and exit codes). It is wired as `checks.<system>.discipline` for both systems.
- [ ] `devShells.default` provides `cargo`, `rustc`, `clippy`, `rustfmt`, `rust-analyzer`, `nixfmt`, `statix`, `deadnix`, `nvd` and `jq`.
- [ ] `.github/workflows/ci.yml` runs on push and pull request on `ubuntu-latest`: installs Nix, makes `/dev/kvm` usable, runs `nix flake check -L` and `nix build .#nixosConfigurations.vm-dev.config.system.build.vm -L`. It is green on this task's pull request.
- [ ] `.claude/harness.json` contains the `rust` package and phases below, and `aih doctor` reports no FAIL lines.
- [ ] Each Strand directory without code has a `README.md` with one line: what it will hold and which phase brings it.
- [ ] `aih verify` prints `VERIFY: PASS`.

## Test data
- Edition: version `1.0`, name `Harbour`. CLI version `0.1.0`.
- Exact strings: `lantea 0.1.0 (Harbour)` · `Condition is not yet known.` · `{"state":"unknown"}` · `No commands are run yet: the condition daemon arrives in Phase 3.`
- `vm-dev` VM: 2 GiB memory, 2 cores, serial console, host name `vm-dev` (both architectures).
- Add to `.claude/harness.json` `packages`:

```json
{
  "name": "rust",
  "root": "pkgs/lantea",
  "files": ["pkgs/lantea/**/*.rs", "pkgs/lantea/**/Cargo.toml", "pkgs/lantea/Cargo.lock"],
  "commands": {
    "format":       { "run": "nix develop -c cargo fmt --all", "scope": "package", "timeout": 120 },
    "format.check": { "run": "nix develop -c cargo fmt --all -- --check", "scope": "package", "timeout": 180 },
    "lint":         { "run": "nix develop -c cargo clippy --workspace --all-targets --locked -- -D warnings", "scope": "package", "timeout": 900 },
    "test":         { "run": "nix develop -c cargo test --workspace --locked", "scope": "package", "timeout": 900 }
  }
}
```

  and set `phases` to: `edit: ["format"]`, `stop: ["lint", "check.stop", "test"]`, `verify: ["format.check", "lint", "test", "check"]`.

## Non-functional
- Evaluates and builds on `x86_64-linux` and `aarch64-linux`; nothing assumes an architecture.
- No flake inputs besides `nixpkgs`.
- User-facing strings follow "Speak plainly" (`.claude/rules/project-lantea.md` L-1).

## Verification
Success check — ONE command that must exit 0 (on the build machine). It is necessary, not sufficient: CI, `aih verify` and the manual check below must also pass.
```
nix flake check -L && nixos-rebuild build-vm --flake .#vm-dev-aarch64
```

Manual check — Primož boots the VM once (`QEMU_OPTS=-nographic ./result/bin/run-vm-dev-vm`), sees the Steward's shell, runs `lantea condition` and `cat /etc/os-release`, and checks CI is green on the pull request.

## Known pitfalls
- Flakes only see files tracked by git: `git add` new files before `nix flake check` or `nix build`, or they are silently missing.
- `nix flake check` evaluates both `nixosConfigurations`; keep host modules cheap to evaluate.
- The guard denies `nixos-rebuild switch|boot|test`, disk and boot tools and `sudo`/`run0` on the build machine (decision 0013). `nixos-rebuild build-vm` and `nix build` are allowed; root-only behaviour is tested inside the VM test.
- `buildRustPackage` needs `Cargo.lock` committed; `cargo … --locked` in the gates fails without it.
- U-2: never weaken, skip or delete a test to get green.

## Docs & changelog
- `README.md` Status: "Phase 0 — Foundation: done (build machine: aarch64; CI: x86_64)."
- No other docs change unless a decision record is added.

## Out of scope
- Disk layout, Btrfs, disko, the `workstation` host (Phase 1).
- The Record writer (Phase 1).
- The `sudo` shim, run0 policy and hardening (Phase 6).
- Anything in `modules/experience/` beyond `default.nix` and READMEs (Phase 7).
- ISO and installer (Phase 8).
- A static (musl) build of `lantea` — the brief asks for a single static binary; a later task.
- A LICENSE file — ask Primož.
- aarch64 runners in CI.
- A positive run0 test for the Keeper (the Keeper can elevate, so polkit and run0 do not refuse everyone) — follow-up for Phase 6, from the review of this task.

## Implementer notes
- 2026-10-09 (implementer): **Formatter.** `pkgs.nixfmt` (nixfmt 1.5.0, RFC style) is the official attribute in 26.05; no rename was needed.
- **os-release.** NixOS 26.05 expresses it exactly. `system.nixos.distroId = mkDefault "lantea"` and `distroName = mkDefault "Lantea OS"` give `ID=lantea` and `NAME="Lantea OS"`. NixOS adds `ID_LIKE=nixos` itself whenever the ID is not `nixos`. After Primož's review, the version keys name the edition (`PRETTY_NAME="Lantea OS 1.0 (Harbour)"`, `VERSION_ID="1.0"`, `VERSION_CODENAME=harbour`); `BUILD_ID` stays NixOS's and `LANTEA_BASE="NixOS 26.05 (Yarara)"` is added — decision 0016. `CPE_NAME` keeps NixOS's form, `cpe:/o:nixos:nixos:26.05`, for vulnerability scanners. Side effect: `DEFAULT_HOSTNAME=lantea`.
- **polkit.** `security.polkit.adminIdentities = mkDefault [ "unix-group:wheel" ]` is set explicitly, although it equals the nixpkgs default, so the provenance is Lantea's.
- **The `lantea` package in the module.** The discipline Strand builds `lantea` with `pkgs.callPackage ../../../pkgs/lantea { }`. There is no `lantea.package` option and no overlay. Decision 0015 (Proposed) records this choice.
- **CLI flags.** `--explain` wins over `--json` when both are given. `--yes` is accepted and ignored because nothing needs confirming yet. Decision 0015 records this too. A bare `lantea` uses clap's `arg_required_else_help`: help (including `Usage:`) goes to stderr, with exit code 2.
- **vm-dev.** The host imports `qemu-vm.nix` directly. The host is then itself a VM configuration: `fileSystems` and the boot loader come from qemu-vm, so `toplevel` evaluates and `build-vm` works. `graphics = false` gives the serial console. The Steward autologin and the Keeper's `initialPassword = "keeper"` exist only in `hosts/vm-dev`.
- **`.gitignore`.** Unchanged: `result`, `*.qcow2` and `target/` were already covered.
- **No LICENSE.** `pkgs/lantea` has no `meta.license`, because the licence is not decided.
- **Intermittent guest crashes on the build machine.** Three early aarch64 boots died soon after start: two runs of `checks.aarch64-linux.discipline`, and one of the dev VM. A minimal control `runNixOSTest` with no Lantea modules died the same way. In each case the guest kernel logged "Internal error: synchronous external abort: 0000000096000010", Oopsed, and was tainted `[M]=MACHINE_CHECK`. Later runs were clean: `aih verify` (all subtests passed), the success check, and a dev-VM boot that reached the Steward's shell. The cause looks like nested KVM on the host (UTM on Apple M4), not Lantea. Expect occasional hung or crashed VM tests on `lantea-bench`; rerun before debugging the module.
