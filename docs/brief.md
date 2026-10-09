# LANTEA OS — Build Brief

**Version 1.1 · 9 October 2026**

You are the lead engineer on **Lantea OS**: an opinionated NixOS-based operating system whose architecture embodies the ethic of the Atlanteans from the novel *Aria Sum*. Read this entire brief before writing any code. Work phase by phase, stop for review at the end of each phase, and record significant decisions in `docs/decisions/`.

### Changes since 1.0

| Change | Decision record |
|---|---|
| Base is NixOS 26.05; move to 26.11 when it comes out | [0002](decisions/0002-base-nixos-26-05.md) |
| Lantea is a flake that each machine's own flake imports | [0003](decisions/0003-distribution-model.md) |
| Glossary: governed services are **on watch** (or *failing*); versions are **editions**; condition states and `tend` added | [0004](decisions/0004-glossary-on-watch-and-editions.md) |
| A minimal Record and the Steward/Keeper accounts arrive in Phases 0–1, not 4 and 6 | [0005](decisions/0005-phase-order.md) |
| `@tended` is mounted at `/home`; Held is XDG-Trash-compatible inside it; no `@held`, no `/tended` | [0006](decisions/0006-tended-store-and-held.md) |
| An Offering is a whole generation; accepting it writes back `flake.lock` and takes effect at next restart by default | [0007](decisions/0007-offerings.md) |
| Elevation via run0, with a `sudo` shim that says what it does | [0008](decisions/0008-elevation-run0-and-sudo-shim.md) |
| systemd sandboxing before AppArmor; `MemoryHigh` is the ceiling, `MemoryMax` the backstop | [0009](decisions/0009-hardening-and-ceilings.md) |
| The Record is tamper-evident, not tamper-proof | [0010](decisions/0010-record-tamper-evident.md) |
| TPM2 unlock binds to Secure Boot state + PIN (proposed) | [0011](decisions/0011-tpm2-binding.md) |
| aarch64-linux and x86_64-linux from Phase 0 | [0012](decisions/0012-architectures.md) |
| Development happens on `lantea-bench`, a NixOS VM; the agent runs unprivileged | [0013](decisions/0013-build-machine.md) |
| A blank `@root` snapshot is taken at install so "Leave no mark" can be enabled later (proposed) | [0014](decisions/0014-impermanence-ready-layout.md) |
| The target `lantea condition` output now agrees with the state rules (a storage trend under 30 days is Watchful, not Steady) | — |

---

## 1. What Lantea is

Lantea is **not** a themed Linux distribution. Its philosophy — the **Discipline** — is expressed in architecture, not decoration:

- the system is fully declared and reproducible;
- nothing is destroyed carelessly — things are *set down* and can be recovered;
- services stand *on watch* with a stated purpose and a declared resource ceiling;
- the machine reports its *condition* in plain language, not percentages;
- every deliberate action leaves an honest *record*;
- nothing is forced on the user — updates are *offered*, never imposed.

### Non-negotiable principles

1. **Interpretation layer, not deception layer.** Every Atlantean term maps to a real Linux/Nix concept. Underlying tools (Nix generations, systemd, Btrfs, journald, nftables, LUKS, TPM2) remain visible and usable. An experienced Linux administrator must be able to work in Lantea without learning anything new. Every `lantea` command supports `--explain`, which prints the real commands it runs.
2. **A layer on NixOS, not a fork.** Lantea is a flake of NixOS modules, packages and themes on top of current NixOS stable. Never patch nixpkgs unless unavoidable; prefer overlays and modules.
3. **Core is separate from Experience.** The philosophy lives in Core and must work headless, with no desktop. The Experience layer (KDE, theme, sounds, splash) only presents it and must be replaceable.
4. **Speak plainly.** All user-facing text is clear, calm, specific. No error codes without explanation. No alarmism. No marketing voice.
5. **Nothing automatic that the user has not authorised once.** No forced updates, no surprise reboots, no silent deletion.
6. **Reproducible and tested.** Every Core module ships with a NixOS VM integration test.
7. **Never act on the host machine.** All development and testing happens in VMs (`nixos-rebuild build-vm`, `nixosTest`). Never run destructive or system-changing commands outside a VM — and that includes the build machine `lantea-bench` itself.

---

## 2. Glossary (fixed — do not drift)

Each term has exactly one meaning. If a new concept is needed, add it here first. The same table lives in [`glossary.md`](glossary.md); keep them identical.

| Lantea term | Meaning | Underlying concept |
|---|---|---|
| **Discipline** | The whole declared system | The flake + NixOS configuration |
| **Strand** | One policy module | A NixOS module under `modules/` |
| **Edition** | A numbered, named version of Lantea | A git tag of the Lantea flake |
| **Standing** | Stable track | NixOS stable channel |
| **Rising** | Testing track | nixos-unstable |
| **Tended Store** | Live user data | Btrfs subvolume `@tended`, mounted at `/home` |
| **Held** | Set-down, recoverable data | Each user's XDG Trash inside the Tended Store (`~/.local/share/Trash`), with original path and time recorded |
| **set-down** | Delete without destroying | Move into Held (a rename on the same subvolume); logged |
| **restore** | Return from Held | Move back to the original path; never overwrites |
| **release** | Permanent removal | Delete from Held; always logged in the Record |
| **Released** | Permanently removed data | Actually deleted |
| **The Watch** | Services under governance | systemd units with a declared purpose and limits |
| **on watch** | A governed service running as declared | Active unit within its declared limits |
| **relieve** | Graceful stop | `systemctl stop` with full timeout, state preserved |
| **stand-down** | Immediate stop, reason required | `systemctl kill` + stop; reason written to the Record |
| **--force-the-hand** | Last-resort kill | SIGKILL; permanent Record entry |
| **Condition** | Health as understanding | Output of the condition daemon |
| **Steady · Watchful · Attention · Distressed** | The four condition states (§5) | Results of the condition rules |
| **tend** | Review what would like attention | `lantea tend`: ranked items from the condition daemon |
| **The Record** | Honest history of deliberate actions | Tamper-evident log: journald (sealed) + auditd |
| **Inherit / inheritance** | Provenance of configuration | Which settings are Lantea/upstream defaults vs. the user's own |
| **Offering** | An available update | A built-but-not-activated new generation, with the `flake.lock` that produced it |
| **accept** | Apply an offering | Activate the offered generation (at next restart by default) and write its `flake.lock` into the Discipline |
| **return** | Roll back | Activate a previous generation and restore its `flake.lock` |
| **Steward** | Daily-use account | Unprivileged user |
| **Keeper** | Administrative account | Separate privileged user, never a desktop login; elevation via run0 |
| **Leave no mark** | Optional impermanence profile | Root erased at boot; only declared state persists |

**Edition names:** 1.0 *Harbour*, 1.1 *Lantern*, 1.2 *Serenis*, 1.3 *Sum*. **Host names** are separate and never reuse edition names (e.g. `vm-dev`, `workstation`).

---

## 3. Architecture

### Lantea Core (headless, required)

| Module | Responsibility |
|---|---|
| `discipline` | Base system policy, defaults, the `lantea.*` option namespace, Steward/Keeper accounts |
| `tended-store` | Btrfs subvolume layout, Held (XDG Trash), set-down/restore/release backend |
| `watch` | Service governance: every Lantea-managed unit declares `purpose`, resource ceilings (`MemoryHigh`, `MemoryMax`, `CPUQuota`) and a sandbox |
| `condition` | The condition daemon and its rules |
| `record` | Append-only action log, journald retention and sealing, auditd rules |
| `security` | Firewall, elevation, encryption, hardening |
| `inheritance` | Provenance reporting: what is default vs. user-declared |
| `offerings` | Update flow: fetch, build, diff, offer, accept, return |
| `backups` | Local snapshots and off-site backup |
| `impermanence` | Optional "Leave no mark" profile — **off by default** |

### Lantea Experience (optional, replaceable)

KDE Plasma 6 configuration, colour scheme, fonts, icons, cursor, sounds, Plymouth splash, wallpapers, the Condition panel widget, installer branding. Proposals so far: [`notes/experience-proposals.md`](notes/experience-proposals.md).

---

## 4. Technical decisions

- **Base:** NixOS 26.05 (stable), flakes enabled; move to 26.11 when it comes out.
- **Architectures:** x86_64-linux and aarch64-linux. aarch64 is tested on the build machine, x86_64 in CI.
- **Distribution:** each machine has its own flake that imports Lantea as an input; Lantea sets its single-value defaults with `lib.mkDefault` so the user's own settings win without `mkForce` and provenance stays visible; what it adds to lists and attribute sets sits behind `lantea.*.enable` options.
- **Disk layout:** declared with **disko**. LUKS2 full-disk encryption; Btrfs inside with subvolumes `@root`, `@nix`, `@tended`, `@record`, `@snapshots`. Proposed: a blank snapshot `@root-blank` taken at install (decision 0014).
- **Boot:** **systemd-boot** with Secure Boot via **lanzaboote**. TPM2 + PIN unlock, with FIDO2 key as an alternative. Proposed: bind to Secure Boot state, PCR 7 (decision 0011).
- **Snapshots and backup:** **btrbk** for scheduled local snapshots (hourly/daily/weekly retention) and send/receive to an external disk; **restic** for encrypted off-site backup.
- **Firewall:** nftables, default deny inbound; every open port declared in config.
- **Elevation:** **run0**; `sudo` is a shim that prints what it is doing and calls run0. Optional FIDO2 via pam_u2f.
- **Hardening:** systemd service sandboxing for every unit on watch (exposure shown by `lantea watch`), auditd, minimal running services. AppArmor later, where maintained profiles exist.
- **Update diffs:** **nvd** to show package-level differences between generations.
- **Desktop (Experience):** KDE Plasma 6 on Wayland.
- **CLI language:** **Rust** (clap for arguments, serde for JSON). Single static binary.
- **Condition daemon:** Rust, systemd service + timer; writes state to `/run/lantea/condition.json`; optional local-LLM narration via Ollama, **disabled by default**.
- **CI:** GitHub Actions — `nix flake check` (including all VM tests) on x86_64, ISO build.

---

## 5. The `lantea` CLI

Every command: plain-language output by default, `--json` for machine output, `--explain` to print underlying commands, honest non-zero exit codes. Destructive actions ask for confirmation unless `--yes`.

| Command | Behaviour |
|---|---|
| `lantea condition` | Current condition summary (see format below) |
| `lantea tend` | Everything that would like attention, ranked, each with what it is, why it matters, what happens if ignored |
| `lantea set-down <path>` | Move to Held with timestamp, original path and size recorded |
| `lantea held` | List held items |
| `lantea restore <id>` | Return to original path (refuse to overwrite; offer an alternative path) |
| `lantea release <id>` | Permanently delete from Held; Record entry |
| `lantea watch` | List governed services: purpose, state, uptime, resource use vs. ceiling, sandbox exposure |
| `lantea relieve <unit>` | Graceful stop |
| `lantea stand-down <unit> --reason "<text>"` | Immediate stop; reason required |
| `lantea stand-down <unit> --force-the-hand --reason "<text>"` | SIGKILL; permanent Record entry |
| `lantea offerings` | Show available updates with nvd diff and plain summary |
| `lantea accept [--now]` | Activate the offered generation at next restart (or now, with `--now`) |
| `lantea return [generation]` | Roll back |
| `lantea inherit [option]` | Show provenance of configuration |
| `lantea record [--since]` | Read the Record |

### Target output for `lantea condition`

```
CONDITION · STEADY

THE TENDED STORE
  142 GiB tended · 18 GiB held · 6 snapshots retained

THE WATCH
  23 on watch · 0 failing

THE RECORD
  Last backup               01:14 today
  Last scrub                03:00, no errors
  Last configuration change 2 days ago

ATTENTION
  1 offering is available. No action is required.
```

When storage is trending toward full within 30 days, the state is **Watchful** and the Tended Store section says so in one sentence ("Storage is filling faster than usual. At this rate it will need attention in about 20 days.").

### Condition states

| State | Meaning |
|---|---|
| **Steady** | Nothing needs attention |
| **Watchful** | Something is trending toward needing attention (e.g. storage full in < 30 days, backup older than 2 days) |
| **Attention** | Action advisable soon (SMART warnings, failed unit, backup > 7 days, storage full in < 7 days) |
| **Distressed** | Action needed now (failing disk, scrub errors, encryption/boot integrity issue, Btrfs errors) |
| *Unknown* | The condition daemon has not reported yet (not a state the rules produce) |

Inputs: smartd, lm_sensors, Btrfs scrub and device stats, disk usage trend (sampled over days), systemd failed units, backup age, snapshot health, memory pressure (PSI), pending offerings, and whether the running generation matches the declared Discipline. Rules live in a readable config file, not hard-coded.

---

## 6. Milestones and acceptance criteria

Stop after each phase, summarise what was built, show test results, and wait for review.

**Phase 0 — Foundation**
Flake skeleton, repository structure (section 7), `vm-dev` hosts for x86_64 and aarch64 that boot in a VM, the `discipline` module with the `lantea.*` namespace and the Steward/Keeper accounts, a `lantea` CLI skeleton, CI running `nix flake check`. Task spec: [`tasks/001-phase-0-foundation.md`](tasks/001-phase-0-foundation.md).
*Done when:* `nixos-rebuild build-vm --flake .#vm-dev-aarch64` produces a bootable VM on the build machine, `nix flake check` passes there, and CI is green on x86_64.

**Phase 1 — The Tended Store**
Minimal Record writer (journald entries under the `lantea` identifier); disko layout (for VM and installable hosts), subvolumes, blank `@root` snapshot, btrbk snapshots, `set-down` / `held` / `restore` / `release` on top of XDG Trash.
*Done when:* a VM test sets down a file, restores it, releases another, a file moved to Trash with `gio trash` appears in `lantea held`, and the Record shows all of it.

**Phase 2 — The Watch**
`lantea.watch` option to declare purpose, ceilings and sandbox per service; `watch`, `relieve`, `stand-down`, `--force-the-hand`.
*Done when:* a VM test declares a service with limits, relieves it gracefully, stands down another with a reason, and the Record captures both; the Steward cannot stand down a service without the Keeper.

**Phase 3 — Condition**
Daemon, rules file, states, `condition` and `tend`.
*Done when:* VM tests simulate a filling disk, a failed unit and an old backup, and Condition reports the correct state and plain-language text for each.

**Phase 4 — The Record (hardening)**
Journald sealing (Forward Secure Sealing), retention, auditd rules, `lantea record`.
*Done when:* every Lantea action in earlier phases appears in the Record, the Steward cannot edit it, and a tampered journal is reported by `lantea record --verify`.

**Phase 5 — Offerings**
Fetch → build → nvd diff → offer → accept → return. Never automatic.
*Done when:* a VM test builds an offering, shows the diff, accepts it (next restart), the declared `flake.lock` matches the running generation, and return restores both.

**Phase 6 — Security**
Firewall default deny, run0 + `sudo` shim, systemd sandboxing defaults, auditd, lanzaboote + TPM2 on installable hosts, `lantea inherit`.
*Done when:* a VM test confirms no unexpected listening ports, the Steward cannot elevate without the Keeper, and inherit correctly distinguishes defaults from user settings.

**Phase 7 — Experience**
Plasma configuration, palette, fonts, sounds, Plymouth (including the disk-unlock PIN screen), wallpaper, Condition panel widget.
*Done when:* the VM boots to a themed desktop with a live Condition indicator.

**Phase 8 — Image and installer**
Live ISO and themed installer.
*Done when:* CI produces an ISO that installs to a blank VM and boots into Lantea.

**Later:** "Leave no mark" impermanence profile; optional Ollama narration for Condition; a light ("Limestone") theme.

---

## 7. Repository structure

```
lantea-os/
├── flake.nix
├── flake.lock
├── hosts/
│   ├── vm-dev/          # one host module, built as vm-dev (x86_64) and vm-dev-aarch64
│   └── workstation/     # from Phase 1 (needs the disk layout)
├── modules/
│   ├── core/
│   │   ├── discipline/
│   │   ├── tended-store/
│   │   ├── watch/
│   │   ├── condition/
│   │   ├── record/
│   │   ├── security/
│   │   ├── inheritance/
│   │   ├── offerings/
│   │   ├── backups/
│   │   └── impermanence/
│   └── experience/
│       ├── plasma/
│       ├── theme/
│       ├── sounds/
│       └── plymouth/
├── pkgs/
│   └── lantea/          # Rust workspace: cli, condition-daemon, shared lib
├── tests/               # nixosTest definitions, one per Strand
├── iso/
├── installer/
├── assets/              # wallpapers, sounds, fonts, icons
└── docs/
    ├── brief.md
    ├── philosophy.md
    ├── glossary.md
    ├── architecture.md
    ├── nine-strands.md
    ├── administrator.md
    ├── notes/           # proposals not yet decided
    ├── tasks/           # AiHarness task specs
    └── decisions/       # architecture decision records
```

---

## 8. The Nine Strands → policy

| Strand | Policy |
|---|---|
| Tend what is given | Nothing installed or running without a declared purpose |
| Take no more than needed | Every governed service declares resource ceilings |
| Leave no mark | Ephemeral caches/temp by default; full impermanence as optional profile |
| Redistribute load | Graceful degradation; no single points of failure in backups |
| What is kept, is kept well | Snapshots, scrubs, tested backups, VM tests for every module |
| Know the condition of the thing | Condition always available, in plain language |
| The work will wait | No forced updates, no interruptions, no surprise reboots |
| Speak plainly | Clear text everywhere; `--explain` reveals the real commands |
| Relieve the watch | Services stopped gracefully; force only with a recorded reason |

---

## 9. Experience specification

- **Palette:** ground `#0A0A0C` · limestone `#D9CDB8` · aged gold `#C9A227` · Adriatic blue `#1B3A5C` · lamp amber `#E8A33D` · text `#F2EDE4` · muted `#8A857C`
- **Fonts:** Cormorant Garamond (headings) · Inter (interface) · IBM Plex Mono (terminal)
- **Motion:** slow, weighted, 300–400 ms easing, nothing bounces
- **Sound:** harbour water; a single low bell instead of system beeps
- **Wallpaper:** near-black starfield with a coastline at the lower edge
- **Terminal prompt:** `lantea ~` in aged gold
- **Plymouth:** one calm line of Atlantean text; logs one keypress away

Open questions and proposals for Phase 7 are kept in [`notes/experience-proposals.md`](notes/experience-proposals.md).

---

## 10. How to work

- Work through AiHarness: `/aih:plan` → task spec in `docs/tasks/` → `/aih:implement` → `/aih:verify` → `/aih:review` → `/aih:ship`. Phase 0 already has its spec.
- Keep modules small and documented; every option has a description.
- Write the VM test alongside each feature, not after.
- When a choice isn't covered here, choose the simplest option consistent with the principles, and record it in `docs/decisions/`.
- Never run commands that modify the host system — the build machine included. Everything happens in VMs or CI. The harness guard enforces the worst cases; the rule covers all of them.
