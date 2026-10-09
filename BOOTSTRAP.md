# Bootstrap — from this folder to Phase 0

One-time steps. Delete this file once Phase 0 is merged.

## 0. Already done

- `lantea-bench` runs NixOS 26.05 with flakes, `/dev/kvm`, `git`, `gh`, `nodejs` and Claude Code for `dev`.
- As `dev`: `gh auth login` with the fine-grained token, git name and no-reply email set.
- `~/aiharness` is cloned with branch `v2` checked out and registered as the `aiharness` marketplace. That marketplace *is* the `~/aiharness` folder, so Claude Code uses whatever branch is checked out there.
- On GitHub: the empty public repo `primafranko/lantea-os` and the private `primafranko/aiharness`.

## 1. On Windows — put the starter on GitHub

In cmd, in the folder that contains this file (`Documents\Projects\lantea-os`):

```
git init -b main
git add -A
git commit -m "docs: brief v1.1, glossary, decisions, harness setup"
git remote add origin https://github.com/primafranko/lantea-os.git
git push -u origin main
```

The first push may open a browser window to sign in to GitHub.

## 2. On the build machine (as `dev`) — clone it

```
git clone https://github.com/primafranko/lantea-os.git ~/lantea-os
```

## 3. On the build machine — add the `nix` and `rust` packs to AiHarness

The packs travel in `tooling/aiharness-packs/`. They go to a branch of AiHarness v2 and a pull request for you to review; the build machine uses that branch until it is merged.

```
cd ~/aiharness
git switch v2
git pull
git switch -c packs/nix-rust
cp -r ~/lantea-os/tooling/aiharness-packs/nix ~/lantea-os/tooling/aiharness-packs/rust plugin/packs/
node --test "test/*.test.mjs"
git add plugin/packs
git commit -m "feat(packs): nix and rust stack packs"
git push -u origin packs/nix-rust
gh pr create --base v2 --title "nix and rust stack packs" --body "First used by lantea-os."
claude plugin marketplace update aiharness
```

`node --test "test/*.test.mjs"` runs the harness's own tests (about 500, with a few slow ones skipped); they must stay green. If `claude plugin marketplace update` is not a known command in your Claude Code version, skip it.

Then remove the packs from Lantea, where they no longer belong:

```
cd ~/lantea-os
git rm -r tooling
git commit -m "chore: packs moved to AiHarness"
```

## 4. On the build machine — adopt AiHarness in Lantea

```
mkdir -p ~/.local/bin
ln -sf ~/aiharness/plugin/bin/aih ~/.local/bin/aih
cd ~/lantea-os
aih init --yes
aih doctor
git add -A
git commit -m "chore: adopt AiHarness"
git push
```

The link makes `aih` available in your own shell too (inside Claude sessions the plugin provides it). `aih init` keeps the existing `CLAUDE.md` and `.claude/harness.json` and adds the managed rules (including `aih-nix-nix.md` and `aih-rust-rust.md`) and settings. Expected until Phase 0 is done:

- the doctor warns that the `rust` pack has no `test` check (Phase 0 adds the Rust package to `harness.json`);
- `aih verify` fails the Nix checks, because there is no `flake.nix` yet. `/aih:init` records that as the starting state, and Phase 0 turns it green.

## 5. Start the agent

```
cd ~/lantea-os
claude
```

Trust the folder, then:

1. `/aih:init` — checks the setup and records the baseline. Answer its questions.
2. `/aih:implement docs/tasks/001-phase-0-foundation.md`
3. `/aih:verify`, `/aih:review`, `/aih:ship` — a pull request on GitHub, with CI.

After Phase 0, start the agent inside the dev shell instead: `nix develop -c claude`.
