# 0013 — Development happens on lantea-bench, as an unprivileged user

**Status:** Accepted · **Date:** 2026-10-09

## Context

Principle 7 forbids acting on the host. A coding agent with shell access needs somewhere to run where the worst it can damage is disposable, and NixOS VM tests need Linux with KVM.

## Decision

- The build machine is `lantea-bench`: NixOS 26.05 (aarch64) in UTM on an M4 Mac mini, with nested virtualisation (`/dev/kvm`).
- Two accounts: `primoz` administers the build machine; `dev` holds the repository and runs the coding agent, and has no admin rights.
- The agent may build, run `nix flake check` and start VMs. It never changes the build machine itself: no `nixos-rebuild switch|boot|test`, no disk, boot or TPM tools, no `sudo`/`run0`. The harness guard denies these.
- The harness gates call their tools through `nix fmt` and `nix develop -c …`, so they use the flake's pinned tools however the agent was started. Once Phase 0 provides the dev shell, start the agent inside it (`nix develop -c claude`) so its own commands find the same tools.

## Consequences

- Builds of the build machine's own configuration are done by a human, as `primoz`.
- If the build machine breaks, it can be reinstalled from scratch in under an hour; nothing of value lives only there (the repo is on GitHub).
