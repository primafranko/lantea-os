#!/usr/bin/env bash
# Evaluate the flake, then build every check for this system without booting a
# VM: a NixOS VM test is built as its driver (its nodes' systems and the linted
# test script), any other check is built as is. Decision 0019: VM tests are
# booted in CI, or on demand with `aih run check.vm`.
set -euo pipefail

system=$(nix eval --impure --raw --expr builtins.currentSystem)

nix flake check --no-build

targets=$(nix eval --raw ".#checks.${system}" --apply "checks:
  builtins.concatStringsSep \" \" (map
    (name: \".#checks.${system}.\" + name + (if checks.\${name} ? driver then \".driver\" else \"\"))
    (builtins.attrNames checks))")

if [ -n "$targets" ]; then
  # $targets is unquoted on purpose: one installable per word.
  nix build --no-link -L $targets
fi
