# Lantea OS

An opinionated layer on NixOS whose architecture embodies the ethic of the Atlanteans from the novel *Aria Sum*: the system is fully declared, nothing is destroyed carelessly, services stand on watch within declared limits, the machine reports its condition in plain language, every deliberate action leaves an honest record, and nothing is forced on the user.

Lantea is not a fork and not a theme. Every Lantea term maps to a real Linux or Nix concept that stays visible underneath, and every `lantea` command can `--explain` the real commands it runs.

## Status

Phase 0 — Foundation: done (build machine: aarch64; CI: x86_64). Edition 1.0 *Harbour* is in development.

## Read first

- [The brief](docs/brief.md) — what Lantea is and the plan, phase by phase
- [Glossary](docs/glossary.md) — the fixed vocabulary
- [Philosophy](docs/philosophy.md) and [Architecture](docs/architecture.md)
- [Decision records](docs/decisions/)

## Building

Development and testing happen in virtual machines only. On a NixOS machine with flakes and KVM:

```
nix flake check -L                                   # all checks, including VM tests
nixos-rebuild build-vm --flake .#vm-dev-aarch64      # or .#vm-dev on x86_64
QEMU_OPTS=-nographic ./result/bin/run-vm-dev-vm
```

## Licence

Not yet decided.
