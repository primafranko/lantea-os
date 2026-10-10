# 0019 — VM tests boot in CI until lantea-bench is stable

**Status:** Accepted (temporary) · **Date:** 2026-10-10 · **Supersedes in part:** [0012](0012-architectures.md), "aarch64 is built and tested locally on the build machine", until this record is retired · Amended 2026-10-10 (CI user namespaces)

## Context

On 2026-10-09, aarch64 test VMs on `lantea-bench` crashed while booting in most runs: the guest kernel logged "Internal error: synchronous external abort" with a `MACHINE_CHECK` taint, or froze within the first minute. A minimal `runNixOSTest` with no Lantea modules crashed the same way, so the fault is in the build machine's nested virtualisation (UTM on an M4 Mac mini, decision 0013), not in Lantea. Five of six boots failed in a row, so a local `aih verify` could not give a verdict, while the same tests passed in CI on x86_64.

## Decision

- CI (x86_64, decision 0012) is the authority for booting VM tests. A change is not done until its VM tests pass in CI.
- Locally, the `check` gate of the verify phase (`tests/check-without-boot.sh`) evaluates the flake and builds every check without booting it: a NixOS VM test is built as its driver, which builds every node's system and lints the test script.
- `check.vm` (`nix flake check -L`) is a named gate in no phase. It boots the VM tests on the build machine on demand: `aih run check.vm`.
- This record is temporary. When `lantea-bench` boots aarch64 test VMs reliably again, a new record retires it and 0012's local aarch64 testing applies again.

## Consequences

- aarch64 VM tests are not booted anywhere while this holds; aarch64 is only built and evaluated. A fault that shows only when an aarch64 VM boots can go unnoticed until this record is retired.
- A local `VERIFY: PASS` no longer means the VM tests passed. Task specs name CI as the place where they must pass.
- `nix flake check -L` still works locally and can be run by hand when the build machine behaves.
- The `lantea` package's tests make mounts inside unprivileged user namespaces (`unshare -Urm`), and CI runs them while building the VM tests. Ubuntu runners restrict such namespaces with AppArmor, so `ci.yml` sets `kernel.apparmor_restrict_unprivileged_userns=0` before the build when that setting exists. If a namespace is still denied, those tests fail with a message saying so; they never skip.
