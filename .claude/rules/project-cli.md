---
paths:
  - "pkgs/lantea/**"
  - "tests/**"
---
# cli — project rules (written by /aih:distill; do not hand-edit)

- WHEN lantea code or a VM test walks, moves, sizes or deletes a tree, or checks a mount or subvolume boundary → decide "same mount" only through the shared mount-ID helper (`statx` STATX_MNT_ID, base = the Held `files` directory), and check the root item itself, not only what is below it. Never use `st_dev`: it is wrong both ways — Btrfs subvolumes each have their own `st_dev` on one mount, and a bind mount shares one `st_dev` across two mounts. <!-- id=C-1 n=3 since=2026-10 check=test:mount_situations_table -->
- WHEN lantea prints a name, path or value that came from the file system or from a file other programs can write → build the message raw and escape it once at the print boundary (control characters and the backslash as `\xHH`), prompts and `--explain` included. <!-- id=C-2 n=2 since=2026-10 check=test:control_characters_are_escaped_on_the_terminal_only -->
