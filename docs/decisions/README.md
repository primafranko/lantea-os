# Decision records

One file per decision; see [0001](0001-record-architecture-decisions.md) for the format.

| # | Decision | Status |
|---|---|---|
| [0001](0001-record-architecture-decisions.md) | Record architecture decisions | Accepted |
| [0002](0002-base-nixos-26-05.md) | Base: NixOS 26.05, then 26.11 | Accepted |
| [0003](0003-distribution-model.md) | Lantea is a flake that each machine's own flake imports | Accepted |
| [0004](0004-glossary-on-watch-and-editions.md) | Glossary: services are "on watch"; versions are "editions" | Accepted |
| [0005](0005-phase-order.md) | A minimal Record and the accounts arrive early | Accepted |
| [0006](0006-tended-store-and-held.md) | Tended Store at /home; Held is XDG Trash inside it | Accepted |
| [0007](0007-offerings.md) | An Offering is one generation; accepting it updates the declared lock | Accepted |
| [0008](0008-elevation-run0-and-sudo-shim.md) | Elevation via run0, with a sudo shim that explains itself | Accepted |
| [0009](0009-hardening-and-ceilings.md) | systemd sandboxing first; MemoryHigh as the ceiling | Accepted |
| [0010](0010-record-tamper-evident.md) | The Record is tamper-evident, not tamper-proof | Accepted |
| [0011](0011-tpm2-binding.md) | TPM2 unlock binds to Secure Boot state plus PIN | Proposed |
| [0012](0012-architectures.md) | x86_64 and aarch64 from Phase 0 | Accepted |
| [0013](0013-build-machine.md) | Development happens on lantea-bench, as an unprivileged user | Accepted |
| [0014](0014-impermanence-ready-layout.md) | Take a blank @root snapshot at install | Proposed |
