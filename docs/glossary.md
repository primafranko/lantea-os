# Glossary

The fixed vocabulary of Lantea. Each term has exactly one meaning, and every term maps to a real Linux or Nix concept that stays visible underneath. If a new concept is needed, add it here **and** in [`brief.md`](brief.md) §2 first, in the same change.

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

## Words that are deliberately *not* terms

- **standing** as a verb ("services standing") — use *on watch*; *Standing* is only the stable track.
- **distressed** for a service — say *failing*; *Distressed* is only a Condition state.
- **release** as a noun for a version — use *edition*; *release* only means permanent removal from Held.
- **update** in user-facing text about the system — use *offering*; "update" is fine in plain explanations next to it ("1 offering (system update) is available").

## Open question

*Strand* means one policy module (`modules/…`), while §8 of the brief also speaks of the *Nine Strands* (principles). There are ten modules and nine Strands, so the two are not one-to-one. To be settled before a Strand is named in user-facing text.
