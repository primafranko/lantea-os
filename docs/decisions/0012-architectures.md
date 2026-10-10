# 0012 — x86_64 and aarch64 from Phase 0

**Status:** Accepted · **Date:** 2026-10-09 · Local aarch64 testing is suspended by [0019](0019-vm-tests-boot-in-ci.md) (temporary)

## Context

The build machine is an Apple Silicon Mac mini running a NixOS VM, which can run aarch64 test VMs fast but x86_64 only by emulation. GitHub's standard runners are x86_64 with KVM.

## Decision

- The flake targets `x86_64-linux` and `aarch64-linux` from the start.
- aarch64 is built and tested locally on the build machine; x86_64 is built and tested in CI.
- Development hosts: `vm-dev` (x86_64-linux) and `vm-dev-aarch64` (aarch64-linux), from one shared host module.

## Consequences

- Core modules and the Rust CLI must not assume an architecture.
- Secure Boot and TPM2 work (Phase 6) is mainly x86_64 and is tested in CI with emulated firmware and TPM; real x86_64 hardware is needed before an edition ships.
