# The tended-store Strand, Phase 1: Held (the Steward's XDG Trash) with
# `lantea set-down`, `held`, `restore` and `release`, and the Record entries
# they leave in journald (decisions 0006, 0017 and 0018).
{ self }:
{
  name = "tended-store";

  nodes.machine =
    { pkgs, ... }:
    {
      imports = [ self.nixosModules.core ];
      lantea.enable = true;
      # `gio trash`, to show that Trash written by other tools is Held.
      environment.systemPackages = [ pkgs.glib ];
    };

  testScript = ''
    import json
    import re
    import shlex

    H = "/home/steward"
    T = f"{H}/.local/share/Trash"
    NOTHING = "Nothing was changed."

    machine.wait_for_unit("multi-user.target")

    def steward(cmd):
        """Runs cmd in the Steward's login shell: (exit status, stdout, stderr)."""
        status, out = machine.execute(
            f"su - steward -c {shlex.quote(cmd)} 2>/tmp/lantea-stderr"
        )
        err = machine.succeed("cat /tmp/lantea-stderr")
        return status, out, err

    def expect(cmd, code, stdout=None, stderr=None):
        status, out, err = steward(cmd)
        assert status == code, f"{cmd}: exit {status}, stdout {out!r}, stderr {err!r}"
        if stdout is not None:
            assert out == stdout, f"{cmd}: stdout {out!r}"
        if stderr is not None:
            assert err == stderr, f"{cmd}: stderr {err!r}"
        return out, err

    def held_ids():
        out, _ = expect("lantea held --json", 0)
        return [item["id"] for item in json.loads(out)["items"]]

    def set_down_line(path, held_id):
        return (
            f"Set down {path}. It is held as {held_id}; "
            f"to bring it back: lantea restore {held_id}"
        )

    with subtest("setup"):
        expect("printf 'twelve bytes' > ~/a.txt && printf 'four' > ~/b.txt", 0)

    with subtest("--explain changes nothing"):
        out, _ = expect("lantea set-down ~/a.txt --explain", 0)
        assert out.startswith(f"mkdir -p -m 700 -- {T}/files {T}/info\n"), out
        assert f"\nmv -n -- {H}/a.txt {T}/files/a.txt\n" in out, out
        assert "\nlogger --journald <<'EOF'\n" in out and out.endswith("\nEOF\n"), out
        expect("test -f ~/a.txt", 0)
        expect(f"test ! -e {T}/files || test -z \"$(ls -A {T}/files)\"", 0)

    with subtest("set down a file"):
        expect("lantea set-down ~/a.txt", 0, set_down_line(f"{H}/a.txt", "a.txt") + "\n", "")
        expect("test ! -e ~/a.txt", 0)
        expect(f"cat {T}/files/a.txt", 0, "twelve bytes")
        info = machine.succeed(f"cat {T}/info/a.txt.trashinfo")
        assert re.fullmatch(
            r"\[Trash Info\]\nPath=/home/steward/a\.txt\n"
            r"DeletionDate=\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}\n",
            info,
        ), info
        out, _ = expect("lantea held", 0)
        assert re.fullmatch(
            r"Held · 1 item\n  a\.txt  /home/steward/a\.txt  12 B  "
            r"set down \d{4}-\d{2}-\d{2} \d{2}:\d{2}\n",
            out,
        ), out

    with subtest("restore it"):
        expect("lantea restore a.txt", 0, f"Restored a.txt to {H}/a.txt.\n", "")
        expect("cat ~/a.txt", 0, "twelve bytes")
        expect("lantea held", 0, "Nothing is held.\n")

    with subtest("release asks, and only on a terminal"):
        expect("lantea set-down ~/b.txt", 0, set_down_line(f"{H}/b.txt", "b.txt") + "\n")
        expect(
            "echo y | lantea release b.txt",
            1,
            "",
            "Release asks before deleting anything, but there is no terminal to ask on. "
            "To release without being asked: lantea release b.txt --yes\n",
        )
        assert held_ids() == ["b.txt"]
        prompt = (
            f"Release b.txt ({H}/b.txt, 4 B)? It will be permanently deleted. [y/N] "
        )
        out, _ = expect("echo n | script -qec 'lantea release b.txt' /dev/null", 1)
        assert prompt in out and "Nothing was released." in out, out
        assert held_ids() == ["b.txt"]
        out, _ = expect("echo y | script -qec 'lantea release b.txt' /dev/null", 0)
        assert prompt in out, out
        assert f"Released b.txt. {H}/b.txt has been permanently deleted." in out, out
        expect(f"test ! -e {T}/files/b.txt && test ! -e {T}/info/b.txt.trashinfo", 0)

    with subtest("items trashed by other tools are Held"):
        status, out, err = steward("printf 'gio' > ~/c.txt && gio trash ~/c.txt")
        assert status == 0, f"gio trash: exit {status}, stderr {err!r}"
        out, _ = expect("lantea held --json", 0)
        items = json.loads(out)["items"]
        assert len(items) == 1, items
        item = items[0]
        assert (item["id"], item["path"], item["size"]) == ("c.txt", f"{H}/c.txt", 3), item
        assert re.fullmatch(r"\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}", item["deleted_at"]), item
        expect(
            "lantea release c.txt --yes",
            0,
            f"Released c.txt. {H}/c.txt has been permanently deleted.\n",
        )

    with subtest("restore never overwrites, and --to restores elsewhere"):
        expect("lantea set-down ~/a.txt", 0)
        expect("printf 'new' > ~/a.txt", 0)
        expect(
            "lantea restore a.txt",
            1,
            "",
            f"{H}/a.txt already exists, so a.txt was not restored. "
            "To restore it elsewhere: lantea restore a.txt --to <path>\n",
        )
        expect("cat ~/a.txt", 0, "new")
        expect(
            f"lantea restore a.txt --to {H}/a-restored.txt",
            0,
            f"Restored a.txt to {H}/a-restored.txt.\n",
        )
        expect("cat ~/a-restored.txt", 0, "twelve bytes")

    with subtest("restore recreates missing parents"):
        expect("mkdir -p ~/gone/deeper && printf 'd' > ~/gone/deeper/d.txt", 0)
        expect("lantea set-down ~/gone/deeper/d.txt", 0)
        expect("rm -r ~/gone", 0)
        expect(
            "lantea restore d.txt",
            0,
            f"Recreated {H}/gone/deeper, which no longer existed.\n"
            f"Restored d.txt to {H}/gone/deeper/d.txt.\n",
        )
        expect("cat ~/gone/deeper/d.txt", 0, "d")

    with subtest("a symlink is set down as itself"):
        hostname = machine.succeed("cat /etc/hostname")
        expect("ln -s /etc/hostname ~/link", 0)
        expect("lantea set-down ~/link", 0, set_down_line(f"{H}/link", "link") + "\n")
        expect(f"test -L {T}/files/link", 0)
        expect(f"readlink {T}/files/link", 0, "/etc/hostname\n")
        assert machine.succeed("cat /etc/hostname") == hostname
        expect(
            "lantea release link --yes",
            0,
            f"Released link. {H}/link has been permanently deleted.\n",
        )
        assert machine.succeed("cat /etc/hostname") == hostname

    with subtest("an untrusted Path= is not restored outside the home"):
        expect(
            f"printf e > {T}/files/evil && "
            f"printf '[Trash Info]\\nPath=/tmp/evil\\nDeletionDate=2026-01-01T00:00:00\\n'"
            f" > {T}/info/evil.trashinfo",
            0,
        )
        expect(
            "lantea restore evil",
            1,
            "",
            "evil was set down from /tmp/evil, which is outside the Tended Store, "
            "so it was not restored there. "
            "To restore it elsewhere: lantea restore evil --to <path>\n",
        )
        machine.succeed("test ! -e /tmp/evil")
        expect(
            f"lantea restore evil --to {H}/evil.txt",
            0,
            f"Restored evil to {H}/evil.txt.\n",
        )

    with subtest("refusals"):
        def refused(cmd, line):
            expect(cmd, 1, "", line + "\n")

        def outside(path):
            return f"{path} is outside the Tended Store, so it cannot be set down. {NOTHING}"

        def contains(path):
            return f"{path} contains your Held items, so it cannot be set down. {NOTHING}"

        def unknown(held_id):
            return f"Nothing is held as {held_id}. To see what is held: lantea held"

        refused("lantea set-down ~/missing.txt", f"{H}/missing.txt does not exist. {NOTHING}")
        expect("printf x > /dev/shm/x", 0)
        refused("lantea set-down /dev/shm/x", outside("/dev/shm/x"))
        expect("printf y > /tmp/y", 0)
        refused("lantea set-down /tmp/y", outside("/tmp/y"))
        expect("ln -s /tmp ~/tmplink", 0)
        refused("lantea set-down ~/tmplink/y", outside(f"{H}/tmplink/y"))
        machine.succeed("test -f /dev/shm/x && test -f /tmp/y")
        refused("lantea set-down ~", contains(H))
        refused("lantea set-down ~/.local/share/Trash/files", contains(f"{T}/files"))
        refused("lantea restore nope", unknown("nope"))
        refused("lantea restore ../a.txt", unknown("../a.txt"))
        refused("lantea release . --yes", unknown("."))
        machine.succeed(
            "mkdir /home/steward/mnt"
            " && mount -t tmpfs tmpfs /home/steward/mnt"
            " && chown steward /home/steward/mnt"
        )
        expect("printf z > ~/mnt/z", 0)
        refused(
            "lantea set-down ~/mnt/z",
            f"{H}/mnt/z is on a separate mount from your Held items, "
            f"so it cannot be set down. {NOTHING}",
        )
        expect("test -f ~/mnt/z", 0)
        machine.succeed(
            "mkdir -p /home/steward/withmount/inner"
            " && mount -t tmpfs tmpfs /home/steward/withmount/inner"
            " && chown -R steward /home/steward/withmount"
        )
        refused(
            "lantea set-down ~/withmount",
            f"{H}/withmount contains {H}/withmount/inner, which is a separate mount, "
            f"so it cannot be set down. {NOTHING}",
        )
        expect("test -d ~/withmount/inner", 0)

    with subtest("a release that meets a mount leaves it in place"):
        expect(
            f"mkdir -p {T}/files/m/sub && printf 'm' > {T}/files/m/top.txt && "
            f"printf '[Trash Info]\\nPath=/home/steward/m\\nDeletionDate=2026-01-01T00:00:00\\n'"
            f" > {T}/info/m.trashinfo",
            0,
        )
        machine.succeed(
            f"mount -t tmpfs tmpfs {T}/files/m/sub"
            f" && printf 'keep' > {T}/files/m/sub/keep.txt"
            f" && chown -R steward {T}/files/m/sub"
        )
        out, _ = expect("lantea held --json", 0)
        m_size = [i for i in json.loads(out)["items"] if i["id"] == "m"][0]["size"]
        assert isinstance(m_size, int), m_size
        m_incomplete = (
            f"m could not be released completely: {T}/files/m/sub is a separate mount, "
            "so it was left in place. The release is in the Record, marked as incomplete. "
            "Some of it may already be deleted. "
            "To see what is held: lantea held"
        )
        expect("lantea release m --yes", 1, "", m_incomplete + "\n")
        expect(f"cat {T}/files/m/sub/keep.txt", 0, "keep")
        expect(f"test ! -e {T}/files/m/top.txt", 0)
        expect(f"test -f {T}/info/m.trashinfo", 0)
        assert "m" in held_ids()
        machine.succeed(f"umount {T}/files/m/sub")

    with subtest("an item that is itself a mount, tmpfs or bind, is refused"):
        machine.succeed(
            "mkdir -p /home/steward/bindsrc /home/steward/bound"
            " && mount --bind /home/steward/bindsrc /home/steward/bound"
        )
        refused(
            "lantea set-down ~/bound",
            f"{H}/bound is on a separate mount from your Held items, "
            f"so it cannot be set down. {NOTHING}",
        )
        for held_id, mount in [
            ("mp", "mount -t tmpfs tmpfs"),
            ("bm", "mount --bind /home/steward/bindsrc"),
        ]:
            expect(
                f"mkdir -p {T}/files/{held_id} && "
                f"printf '[Trash Info]\\nPath=/home/steward/{held_id}\\nDeletionDate=2026-01-01T00:00:00\\n'"
                f" > {T}/info/{held_id}.trashinfo",
                0,
            )
            machine.succeed(f"{mount} {T}/files/{held_id}")
            out, _ = expect("lantea held --json", 0)
            item = [i for i in json.loads(out)["items"] if i["id"] == held_id][0]
            assert item["size"] is None, item
            refused(
                f"lantea restore {held_id}",
                f"{H}/{held_id} is on a separate mount from your Held items, "
                f"so {held_id} was not restored. {NOTHING}",
            )
            refused(
                f"lantea release {held_id} --yes",
                f"{held_id} is a separate mount, so it cannot be released. {NOTHING}",
            )
            machine.succeed(f"umount {T}/files/{held_id}")

    with subtest("a home that belongs to another user is not touched"):
        status, out = machine.execute("HOME=/home/steward lantea held 2>/tmp/root-stderr")
        err = machine.succeed("cat /tmp/root-stderr")
        assert status == 1 and out == "", (status, out, err)
        assert err == (
            f"Your home directory {H} belongs to another user, so lantea will not act on it. "
            f"Run lantea as that user instead. {NOTHING}\n"
        ), err

    with subtest("the installed binary has no socket override"):
        out = machine.execute(
            'grep -c LANTEA_JOURNAL_SOCKET "$(readlink -f "$(command -v lantea)")"'
        )[1]
        assert out == "0\n", out

    with subtest("the Record holds one entry per action"):
        uid = machine.succeed("id -u steward").strip()
        machine.wait_until_succeeds("test $(journalctl -t lantea -o json | wc -l) -ge 14")
        entries = [
            json.loads(line)
            for line in machine.succeed("journalctl -t lantea -o json").splitlines()
        ]
        ids = {
            "set-down": "2667c9a683a44b908958fc443423992e",
            "restore": "74f4239d420c461791598bcc3b4991cc",
            "release": "9095fa701fd14a6c94a119136543dbea",
            "release-incomplete": "a2a943364ecc42288bf2ab9c44ae4db8",
        }

        def entry(action, held_id, path, size=None):
            message = {
                "set-down": set_down_line(path, held_id),
                "restore": f"Restored {held_id} to {path}.",
                "release": f"Released {held_id}. {path} has been permanently deleted.",
                "release-incomplete": m_incomplete,
            }[action]
            fields = {
                "MESSAGE": message,
                "MESSAGE_ID": ids[action],
                "PRIORITY": "4" if action == "release-incomplete" else "5",
                "SYSLOG_IDENTIFIER": "lantea",
                "LANTEA_RECORD_VERSION": "1",
                "LANTEA_ACTION": action,
                "LANTEA_HELD_ID": held_id,
                "LANTEA_PATH": path,
            }
            if size is not None:
                fields["LANTEA_SIZE"] = str(size)
            return fields

        expected = [
            entry("set-down", "a.txt", f"{H}/a.txt", 12),
            entry("restore", "a.txt", f"{H}/a.txt"),
            entry("set-down", "b.txt", f"{H}/b.txt", 4),
            entry("release", "b.txt", f"{H}/b.txt", 4),
            entry("release", "c.txt", f"{H}/c.txt", 3),
            entry("set-down", "a.txt", f"{H}/a.txt", 12),
            entry("restore", "a.txt", f"{H}/a-restored.txt"),
            entry("set-down", "d.txt", f"{H}/gone/deeper/d.txt", 1),
            entry("restore", "d.txt", f"{H}/gone/deeper/d.txt"),
            entry("set-down", "link", f"{H}/link", len("/etc/hostname")),
            entry("release", "link", f"{H}/link", len("/etc/hostname")),
            entry("restore", "evil", f"{H}/evil.txt"),
            entry("release", "m", f"{H}/m", m_size),
            entry("release-incomplete", "m", f"{H}/m", m_size),
        ]
        assert len(entries) == len(expected), entries
        for got, want in zip(entries, expected):
            # Fields without a leading underscore are the ones lantea wrote.
            written = {k: v for k, v in got.items() if not k.startswith("_")}
            assert written == want, (written, want)
            assert got["_UID"] == uid, got
  '';
}
