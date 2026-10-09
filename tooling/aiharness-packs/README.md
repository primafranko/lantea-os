# Packs for AiHarness v2

Two generic stack packs, first used by Lantea. They belong in the AiHarness repository at `plugin/packs/nix/` and `plugin/packs/rust/`; `BOOTSTRAP.md` step 3 moves them there and removes this folder.

| Pack | Detects | Gates |
|---|---|---|
| `nix` | `flake.nix` | edit: `nix fmt` · stop: statix + deadnix, `nix flake check --no-build` · verify: format check, lint, `nix flake check -L` (with VM tests) |
| `rust` | `Cargo.toml` | edit: `cargo fmt` · stop: clippy `-D warnings`, `cargo test` · verify: fmt check, clippy, test |

Guard additions are `ask`, not `deny`: activating a configuration, updating `flake.lock`, garbage-collecting the store, `cargo update`, publishing crates. Projects that need stricter rules (Lantea does) add `deny` entries in their own `.claude/harness.json`.

Not yet covered by the harness's slow tests (`AIH_SLOW=1 node --test test/packs.slow.test.mjs`); adding a Nix and a Rust case there is a good follow-up.
