---
paths:
  - "**/*.nix"
  - "flake.lock"
---
# Nix

<!-- Managed by AiHarness (aih sync). Change it in the harness repo. -->

- WHEN unsure what a NixOS option or nixpkgs attribute does or is called → read it for the pinned revision (`nix eval`, `nix search`, the module source in the store); never from memory — names change between releases. <!-- id=NX-1 n=0 since=2026-10 check=none -->
- WHEN adding a flake input → pin it through `flake.lock`, make it follow the project's nixpkgs (`inputs.x.inputs.nixpkgs.follows = "nixpkgs"`) unless there is a stated reason; never fetch without a hash or rev (`builtins.fetchTarball` without `sha256`, `<nixpkgs>`, `builtins.getEnv`). <!-- id=NX-2 n=0 since=2026-10 check=none -->
- WHEN writing a NixOS module → declare options with `lib.mkOption { type; default; description; }` (or `lib.mkEnableOption`), guard config with `lib.mkIf cfg.enable`, qualify with `lib.` instead of a file-wide `with lib;`. <!-- id=NX-3 n=0 since=2026-10 check=hook:stop-guard -->
- WHEN setting a value that a user or another module may override → `lib.mkDefault`; `lib.mkForce` only with a comment saying why nothing else works. <!-- id=NX-4 n=0 since=2026-10 check=none -->
- WHEN a derivation takes local sources → pass only what it needs (`lib.fileset`, `lib.cleanSource`), never the whole repo, so unrelated edits don't trigger rebuilds. <!-- id=NX-5 n=0 since=2026-10 check=none -->
- WHEN writing a NixOS VM test → `pkgs.testers.runNixOSTest`; wait on state (`wait_for_unit`, `wait_for_open_port`, `wait_until_succeeds`), never `sleep`; assert exact output with `succeed`/`fail`. <!-- id=NX-6 n=0 since=2026-10 check=none -->
- WHEN a new file is not seen by `nix build` or `nix flake check` → `git add` it; flakes only see tracked files. <!-- id=NX-7 n=0 since=2026-10 check=none -->
- WHEN tempted to activate a configuration on the machine you are working on → don't; build it (`nixos-rebuild build`, `build-vm`) or test it in a VM. <!-- id=NX-8 n=0 since=2026-10 check=hook:guard -->
