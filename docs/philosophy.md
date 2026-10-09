# Philosophy

Lantea is an operating system built on the ethic of the Atlanteans in the novel *Aria Sum*. That ethic is called the **Discipline**, and Lantea expresses it in architecture, not decoration. A theme can be swapped out in an afternoon; the Discipline is what stays when the theme is gone.

## What the Discipline asks of the machine

- **The system is fully declared and reproducible.** What the machine is, is written down. Nothing important lives only in its current state.
- **Nothing is destroyed carelessly.** Things are *set down* and can be recovered. Permanent removal is a separate, deliberate act, and it is recorded.
- **Services stand on watch.** Each one has a stated purpose and a declared ceiling on what it may take.
- **The machine knows its condition** and reports it in plain language, not in percentages.
- **Every deliberate action leaves an honest record.**
- **Nothing is forced on the user.** Updates are *offered*, never imposed. The work will wait.

## Principles

1. **Interpretation, not deception.** Every Lantea term maps to a real Linux or Nix concept, and the real tools stay visible and usable. An experienced Linux administrator can work in Lantea without learning anything new. Every `lantea` command can `--explain` the real commands it runs.
2. **A layer on NixOS, not a fork.** Lantea is modules, packages and themes on top of NixOS.
3. **Core is separate from Experience.** The Discipline lives in Core and works without a desktop. The desktop only presents it.
4. **Speak plainly.** Clear, calm, specific. No unexplained codes, no alarm, no marketing voice.
5. **Nothing automatic that the user has not authorised once.**
6. **Reproducible and tested.** Every Core module ships with a VM test.
7. **Never act on the host.** Development and testing happen in virtual machines.

## The Nine Strands

| Strand | What it means in the system |
|---|---|
| Tend what is given | Nothing installed or running without a declared purpose |
| Take no more than needed | Every governed service declares resource ceilings |
| Leave no mark | Ephemeral caches and temp by default; full impermanence as an optional profile |
| Redistribute load | Graceful degradation; no single points of failure in backups |
| What is kept, is kept well | Snapshots, scrubs, tested backups, VM tests for every module |
| Know the condition of the thing | Condition always available, in plain language |
| The work will wait | No forced updates, no interruptions, no surprise reboots |
| Speak plainly | Clear text everywhere; `--explain` reveals the real commands |
| Relieve the watch | Services stopped gracefully; force only with a recorded reason |

The vocabulary is fixed in [`glossary.md`](glossary.md).
