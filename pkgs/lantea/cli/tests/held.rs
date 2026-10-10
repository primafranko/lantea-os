//! `lantea set-down`, `held`, `restore` and `release`, and the Record entries
//! they write (decisions 0017 and 0018), against a temporary home and a
//! datagram socket bound by the test in place of journald.

#[cfg(not(feature = "test-journal-socket"))]
compile_error!(
    "cli/tests/held.rs needs the test-journal-socket feature: run cargo test with --all-features"
);

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::Write;
use std::os::unix::fs::{MetadataExt, PermissionsExt, symlink};
use std::os::unix::net::UnixDatagram;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

const SET_DOWN_ID: &str = "2667c9a683a44b908958fc443423992e";
const RESTORE_ID: &str = "74f4239d420c461791598bcc3b4991cc";
const RELEASE_ID: &str = "9095fa701fd14a6c94a119136543dbea";
const RELEASE_INCOMPLETE_ID: &str = "a2a943364ecc42288bf2ab9c44ae4db8";

/// What `du -s --apparent-size --block-size=1` reports: the `lstat` size of
/// the path and every entry under it, hard links once, symlinks not followed.
fn du_apparent(path: &Path) -> u64 {
    fn walk(path: &Path, seen: &mut BTreeSet<(u64, u64)>) -> u64 {
        let meta = fs::symlink_metadata(path).unwrap();
        if !seen.insert((meta.dev(), meta.ino())) {
            return 0;
        }
        let mut total = meta.len();
        if meta.is_dir() {
            for entry in fs::read_dir(path).unwrap() {
                total += walk(&entry.unwrap().path(), seen);
            }
        }
        total
    }
    walk(path, &mut BTreeSet::new())
}

struct Env {
    _root: tempfile::TempDir,
    /// The temporary home, canonical.
    home: PathBuf,
    /// A directory next to the home, outside the Tended Store.
    outside: PathBuf,
    /// `<home>/.local/share/Trash`.
    trash: PathBuf,
    sock_path: PathBuf,
    sock: UnixDatagram,
}

impl Env {
    fn new() -> Env {
        let root = tempfile::tempdir().expect("a temporary directory");
        let base = fs::canonicalize(root.path()).expect("canonical temp dir");
        let home = base.join("home");
        let outside = base.join("outside");
        fs::create_dir(&home).unwrap();
        fs::create_dir(&outside).unwrap();
        let sock_path = base.join("journal.socket");
        let sock = UnixDatagram::bind(&sock_path).expect("bind the test journal socket");
        sock.set_nonblocking(true).unwrap();
        let trash = home.join(".local/share/Trash");
        Env {
            _root: root,
            home,
            outside,
            trash,
            sock_path,
            sock,
        }
    }

    fn h(&self, rel: &str) -> String {
        format!("{}/{rel}", self.home.display())
    }

    fn t(&self, rel: &str) -> String {
        format!("{}/{rel}", self.trash.display())
    }

    fn command(&self, args: &[&str]) -> Command {
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_lantea"));
        cmd.args(args)
            .current_dir(&self.home)
            .env("HOME", &self.home)
            .env("XDG_DATA_HOME", self.home.join(".local/share"))
            .env("LANTEA_JOURNAL_SOCKET", &self.sock_path);
        cmd
    }

    /// Runs lantea with stdin from /dev/null (not a terminal).
    fn run(&self, args: &[&str]) -> Output {
        self.command(args)
            .stdin(Stdio::null())
            .output()
            .expect("the lantea binary runs")
    }

    fn run_with_stdin(&self, args: &[&str], input: &str) -> Output {
        let mut child = self
            .command(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("the lantea binary runs");
        // lantea may exit without reading; a broken pipe is fine here.
        let _ = child.stdin.take().unwrap().write_all(input.as_bytes());
        child.wait_with_output().unwrap()
    }

    /// Every datagram received so far, decoded into fields.
    fn entries(&self) -> Vec<BTreeMap<String, Vec<u8>>> {
        let mut out = Vec::new();
        let mut buf = vec![0u8; 65536];
        while let Ok(n) = self.sock.recv(&mut buf) {
            out.push(decode(&buf[..n]));
        }
        out
    }

    fn write(&self, rel: &str, content: &str) {
        let p = self.home.join(rel);
        if let Some(parent) = p.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(p, content).unwrap();
    }

    /// Puts an item into the Trash the way another tool would.
    fn hold(&self, id: &str, content: &str, info: &str) {
        fs::create_dir_all(self.trash.join("files")).unwrap();
        fs::create_dir_all(self.trash.join("info")).unwrap();
        fs::write(self.trash.join("files").join(id), content).unwrap();
        fs::write(
            self.trash.join("info").join(format!("{id}.trashinfo")),
            info,
        )
        .unwrap();
    }
}

/// Decodes journald's native protocol, both the `KEY=value` and binary forms.
fn decode(mut data: &[u8]) -> BTreeMap<String, Vec<u8>> {
    let mut fields = BTreeMap::new();
    while !data.is_empty() {
        let end = data
            .iter()
            .position(|&b| b == b'=' || b == b'\n')
            .expect("a field name ends");
        let key = String::from_utf8(data[..end].to_vec()).unwrap();
        let value;
        if data[end] == b'=' {
            let rest = &data[end + 1..];
            let nl = rest.iter().position(|&b| b == b'\n').expect("a newline");
            value = rest[..nl].to_vec();
            data = &rest[nl + 1..];
        } else {
            let rest = &data[end + 1..];
            let len = u64::from_le_bytes(rest[..8].try_into().unwrap()) as usize;
            value = rest[8..8 + len].to_vec();
            assert_eq!(rest[8 + len], b'\n', "binary field {key} ends in a newline");
            data = &rest[8 + len + 1..];
        }
        assert!(fields.insert(key, value).is_none(), "duplicate field");
    }
    fields
}

fn expected_entry(
    message: &str,
    action: &str,
    id: &str,
    path: &str,
    size: Option<u64>,
) -> BTreeMap<String, Vec<u8>> {
    let message_id = match action {
        "set-down" => SET_DOWN_ID,
        "restore" => RESTORE_ID,
        "release-incomplete" => RELEASE_INCOMPLETE_ID,
        _ => RELEASE_ID,
    };
    let priority = if action == "release-incomplete" {
        "4"
    } else {
        "5"
    };
    let mut m = BTreeMap::new();
    let mut put = |k: &str, v: &str| {
        m.insert(k.to_string(), v.as_bytes().to_vec());
    };
    put("MESSAGE", message);
    put("MESSAGE_ID", message_id);
    put("PRIORITY", priority);
    put("SYSLOG_IDENTIFIER", "lantea");
    put("LANTEA_RECORD_VERSION", "1");
    put("LANTEA_ACTION", action);
    put("LANTEA_HELD_ID", id);
    put("LANTEA_PATH", path);
    if let Some(size) = size {
        put("LANTEA_SIZE", &size.to_string());
    }
    m
}

fn stdout(o: &Output) -> String {
    String::from_utf8(o.stdout.clone()).unwrap()
}

fn stderr(o: &Output) -> String {
    String::from_utf8(o.stderr.clone()).unwrap()
}

#[track_caller]
fn assert_ok(o: &Output, expected_stdout: &str) {
    assert_eq!(o.status.code(), Some(0), "stderr: {}", stderr(o));
    assert_eq!(stdout(o), expected_stdout);
    assert_eq!(stderr(o), "");
}

#[track_caller]
fn assert_refused(o: &Output, expected_stderr: &str) {
    assert_eq!(o.status.code(), Some(1), "stdout: {}", stdout(o));
    assert_eq!(stdout(o), "");
    assert_eq!(stderr(o), expected_stderr);
}

fn assert_deletion_date(date: &str) {
    let b = date.as_bytes();
    assert_eq!(b.len(), 19, "{date}");
    for (i, c) in b.iter().enumerate() {
        match i {
            4 | 7 => assert_eq!(*c, b'-', "{date}"),
            10 => assert_eq!(*c, b'T', "{date}"),
            13 | 16 => assert_eq!(*c, b':', "{date}"),
            _ => assert!(c.is_ascii_digit(), "{date}"),
        }
    }
}

/// Checks an info file is exactly the three lines and returns its date.
fn read_info(env: &Env, id: &str, path: &str) -> String {
    let info = fs::read_to_string(env.trash.join("info").join(format!("{id}.trashinfo"))).unwrap();
    let prefix = format!("[Trash Info]\nPath={path}\nDeletionDate=");
    assert!(info.starts_with(&prefix), "{info:?}");
    let date = info[prefix.len()..]
        .strip_suffix('\n')
        .expect("ends in a newline");
    assert_deletion_date(date);
    date.to_string()
}

// ---------------------------------------------------------------- set-down

#[test]
fn set_down_moves_the_file_writes_the_info_and_the_record() {
    let env = Env::new();
    env.write("a.txt", "twelve bytes");
    let inode = fs::metadata(env.home.join("a.txt")).unwrap().ino();

    let out = env.run(&["set-down", &env.h("a.txt")]);
    let sentence = format!(
        "Set down {}. It is held as a.txt; to bring it back: lantea restore a.txt",
        env.h("a.txt")
    );
    assert_ok(&out, &format!("{sentence}\n"));

    assert!(!env.home.join("a.txt").exists());
    let held = env.trash.join("files/a.txt");
    assert_eq!(fs::read_to_string(&held).unwrap(), "twelve bytes");
    assert_eq!(fs::metadata(&held).unwrap().ino(), inode);
    read_info(&env, "a.txt", &env.h("a.txt"));

    assert_eq!(
        env.entries(),
        vec![expected_entry(
            &sentence,
            "set-down",
            "a.txt",
            &env.h("a.txt"),
            Some(12)
        )]
    );
}

#[test]
fn set_down_relative_path_is_made_absolute() {
    let env = Env::new();
    env.write("a.txt", "twelve bytes");
    let out = env.run(&["set-down", "./a.txt"]);
    assert_ok(
        &out,
        &format!(
            "Set down {}. It is held as a.txt; to bring it back: lantea restore a.txt\n",
            env.h("a.txt")
        ),
    );
}

#[test]
fn set_down_percent_encodes_the_path_and_quotes_the_id() {
    let env = Env::new();
    env.write("two words.txt", "x");
    let out = env.run(&["set-down", &env.h("two words.txt")]);
    assert_ok(
        &out,
        &format!(
            "Set down {}. It is held as two words.txt; to bring it back: lantea restore 'two words.txt'\n",
            env.h("two words.txt")
        ),
    );
    read_info(&env, "two words.txt", &env.h("two%20words.txt"));
}

#[test]
fn set_down_takes_the_next_free_id() {
    let env = Env::new();
    env.write("a.txt", "one");
    assert_eq!(
        env.run(&["set-down", &env.h("a.txt")]).status.code(),
        Some(0)
    );
    env.write("a.txt", "two");
    let out = env.run(&["set-down", &env.h("a.txt")]);
    assert_ok(
        &out,
        &format!(
            "Set down {}. It is held as a.txt.2; to bring it back: lantea restore a.txt.2\n",
            env.h("a.txt")
        ),
    );
    assert_eq!(
        fs::read_to_string(env.trash.join("files/a.txt.2")).unwrap(),
        "two"
    );
    read_info(&env, "a.txt.2", &env.h("a.txt"));
    let entries = env.entries();
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[1]["LANTEA_HELD_ID"], b"a.txt.2");
}

#[test]
fn set_down_a_directory_has_its_du_apparent_size_and_does_not_follow_symlinks() {
    let env = Env::new();
    env.write("dir/one", "abc");
    env.write("dir/sub/two", "defgh");
    fs::hard_link(env.home.join("dir/one"), env.home.join("dir/sub/hard")).unwrap();
    fs::write(env.outside.join("big"), "0123456789").unwrap();
    symlink(env.outside.join("big"), env.home.join("dir/sub/link")).unwrap();
    let size = du_apparent(&env.home.join("dir"));
    // Directories, the link's own size, the hard-linked file once.
    let link_len = env.outside.join("big").as_os_str().len() as u64;
    let dirs = fs::metadata(env.home.join("dir")).unwrap().len()
        + fs::metadata(env.home.join("dir/sub")).unwrap().len();
    assert_eq!(size, dirs + 3 + 5 + link_len);

    let out = env.run(&["set-down", "--json", &env.h("dir")]);
    assert_ok(
        &out,
        &format!(
            "{{\"action\":\"set-down\",\"id\":\"dir\",\"path\":\"{}\",\"size\":{size}}}\n",
            env.h("dir")
        ),
    );
    assert_eq!(du_apparent(&env.trash.join("files/dir")), size);
    let text = stdout(&env.run(&["held", "--json"]));
    assert!(text.contains(&format!("\"size\":{size}}}")), "{text}");
    assert!(env.trash.join("files/dir/sub/two").is_file());
    assert_eq!(
        fs::read_to_string(env.outside.join("big")).unwrap(),
        "0123456789"
    );
}

#[test]
fn set_down_a_symlink_moves_the_link_itself() {
    let env = Env::new();
    let target = env.outside.join("hostname");
    fs::write(&target, "host").unwrap();
    symlink(&target, env.home.join("link")).unwrap();

    let out = env.run(&["set-down", "--json", &env.h("link")]);
    let size = target.as_os_str().len();
    assert_ok(
        &out,
        &format!(
            "{{\"action\":\"set-down\",\"id\":\"link\",\"path\":\"{}\",\"size\":{size}}}\n",
            env.h("link")
        ),
    );
    let held = env.trash.join("files/link");
    assert!(
        fs::symlink_metadata(&held)
            .unwrap()
            .file_type()
            .is_symlink()
    );
    assert_eq!(fs::read_link(&held).unwrap(), target);
    assert_eq!(fs::read_to_string(&target).unwrap(), "host");
    assert!(fs::symlink_metadata(env.home.join("link")).is_err());

    // Release removes the link, never its target.
    let out = env.run(&["release", "link", "--yes"]);
    assert_eq!(out.status.code(), Some(0), "{}", stderr(&out));
    assert!(fs::symlink_metadata(&held).is_err());
    assert_eq!(fs::read_to_string(&target).unwrap(), "host");
}

#[test]
fn set_down_json() {
    let env = Env::new();
    env.write("a.txt", "twelve bytes");
    let out = env.run(&["--json", "set-down", &env.h("a.txt")]);
    assert_ok(
        &out,
        &format!(
            "{{\"action\":\"set-down\",\"id\":\"a.txt\",\"path\":\"{}\",\"size\":12}}\n",
            env.h("a.txt")
        ),
    );
    assert_eq!(env.entries().len(), 1);
}

#[test]
fn set_down_refusals_change_nothing_and_write_no_record() {
    let env = Env::new();
    env.write("a.txt", "twelve bytes");
    let nothing = "Nothing was changed.";

    // Missing.
    let out = env.run(&["set-down", &env.h("missing.txt")]);
    assert_refused(
        &out,
        &format!("{} does not exist. {nothing}\n", env.h("missing.txt")),
    );
    let out = env.run(&["set-down", &env.h("gone/missing.txt")]);
    assert_refused(
        &out,
        &format!("{} does not exist. {nothing}\n", env.h("gone/missing.txt")),
    );

    // Outside the home.
    let x = env.outside.join("x");
    fs::write(&x, "x").unwrap();
    let out = env.run(&["set-down", x.to_str().unwrap()]);
    assert_refused(
        &out,
        &format!(
            "{} is outside the Tended Store, so it cannot be set down. {nothing}\n",
            x.display()
        ),
    );
    assert!(x.exists());

    // Through a parent symlink that leads outside.
    symlink(&env.outside, env.home.join("outlink")).unwrap();
    let out = env.run(&["set-down", &env.h("outlink/x")]);
    assert_refused(
        &out,
        &format!(
            "{} is outside the Tended Store, so it cannot be set down. {nothing}\n",
            env.h("outlink/x")
        ),
    );
    assert!(x.exists());

    // The home, the Trash, anything in it, anything containing it.
    fs::create_dir_all(env.trash.join("files")).unwrap();
    fs::write(env.trash.join("files/held"), "h").unwrap();
    for p in [
        env.home.display().to_string(),
        env.trash.display().to_string(),
        env.t("files"),
        env.t("info"),
        env.t("files/held"),
        env.h(".local"),
        env.h(".local/share"),
    ] {
        let out = env.run(&["set-down", &p]);
        assert_refused(
            &out,
            &format!("{p} contains your Held items, so it cannot be set down. {nothing}\n"),
        );
    }
    assert!(env.trash.join("files/held").exists());
    assert!(env.home.join("a.txt").exists());
    assert_eq!(env.entries(), vec![]);
}

#[test]
fn set_down_explain_prints_the_commands_and_changes_nothing() {
    let env = Env::new();
    env.write("a.txt", "twelve bytes");
    let out = env.run(&["set-down", "--explain", "--json", &env.h("a.txt")]);
    assert_eq!(out.status.code(), Some(0), "{}", stderr(&out));
    let text = stdout(&out);
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(
        lines[0],
        format!("mkdir -p -m 700 -- {} {}", env.t("files"), env.t("info"))
    );
    let printf = format!(
        "printf '[Trash Info]\\nPath=%s\\nDeletionDate=%s\\n' '{}' '",
        env.h("a.txt")
    );
    assert!(lines[1].starts_with(&printf), "{}", lines[1]);
    let tail = format!("' > {}", env.t("info/a.txt.trashinfo"));
    assert!(lines[1].ends_with(&tail), "{}", lines[1]);
    assert_deletion_date(&lines[1][printf.len()..lines[1].len() - tail.len()]);
    assert_eq!(
        lines[2],
        format!("mv -n -- {} {}", env.h("a.txt"), env.t("files/a.txt"))
    );
    let sentence = format!(
        "Set down {}. It is held as a.txt; to bring it back: lantea restore a.txt",
        env.h("a.txt")
    );
    assert_eq!(
        lines[3..],
        [
            "logger --journald <<'EOF'".to_string(),
            format!("MESSAGE={sentence}"),
            format!("MESSAGE_ID={SET_DOWN_ID}"),
            "PRIORITY=5".to_string(),
            "SYSLOG_IDENTIFIER=lantea".to_string(),
            "LANTEA_RECORD_VERSION=1".to_string(),
            "LANTEA_ACTION=set-down".to_string(),
            "LANTEA_HELD_ID=a.txt".to_string(),
            format!("LANTEA_PATH={}", env.h("a.txt")),
            "LANTEA_SIZE=12".to_string(),
            "EOF".to_string(),
        ]
    );
    assert!(text.ends_with("EOF\n"));
    assert!(env.home.join("a.txt").exists());
    assert!(!env.trash.exists());
    assert_eq!(env.entries(), vec![]);
}

#[test]
fn set_down_explain_quotes_paths_when_needed() {
    let env = Env::new();
    env.write("two words.txt", "x");
    let out = env.run(&["set-down", "--explain", &env.h("two words.txt")]);
    assert_eq!(out.status.code(), Some(0), "{}", stderr(&out));
    let text = stdout(&out);
    let line = text.lines().nth(2).unwrap().to_string();
    assert_eq!(
        line,
        format!(
            "mv -n -- '{}' '{}'",
            env.h("two words.txt"),
            env.t("files/two words.txt")
        )
    );
}

// ---------------------------------------------------------------- held

#[test]
fn held_with_nothing_held() {
    let env = Env::new();
    assert_ok(&env.run(&["held"]), "Nothing is held.\n");
    assert_ok(
        &env.run(&["held", "--json"]),
        "{\"items\":[],\"unreadable\":0}\n",
    );
    assert!(!env.trash.exists());
}

#[test]
fn held_explain() {
    let env = Env::new();
    assert_ok(
        &env.run(&["held", "--explain"]),
        &format!(
            "cat -- {}/info/*.trashinfo\ndu -s --apparent-size --block-size=1 -- {}/files/*\n",
            env.trash.display(),
            env.trash.display()
        ),
    );
}

#[test]
fn held_lists_items_newest_first_and_skips_broken_ones() {
    let env = Env::new();
    let info = |path: &str, date: &str| format!("[Trash Info]\nPath={path}\nDeletionDate={date}\n");
    env.hold(
        "old.txt",
        &"o".repeat(1536),
        &info(&env.h("old.txt"), "2026-01-02T03:04:05"),
    );
    env.hold(
        "b",
        "bb",
        &info(&env.h("two%20words"), "2026-05-06T07:08:09"),
    );
    env.hold("a", "a", &info("/tmp/evil", "2026-05-06T07:08:09"));
    // An info file without its file, a file without its info, an unparsable one.
    fs::write(
        env.trash.join("info/orphan.trashinfo"),
        info(&env.h("orphan"), "2026-05-06T07:08:09"),
    )
    .unwrap();
    fs::write(env.trash.join("files/lonely"), "l").unwrap();
    env.hold("broken", "b", "not an info file\n");
    env.hold("nodate", "b", "[Trash Info]\nPath=/x\n");

    assert_ok(
        &env.run(&["held"]),
        &format!(
            "Held · 3 items\n  a  /tmp/evil  1 B  set down 2026-05-06 07:08\n  b  {}  2 B  set down 2026-05-06 07:08\n  old.txt  {}  1.5 KiB  set down 2026-01-02 03:04\n2 items in Held could not be read and are not shown.\n",
            env.h("two words"),
            env.h("old.txt"),
        ),
    );
    assert_ok(
        &env.run(&["held", "--json"]),
        &format!(
            "{{\"items\":[{{\"id\":\"a\",\"path\":\"/tmp/evil\",\"deleted_at\":\"2026-05-06T07:08:09\",\"size\":1}},{{\"id\":\"b\",\"path\":\"{}\",\"deleted_at\":\"2026-05-06T07:08:09\",\"size\":2}},{{\"id\":\"old.txt\",\"path\":\"{}\",\"deleted_at\":\"2026-01-02T03:04:05\",\"size\":1536}}],\"unreadable\":2}}\n",
            env.h("two words"),
            env.h("old.txt"),
        ),
    );
    assert_eq!(env.entries(), vec![]);
}

#[test]
fn held_one_item_and_size_units() {
    let env = Env::new();
    env.write("a.txt", "twelve bytes");
    env.run(&["set-down", &env.h("a.txt")]);
    let text = stdout(&env.run(&["held"]));
    let prefix = format!(
        "Held · 1 item\n  a.txt  {}  12 B  set down ",
        env.h("a.txt")
    );
    assert!(text.starts_with(&prefix), "{text}");
    assert_eq!(
        text.len(),
        prefix.len() + "YYYY-MM-DD HH:MM\n".len(),
        "{text}"
    );

    let env = Env::new();
    let info = format!(
        "[Trash Info]\nPath={}\nDeletionDate=2026-01-02T03:04:05\n",
        env.h("big")
    );
    env.hold("big", "", &info);
    let f = fs::File::options()
        .write(true)
        .open(env.trash.join("files/big"))
        .unwrap();
    f.set_len(3 * 1024 * 1024 + 512 * 1024).unwrap();
    let text = stdout(&env.run(&["held"]));
    assert!(text.contains("  3.5 MiB  "), "{text}");

    // The unit is chosen after rounding.
    f.set_len(1_048_575).unwrap();
    let text = stdout(&env.run(&["held"]));
    assert!(text.contains("  1.0 MiB  "), "{text}");
}

// ---------------------------------------------------------------- restore

#[test]
fn restore_moves_the_item_back() {
    let env = Env::new();
    env.write("a.txt", "twelve bytes");
    env.run(&["set-down", &env.h("a.txt")]);
    env.entries();

    let out = env.run(&["restore", "a.txt"]);
    let sentence = format!("Restored a.txt to {}.", env.h("a.txt"));
    assert_ok(&out, &format!("{sentence}\n"));
    assert_eq!(
        fs::read_to_string(env.home.join("a.txt")).unwrap(),
        "twelve bytes"
    );
    assert!(!env.trash.join("files/a.txt").exists());
    assert!(!env.trash.join("info/a.txt.trashinfo").exists());
    assert_eq!(
        env.entries(),
        vec![expected_entry(
            &sentence,
            "restore",
            "a.txt",
            &env.h("a.txt"),
            None
        )]
    );
    assert_ok(&env.run(&["held"]), "Nothing is held.\n");
}

#[test]
fn restore_json_and_to() {
    let env = Env::new();
    env.write("a.txt", "twelve bytes");
    env.run(&["set-down", &env.h("a.txt")]);
    env.entries();
    let to = env.h("a-restored.txt");
    let out = env.run(&["restore", "a.txt", "--to", &to, "--json"]);
    assert_ok(
        &out,
        &format!("{{\"action\":\"restore\",\"id\":\"a.txt\",\"path\":\"{to}\",\"created\":[]}}\n"),
    );
    assert_eq!(fs::read_to_string(&to).unwrap(), "twelve bytes");
    let entries = env.entries();
    assert_eq!(
        entries,
        vec![expected_entry(
            &format!("Restored a.txt to {to}."),
            "restore",
            "a.txt",
            &to,
            None
        )]
    );

    // --to relative to the current directory.
    env.write("b.txt", "b");
    env.run(&["set-down", &env.h("b.txt")]);
    let out = env.run(&["restore", "b.txt", "--to", "b2.txt"]);
    assert_ok(&out, &format!("Restored b.txt to {}.\n", env.h("b2.txt")));
}

#[test]
fn restore_recreates_missing_parents() {
    let env = Env::new();
    env.write("gone/deeper/d.txt", "d");
    env.run(&["set-down", &env.h("gone/deeper/d.txt")]);
    fs::remove_dir_all(env.home.join("gone")).unwrap();
    env.entries();

    let explain = env.run(&["restore", "d.txt", "--explain"]);
    assert_eq!(explain.status.code(), Some(0));
    let text = stdout(&explain);
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines[0], format!("mkdir -p -- {}", env.h("gone/deeper")));
    assert_eq!(
        lines[1],
        format!(
            "mv -n -- {} {}",
            env.t("files/d.txt"),
            env.h("gone/deeper/d.txt")
        )
    );
    assert_eq!(lines[2], format!("rm -- {}", env.t("info/d.txt.trashinfo")));
    assert_eq!(lines[3], "logger --journald <<'EOF'");
    assert!(!env.home.join("gone").exists());

    let out = env.run(&["restore", "d.txt"]);
    let sentence = format!("Restored d.txt to {}.", env.h("gone/deeper/d.txt"));
    assert_ok(
        &out,
        &format!(
            "Recreated {}, which no longer existed.\n{sentence}\n",
            env.h("gone/deeper")
        ),
    );
    assert_eq!(
        fs::read_to_string(env.home.join("gone/deeper/d.txt")).unwrap(),
        "d"
    );
    assert_eq!(
        env.entries(),
        vec![expected_entry(
            &sentence,
            "restore",
            "d.txt",
            &env.h("gone/deeper/d.txt"),
            None
        )]
    );

    env.run(&["set-down", &env.h("gone/deeper/d.txt")]);
    fs::remove_dir_all(env.home.join("gone")).unwrap();
    let out = env.run(&["restore", "d.txt", "--json"]);
    assert_ok(
        &out,
        &format!(
            "{{\"action\":\"restore\",\"id\":\"d.txt\",\"path\":\"{}\",\"created\":[\"{}\",\"{}\"]}}\n",
            env.h("gone/deeper/d.txt"),
            env.h("gone"),
            env.h("gone/deeper")
        ),
    );
}

#[test]
fn restore_never_overwrites() {
    let env = Env::new();
    env.write("a.txt", "twelve bytes");
    env.run(&["set-down", &env.h("a.txt")]);
    env.write("a.txt", "new");
    env.entries();
    let out = env.run(&["restore", "a.txt"]);
    assert_refused(
        &out,
        &format!(
            "{} already exists, so a.txt was not restored. To restore it elsewhere: lantea restore a.txt --to <path>\n",
            env.h("a.txt")
        ),
    );
    assert_eq!(fs::read_to_string(env.home.join("a.txt")).unwrap(), "new");
    assert!(env.trash.join("files/a.txt").exists());
    assert!(env.trash.join("info/a.txt.trashinfo").exists());
    assert_eq!(env.entries(), vec![]);
}

#[test]
fn restore_refuses_an_untrusted_path() {
    let env = Env::new();
    let evil = env.outside.join("evil");
    for (path, shown) in [
        (evil.display().to_string(), evil.display().to_string()),
        (env.h("../outside/evil"), env.h("../outside/evil")),
        (env.h("./evil"), env.h("./evil")),
        (env.h("outlink/evil"), env.h("outlink/evil")),
        (
            format!("{}/%2E%2E/outside/evil", env.home.display()),
            format!("{}/../outside/evil", env.home.display()),
        ),
    ] {
        let _ = fs::remove_file(env.home.join("outlink"));
        symlink(&env.outside, env.home.join("outlink")).unwrap();
        env.hold(
            "evil",
            "e",
            &format!("[Trash Info]\nPath={path}\nDeletionDate=2026-01-02T03:04:05\n"),
        );
        let out = env.run(&["restore", "evil"]);
        assert_refused(
            &out,
            &format!(
                "evil was set down from {shown}, which is outside the Tended Store, so it was not restored there. To restore it elsewhere: lantea restore evil --to <path>\n"
            ),
        );
        assert!(!evil.exists());
        assert!(env.trash.join("files/evil").exists());
    }

    let out = env.run(&["restore", "evil", "--to", evil.to_str().unwrap()]);
    assert_refused(
        &out,
        &format!(
            "{} is outside the Tended Store, so evil was not restored. Nothing was changed.\n",
            evil.display()
        ),
    );
    let out = env.run(&["restore", "evil", "--to", &env.h("outlink/new/evil")]);
    assert_refused(
        &out,
        &format!(
            "{} is outside the Tended Store, so evil was not restored. Nothing was changed.\n",
            env.h("outlink/new/evil")
        ),
    );
    assert!(!env.outside.join("new").exists());
    assert_eq!(env.entries(), vec![]);

    let to = env.h("evil.txt");
    let out = env.run(&["restore", "evil", "--to", &to]);
    assert_ok(&out, &format!("Restored evil to {to}.\n"));
    assert_eq!(fs::read_to_string(&to).unwrap(), "e");
    assert_eq!(env.entries().len(), 1);
}

#[test]
fn restore_refuses_unknown_and_invalid_ids() {
    let env = Env::new();
    env.write("a.txt", "twelve bytes");
    env.run(&["set-down", &env.h("a.txt")]);
    env.entries();
    for id in ["nope", "", ".", "..", "../a.txt", "files/a.txt", "a.txt/"] {
        let out = env.run(&["restore", id]);
        assert_refused(
            &out,
            &format!("Nothing is held as {id}. To see what is held: lantea held\n"),
        );
        let out = env.run(&["release", id, "--yes"]);
        assert_refused(
            &out,
            &format!("Nothing is held as {id}. To see what is held: lantea held\n"),
        );
    }
    assert!(env.trash.join("files/a.txt").exists());
    assert_eq!(env.entries(), vec![]);
}

#[test]
fn restore_explain_without_missing_parents() {
    let env = Env::new();
    env.write("a.txt", "twelve bytes");
    env.run(&["set-down", &env.h("a.txt")]);
    env.entries();
    let out = env.run(&["restore", "a.txt", "--explain"]);
    let sentence = format!("Restored a.txt to {}.", env.h("a.txt"));
    assert_ok(
        &out,
        &format!(
            "mv -n -- {} {}\nrm -- {}\nlogger --journald <<'EOF'\nMESSAGE={sentence}\nMESSAGE_ID={RESTORE_ID}\nPRIORITY=5\nSYSLOG_IDENTIFIER=lantea\nLANTEA_RECORD_VERSION=1\nLANTEA_ACTION=restore\nLANTEA_HELD_ID=a.txt\nLANTEA_PATH={}\nEOF\n",
            env.t("files/a.txt"),
            env.h("a.txt"),
            env.t("info/a.txt.trashinfo"),
            env.h("a.txt"),
        ),
    );
    assert!(env.trash.join("files/a.txt").exists());
    assert_eq!(env.entries(), vec![]);
}

// ---------------------------------------------------------------- release

#[test]
fn release_without_a_terminal_refuses() {
    let env = Env::new();
    env.write("b.txt", "four");
    env.run(&["set-down", &env.h("b.txt")]);
    env.entries();
    let refusal = "Release asks before deleting anything, but there is no terminal to ask on. To release without being asked: lantea release b.txt --yes\n";
    assert_refused(&env.run_with_stdin(&["release", "b.txt"], "y\n"), refusal);
    assert_refused(&env.run(&["release", "b.txt", "--json"]), refusal);
    assert!(env.trash.join("files/b.txt").exists());
    assert!(env.trash.join("info/b.txt.trashinfo").exists());
    assert_eq!(env.entries(), vec![]);
}

#[test]
fn release_yes_deletes_and_records() {
    let env = Env::new();
    env.write("b.txt", "four");
    env.run(&["set-down", &env.h("b.txt")]);
    env.entries();
    let out = env.run(&["release", "b.txt", "--yes"]);
    let sentence = format!(
        "Released b.txt. {} has been permanently deleted.",
        env.h("b.txt")
    );
    assert_ok(&out, &format!("{sentence}\n"));
    assert!(!env.trash.join("files/b.txt").exists());
    assert!(!env.trash.join("info/b.txt.trashinfo").exists());
    assert_eq!(
        env.entries(),
        vec![expected_entry(
            &sentence,
            "release",
            "b.txt",
            &env.h("b.txt"),
            Some(4)
        )]
    );

    env.write("b.txt", "four");
    env.run(&["set-down", &env.h("b.txt")]);
    let out = env.run(&["--yes", "--json", "release", "b.txt"]);
    assert_ok(
        &out,
        &format!(
            "{{\"action\":\"release\",\"id\":\"b.txt\",\"path\":\"{}\",\"size\":4}}\n",
            env.h("b.txt")
        ),
    );
}

#[test]
fn release_a_directory_never_follows_symlinks() {
    let env = Env::new();
    fs::write(env.outside.join("keep"), "keep").unwrap();
    env.write("dir/one", "abc");
    env.write("dir/sub/two", "defgh");
    symlink(&env.outside, env.home.join("dir/sub/out")).unwrap();
    env.run(&["set-down", &env.h("dir")]);
    let out = env.run(&["release", "dir", "--yes"]);
    assert_ok(
        &out,
        &format!(
            "Released dir. {} has been permanently deleted.\n",
            env.h("dir")
        ),
    );
    assert!(!env.trash.join("files/dir").exists());
    assert_eq!(
        fs::read_to_string(env.outside.join("keep")).unwrap(),
        "keep"
    );
}

#[test]
fn release_explain_asks_nothing_and_changes_nothing() {
    let env = Env::new();
    env.write("b.txt", "four");
    env.run(&["set-down", &env.h("b.txt")]);
    env.entries();
    let out = env.run(&["release", "b.txt", "--explain"]);
    let sentence = format!(
        "Released b.txt. {} has been permanently deleted.",
        env.h("b.txt")
    );
    assert_ok(
        &out,
        &format!(
            "logger --journald <<'EOF'\nMESSAGE={sentence}\nMESSAGE_ID={RELEASE_ID}\nPRIORITY=5\nSYSLOG_IDENTIFIER=lantea\nLANTEA_RECORD_VERSION=1\nLANTEA_ACTION=release\nLANTEA_HELD_ID=b.txt\nLANTEA_PATH={}\nLANTEA_SIZE=4\nEOF\nrm -rf -- {} {}\n",
            env.h("b.txt"),
            env.t("files/b.txt"),
            env.t("info/b.txt.trashinfo"),
        ),
    );
    assert!(env.trash.join("files/b.txt").exists());
    assert_eq!(env.entries(), vec![]);
}

// ---------------------------------------------------------------- the Record

#[test]
fn a_record_failure_is_reported_but_the_action_stands() {
    let env = Env::new();
    env.write("a.txt", "twelve bytes");
    let out = env
        .command(&["set-down", &env.h("a.txt")])
        .env("LANTEA_JOURNAL_SOCKET", env.outside.join("no-such-socket"))
        .stdin(Stdio::null())
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(1));
    let err = stderr(&out);
    assert!(
        err.starts_with("The set-down was done, but the Record could not be written: "),
        "{err}"
    );
    assert!(err.ends_with(".\n"), "{err}");
    assert!(!env.home.join("a.txt").exists());
    assert!(env.trash.join("files/a.txt").exists());

    let out = env
        .command(&["restore", "a.txt"])
        .env("LANTEA_JOURNAL_SOCKET", env.outside.join("no-such-socket"))
        .stdin(Stdio::null())
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(1));
    assert!(
        stderr(&out).starts_with("The restore was done, but the Record could not be written: ")
    );
    assert!(env.home.join("a.txt").exists());
}

#[test]
fn a_release_the_record_cannot_hold_deletes_nothing() {
    let env = Env::new();
    env.write("a.txt", "twelve bytes");
    env.run(&["set-down", &env.h("a.txt")]);
    env.entries();
    let out = env
        .command(&["release", "a.txt", "--yes"])
        .env("LANTEA_JOURNAL_SOCKET", env.outside.join("no-such-socket"))
        .stdin(Stdio::null())
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(1));
    assert_eq!(stdout(&out), "");
    assert_eq!(
        stderr(&out),
        "The Record could not be written, so a.txt was not released: No such file or directory (os error 2). Nothing was deleted.\n"
    );
    assert!(env.trash.join("files/a.txt").exists());
    assert!(env.trash.join("info/a.txt.trashinfo").exists());
}

/// Runs `f` with `dir` made read-only, then makes it writable again so the
/// temporary directory can be cleaned up.
fn with_read_only<T>(dir: &Path, f: impl FnOnce() -> T) -> T {
    fs::set_permissions(dir, fs::Permissions::from_mode(0o500)).unwrap();
    let result = f();
    fs::set_permissions(dir, fs::Permissions::from_mode(0o700)).unwrap();
    result
}

#[test]
fn a_partial_release_writes_release_incomplete() {
    let env = Env::new();
    env.write("dir/locked/inner", "in");
    env.run(&["set-down", &env.h("dir")]);
    let size = du_apparent(&env.trash.join("files/dir"));
    env.entries();
    let locked = env.trash.join("files/dir/locked");
    let out = with_read_only(&locked, || env.run(&["release", "dir", "--yes"]));
    assert_refused(&out, &format!("{}\n", partial_eacces(&env)));
    assert_eq!(
        env.entries(),
        vec![
            expected_entry(
                &format!(
                    "Released dir. {} has been permanently deleted.",
                    env.h("dir")
                ),
                "release",
                "dir",
                &env.h("dir"),
                Some(size)
            ),
            expected_entry(
                &partial_eacces(&env),
                "release-incomplete",
                "dir",
                &env.h("dir"),
                Some(size)
            ),
        ]
    );
    assert!(env.trash.join("files/dir/locked/inner").exists());
}

#[test]
fn an_info_file_that_cannot_be_removed_is_reported() {
    let env = Env::new();
    env.write("a.txt", "twelve bytes");
    env.write("b.txt", "four");
    env.run(&["set-down", &env.h("a.txt")]);
    env.run(&["set-down", &env.h("b.txt")]);
    env.entries();
    let info = env.trash.join("info");

    let out = with_read_only(&info, || env.run(&["restore", "a.txt"]));
    assert_eq!(out.status.code(), Some(1));
    assert_eq!(
        stdout(&out),
        format!("Restored a.txt to {}.\n", env.h("a.txt"))
    );
    assert_eq!(
        stderr(&out),
        format!(
            "a.txt was restored to {}, but {} could not be removed: Permission denied (os error 13). It may still be listed by lantea held.\n",
            env.h("a.txt"),
            env.t("info/a.txt.trashinfo")
        )
    );
    assert_eq!(
        fs::read_to_string(env.home.join("a.txt")).unwrap(),
        "twelve bytes"
    );

    let out = with_read_only(&info, || env.run(&["release", "b.txt", "--yes"]));
    assert_eq!(out.status.code(), Some(1));
    assert_eq!(
        stderr(&out),
        format!(
            "b.txt was released, but {} could not be removed: Permission denied (os error 13). It may still be listed by lantea held. Check that you own {}.\n",
            env.t("info/b.txt.trashinfo"),
            env.t("info")
        )
    );
    assert!(!env.trash.join("files/b.txt").exists());
    let entries = env.entries();
    let actions: Vec<&[u8]> = entries.iter().map(|e| &e["LANTEA_ACTION"][..]).collect();
    assert_eq!(actions, [&b"restore"[..], &b"release"[..]]);
}

#[test]
fn a_value_with_a_newline_arrives_intact() {
    let env = Env::new();
    env.write("line\nbreak", "nl");
    let out = env.run(&["set-down", &env.h("line\nbreak")]);
    assert_eq!(out.status.code(), Some(0), "{}", stderr(&out));
    let entries = env.entries();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0]["LANTEA_HELD_ID"], b"line\nbreak");
    assert_eq!(
        entries[0]["LANTEA_PATH"],
        env.h("line\nbreak").as_bytes().to_vec()
    );
    let message = String::from_utf8(entries[0]["MESSAGE"].clone()).unwrap();
    assert!(message.contains("held as line\nbreak;"), "{message}");
    // The file name is percent-encoded in the info file.
    read_info(&env, "line\nbreak", &env.h("line%0Abreak"));
}

#[test]
fn held_and_refusals_write_no_record() {
    let env = Env::new();
    env.run(&["held"]);
    env.run(&["held", "--json"]);
    env.run(&["restore", "nope"]);
    env.run(&["release", "nope", "--yes"]);
    env.run(&["set-down", &env.h("missing")]);
    assert_eq!(env.entries(), vec![]);
}

// ---------------------------------------------------------------- review fixes

#[test]
fn control_characters_are_escaped_on_the_terminal_only() {
    let env = Env::new();
    let name = "line\nbreak\u{1b}\u{85}\\x";
    let shown = "line\\x0abreak\\x1b\\x85\\x5cx";
    env.write(name, "nl");

    let out = env.run(&["set-down", &env.h(name)]);
    assert_ok(
        &out,
        &format!(
            "Set down {}. It is held as {shown}; to bring it back: lantea restore '{shown}'\n",
            env.h(shown)
        ),
    );
    // The Record keeps the raw text.
    let entries = env.entries();
    assert_eq!(entries[0]["LANTEA_HELD_ID"], name.as_bytes());
    assert_eq!(
        entries[0]["MESSAGE"],
        format!(
            "Set down {}. It is held as {name}; to bring it back: lantea restore '{name}'",
            env.h(name)
        )
        .as_bytes()
    );

    let text = stdout(&env.run(&["held"]));
    assert!(
        text.starts_with(&format!(
            "Held · 1 item\n  {shown}  {}  2 B  ",
            env.h(shown)
        )),
        "{text}"
    );
    assert_eq!(text.lines().count(), 2, "{text}");
    // --json keeps the raw text, JSON-escaped.
    let json = stdout(&env.run(&["held", "--json"]));
    assert!(
        json.contains("\"id\":\"line\\nbreak\\u001b\u{85}\\\\x\""),
        "{json}"
    );

    // --explain, including the logger block, stays one line per field.
    let explain = stdout(&env.run(&["release", name, "--explain"]));
    assert!(
        explain.contains(&format!("\nLANTEA_HELD_ID={shown}\n")),
        "{explain}"
    );
    assert!(
        explain.contains(&format!("\nLANTEA_PATH={}\n", env.h(shown))),
        "{explain}"
    );
    assert_eq!(explain.lines().count(), 12, "{explain}");

    // Refusals and messages.
    let out = env.run(&["restore", "nope\u{7}"]);
    assert_refused(
        &out,
        "Nothing is held as nope\\x07. To see what is held: lantea held\n",
    );
    let out = env.run(&["release", name]);
    assert_refused(
        &out,
        &format!(
            "Release asks before deleting anything, but there is no terminal to ask on. To release without being asked: lantea release '{shown}' --yes\n"
        ),
    );
    let out = env.run(&["restore", name]);
    assert_ok(&out, &format!("Restored {shown} to {}.\n", env.h(shown)));
}

#[test]
fn unusable_info_files_are_skipped() {
    let env = Env::new();
    let info =
        |path: &str| format!("[Trash Info]\nPath={path}\nDeletionDate=2026-01-02T03:04:05\n");
    env.hold("ok", "o", &info(&env.h("ok")));
    // A FIFO must not make held wait.
    fs::write(env.trash.join("files/fifo"), "f").unwrap();
    rustix::fs::mknodat(
        rustix::fs::CWD,
        env.trash.join("info/fifo.trashinfo"),
        rustix::fs::FileType::Fifo,
        rustix::fs::Mode::RUSR | rustix::fs::Mode::WUSR,
        0,
    )
    .unwrap();
    // A symlink to a good info file.
    fs::write(env.outside.join("good.trashinfo"), info(&env.h("linked"))).unwrap();
    fs::write(env.trash.join("files/linked"), "l").unwrap();
    symlink(
        env.outside.join("good.trashinfo"),
        env.trash.join("info/linked.trashinfo"),
    )
    .unwrap();
    // Larger than 64 KiB.
    let mut big = info(&env.h("big"));
    big.push_str(&format!("# {}\n", "x".repeat(65536)));
    env.hold("big", "b", &big);
    // A relative Path=.
    env.hold("relative", "r", &info("relative/evil"));

    assert_ok(
        &env.run(&["held", "--json"]),
        &format!(
            "{{\"items\":[{{\"id\":\"ok\",\"path\":\"{}\",\"deleted_at\":\"2026-01-02T03:04:05\",\"size\":1}}],\"unreadable\":4}}\n",
            env.h("ok")
        ),
    );
    for id in ["fifo", "linked", "big", "relative"] {
        assert_refused(
            &env.run(&["restore", id]),
            &format!("Nothing is held as {id}. To see what is held: lantea held\n"),
        );
        assert_refused(
            &env.run(&["release", id, "--yes"]),
            &format!("Nothing is held as {id}. To see what is held: lantea held\n"),
        );
    }
    assert_eq!(env.entries(), vec![]);
}

#[test]
fn an_item_whose_size_is_unknown_is_still_held() {
    let env = Env::new();
    env.write("dir/closed/inner", "in");
    env.write("dir2/closed/inner", "in");
    env.run(&["set-down", &env.h("dir")]);
    env.run(&["set-down", &env.h("dir2")]);
    env.entries();
    let closed = env.trash.join("files/dir/closed");
    let closed2 = env.trash.join("files/dir2/closed");
    fs::set_permissions(&closed, fs::Permissions::from_mode(0o300)).unwrap();
    fs::set_permissions(&closed2, fs::Permissions::from_mode(0o300)).unwrap();

    let text = stdout(&env.run(&["held"]));
    assert!(
        text.contains(&format!("  dir  {}  size unknown  set down ", env.h("dir"))),
        "{text}"
    );
    let json = stdout(&env.run(&["held", "--json"]));
    assert!(
        json.contains(&format!("{{\"id\":\"dir\",\"path\":\"{}\",", env.h("dir"))),
        "{json}"
    );
    assert!(json.contains("\"size\":null"), "{json}");

    // Restore works as usual.
    assert_ok(
        &env.run(&["restore", "dir"]),
        &format!("Restored dir to {}.\n", env.h("dir")),
    );
    fs::set_permissions(
        env.home.join("dir/closed"),
        fs::Permissions::from_mode(0o700),
    )
    .unwrap();

    // Release still tries, and its entry has no LANTEA_SIZE.
    env.entries();
    let out = env.run(&["release", "dir2", "--yes"]);
    fs::set_permissions(&closed2, fs::Permissions::from_mode(0o700)).unwrap();
    assert_eq!(out.status.code(), Some(1), "{}", stderr(&out));
    let entries = env.entries();
    assert_eq!(
        entries[0],
        expected_entry(
            &format!(
                "Released dir2. {} has been permanently deleted.",
                env.h("dir2")
            ),
            "release",
            "dir2",
            &env.h("dir2"),
            None
        )
    );
    assert_eq!(entries[1]["LANTEA_ACTION"], b"release-incomplete");
    assert!(!entries[1].contains_key("LANTEA_SIZE"));
}

/// The mount ID of a path, not following a final symlink.
fn mount_id(path: &Path) -> u64 {
    rustix::fs::statx(
        rustix::fs::CWD,
        path,
        rustix::fs::AtFlags::SYMLINK_NOFOLLOW,
        rustix::fs::StatxFlags::MNT_ID,
    )
    .unwrap()
    .stx_mnt_id
}

#[test]
fn set_down_refuses_an_item_on_another_mount_than_held() {
    // Held lives on /dev/shm, the home in the temporary directory: two mounts.
    let env = Env::new();
    let shm = tempfile::tempdir_in("/dev/shm").expect("a directory in /dev/shm");
    assert_ne!(
        mount_id(shm.path()),
        mount_id(&env.home),
        "this test needs /dev/shm and the temporary directory on different mounts"
    );
    env.write("a.txt", "twelve bytes");
    let out = env
        .command(&["set-down", &env.h("a.txt")])
        .env("XDG_DATA_HOME", shm.path())
        .stdin(Stdio::null())
        .output()
        .unwrap();
    assert_refused(
        &out,
        &format!(
            "{} is on a separate mount from your Held items, so it cannot be set down. Nothing was changed.\n",
            env.h("a.txt")
        ),
    );
    assert!(env.home.join("a.txt").exists());
    assert_eq!(env.entries(), vec![]);
}

#[test]
fn held_says_how_many_items_could_not_be_read() {
    let env = Env::new();
    env.hold("bad", "b", "not an info file\n");
    assert_ok(
        &env.run(&["held"]),
        "Nothing is held.\n1 item in Held could not be read and is not shown.\n",
    );
    assert_ok(
        &env.run(&["held", "--json"]),
        "{\"items\":[],\"unreadable\":1}\n",
    );
    env.hold(
        "rel",
        "r",
        "[Trash Info]\nPath=rel\nDeletionDate=2026-01-02T03:04:05\n",
    );
    assert_ok(
        &env.run(&["held"]),
        "Nothing is held.\n2 items in Held could not be read and are not shown.\n",
    );
}

#[test]
fn a_single_quote_is_quoted_without_a_backslash() {
    let env = Env::new();
    env.write("it's", "q");
    let out = env.run(&["set-down", &env.h("it's")]);
    assert_ok(
        &out,
        &format!(
            "Set down {}. It is held as it's; to bring it back: lantea restore 'it'\"'\"'s'\n",
            env.h("it's")
        ),
    );
}

// ---------------------------------------------------------------- second review fixes

/// What one command printed inside a namespace.
struct Ran {
    code: i32,
    stdout: String,
    stderr: String,
}

/// Runs `script` as root of a new user and mount namespace
/// (`unshare -Urm`), so it can mount without real root. Inside, `run NAME
/// ARGS…` runs lantea and keeps its output under NAME. `$H` is the home, `$T`
/// the Trash. Fails loudly when user namespaces are not available.
fn in_namespace(env: &Env, script: &str) -> BTreeMap<String, Ran> {
    let results = env.outside.join("results");
    let _ = fs::remove_dir_all(&results);
    fs::create_dir(&results).unwrap();
    let prelude = r#"
run() {
    n=$1; shift
    set +e
    "$LANTEA" "$@" >"$RES/$n.out" 2>"$RES/$n.err" </dev/null
    echo $? >"$RES/$n.code"
    set -e
}
"#;
    require_user_namespaces();
    let out = Command::new("unshare")
        .args(["-Urm", "sh", "-euc"])
        .arg(format!("{prelude}\n{script}"))
        .current_dir(&env.home)
        .env("HOME", &env.home)
        .env("XDG_DATA_HOME", env.home.join(".local/share"))
        .env("LANTEA_JOURNAL_SOCKET", &env.sock_path)
        .env("LANTEA", env!("CARGO_BIN_EXE_lantea"))
        .env("H", &env.home)
        .env("T", &env.trash)
        .env("RES", &results)
        .stdin(Stdio::null())
        .output()
        .expect("unshare runs; this test needs util-linux and user namespaces");
    assert!(
        out.status.success(),
        "the script in the user namespace failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let mut ran = BTreeMap::new();
    for entry in fs::read_dir(&results).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().is_some_and(|e| e == "code") {
            let name = path.file_stem().unwrap().to_str().unwrap().to_string();
            let read =
                |ext: &str| fs::read_to_string(results.join(format!("{name}.{ext}"))).unwrap();
            ran.insert(
                name.clone(),
                Ran {
                    code: read("code").trim().parse().unwrap(),
                    stdout: read("out"),
                    stderr: read("err"),
                },
            );
        }
    }
    ran
}

#[track_caller]
fn assert_ran(ran: &BTreeMap<String, Ran>, name: &str, code: i32, stdout: &str, stderr: &str) {
    let r = ran
        .get(name)
        .unwrap_or_else(|| panic!("{name} did not run"));
    assert_eq!(
        (r.code, r.stdout.as_str(), r.stderr.as_str()),
        (code, stdout, stderr),
        "{name}"
    );
}

const INFO_DATE: &str = "DeletionDate=2026-01-02T03:04:05";

/// One table: each command against each mount situation.
///
/// Every combination runs here in a user namespace, so none needs real root.
/// VM test step 12 repeats the item-that-is-a-mount (tmpfs) and bind-mount
/// cases as real root, and the mount-below-the-item case.
#[test]
fn mount_situations_table() {
    let separate_set_down = |p: &str| {
        format!(
            "{p} is on a separate mount from your Held items, so it cannot be set down. Nothing was changed.\n"
        )
    };
    let separate_restore = |p: &str, id: &str| {
        format!(
            "{p} is on a separate mount from your Held items, so {id} was not restored. Nothing was changed.\n"
        )
    };
    let cannot_release = |id: &str| {
        format!("{id} is a separate mount, so it cannot be released. Nothing was changed.\n")
    };

    // The item itself is a mount (tmpfs), and a bind mount on the same file
    // system: both are refused alike.
    for (situation, mount) in [
        ("tmpfs", "mount -t tmpfs tmpfs"),
        ("bind", "mount --bind \"$H/bindsrc\""),
    ] {
        let env = Env::new();
        let script = format!(
            r#"
mkdir -p "$H/bindsrc" "$H/item" "$T/files/held" "$T/info"
{mount} "$H/item"
echo x > "$H/item/x"
{mount} "$T/files/held"
printf '[Trash Info]\nPath=%s\n{INFO_DATE}\n' "$H/back" > "$T/info/held.trashinfo"
run set-down set-down "$H/item"
run held held
run held-json held --json
run restore restore held
run release release held --yes
test -e "$T/info/held.trashinfo"
"#
        );
        let ran = in_namespace(&env, &script);
        assert_ran(&ran, "set-down", 1, "", &separate_set_down(&env.h("item")));
        assert_ran(
            &ran,
            "held",
            0,
            &format!(
                "Held · 1 item\n  held  {}  size unknown  set down 2026-01-02 03:04\n",
                env.h("back")
            ),
            "",
        );
        assert_ran(
            &ran,
            "held-json",
            0,
            &format!(
                "{{\"items\":[{{\"id\":\"held\",\"path\":\"{}\",\"deleted_at\":\"2026-01-02T03:04:05\",\"size\":null}}],\"unreadable\":0}}\n",
                env.h("back")
            ),
            "",
        );
        assert_ran(
            &ran,
            "restore",
            1,
            "",
            &separate_restore(&env.h("back"), "held"),
        );
        assert_ran(&ran, "release", 1, "", &cannot_release("held"));
        assert!(!env.home.join("back").exists(), "{situation}");
        assert_eq!(env.entries(), vec![], "{situation}");
    }

    // A mount below the item, tmpfs and bind.
    for (situation, mount) in [
        ("tmpfs", "mount -t tmpfs tmpfs"),
        ("bind", "mount --bind \"$H/bindsrc\""),
    ] {
        let env = Env::new();
        let script = format!(
            r#"
mkdir -p "$H/bindsrc" "$H/item/inner" "$T/files/held/sub" "$T/info"
{mount} "$H/item/inner"
printf 'm' > "$T/files/held/top.txt"
{mount} "$T/files/held/sub"
echo keep > "$T/files/held/sub/keep.txt"
printf '[Trash Info]\nPath=%s\n{INFO_DATE}\n' "$H/back" > "$T/info/held.trashinfo"
stat -c %s "$T/files/held" > "$RES/dirsize"
run set-down set-down "$H/item"
run held held
run held-json held --json
run release release held --yes
test -e "$T/files/held/sub/keep.txt"
test ! -e "$T/files/held/top.txt"
test -e "$T/info/held.trashinfo"
run restore restore held
test -e "$H/back/sub/keep.txt"
"#
        );
        let ran = in_namespace(&env, &script);
        let dir_size: u64 = fs::read_to_string(env.outside.join("results/dirsize"))
            .unwrap()
            .trim()
            .parse()
            .unwrap();
        // The directory and top.txt; the mount below is neither counted nor entered.
        let size = dir_size + 1;
        assert_ran(
            &ran,
            "set-down",
            1,
            "",
            &format!(
                "{} contains {}, which is a separate mount, so it cannot be set down. Nothing was changed.\n",
                env.h("item"),
                env.h("item/inner")
            ),
        );
        assert_ran(
            &ran,
            "held",
            0,
            &format!(
                "Held · 1 item\n  held  {}  {size} B  set down 2026-01-02 03:04\n",
                env.h("back")
            )
            .replace(&format!("{size} B"), &human(size)),
            "",
        );
        assert_ran(
            &ran,
            "held-json",
            0,
            &format!(
                "{{\"items\":[{{\"id\":\"held\",\"path\":\"{}\",\"deleted_at\":\"2026-01-02T03:04:05\",\"size\":{size}}}],\"unreadable\":0}}\n",
                env.h("back")
            ),
            "",
        );
        let incomplete = format!(
            "held could not be released completely: {} is a separate mount, so it was left in place. The release is in the Record, marked as incomplete. Some of it may already be deleted. To see what is held: lantea held",
            env.t("files/held/sub")
        );
        assert_ran(&ran, "release", 1, "", &format!("{incomplete}\n"));
        // Restore moves the item, with the mount below it, back home.
        assert_ran(
            &ran,
            "restore",
            0,
            &format!("Restored held to {}.\n", env.h("back")),
            "",
        );
        let entries = env.entries();
        let release = format!(
            "Released held. {} has been permanently deleted.",
            env.h("back")
        );
        assert_eq!(
            entries,
            vec![
                expected_entry(&release, "release", "held", &env.h("back"), Some(size)),
                expected_entry(
                    &incomplete,
                    "release-incomplete",
                    "held",
                    &env.h("back"),
                    Some(size)
                ),
                expected_entry(
                    &format!("Restored held to {}.", env.h("back")),
                    "restore",
                    "held",
                    &env.h("back"),
                    None
                ),
            ],
            "{situation}"
        );
    }
}

/// `human_size` as lantea prints it, for the sizes this file uses.
fn human(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{bytes} B")
    } else {
        format!("{:.1} KiB", bytes as f64 / 1024.0)
    }
}

#[test]
fn held_owned_by_another_user_is_refused() {
    // /nix/store is not ours; bound over ~/.local inside a user namespace it
    // shows as owned by another uid.
    let env = Env::new();
    let store_owner = fs::metadata("/nix/store").unwrap().uid();
    let our_uid = fs::metadata(&env.home).unwrap().uid();
    assert_ne!(
        store_owner, our_uid,
        "this test needs /nix/store owned by another user"
    );
    let script = r#"
mkdir -p "$H/.local"
mount --rbind /nix/store "$H/.local"
run held held
run held-json held --json
run set-down set-down "$H/a.txt"
run restore restore a.txt
run release release a.txt --yes
"#;
    env.write("a.txt", "twelve bytes");
    let ran = in_namespace(&env, script);
    let line = format!(
        "Held at {} belongs to another user, so lantea will not act on it. Run lantea as that user instead. Nothing was changed.\n",
        env.trash.display()
    );
    for name in ["held", "held-json", "set-down", "restore", "release"] {
        assert_ran(&ran, name, 1, "", &line);
    }
    assert!(env.home.join("a.txt").exists());
    assert_eq!(env.entries(), vec![]);
}

#[test]
fn restore_removes_the_directories_it_created_when_it_fails() {
    let env = Env::new();
    env.write("gone/deeper/d.txt", "d");
    env.run(&["set-down", &env.h("gone/deeper/d.txt")]);
    fs::remove_dir_all(env.home.join("gone")).unwrap();
    env.entries();
    // With umask 222 the created `gone` is read-only, so creating `deeper`
    // inside it fails after `gone` was created.
    let out = Command::new("sh")
        .args([
            "-c",
            "umask 222; exec \"$0\" restore d.txt",
            env!("CARGO_BIN_EXE_lantea"),
        ])
        .current_dir(&env.home)
        .env("HOME", &env.home)
        .env("XDG_DATA_HOME", env.home.join(".local/share"))
        .env("LANTEA_JOURNAL_SOCKET", &env.sock_path)
        .stdin(Stdio::null())
        .output()
        .unwrap();
    assert_refused(
        &out,
        &format!(
            "d.txt could not be restored to {}: Permission denied (os error 13). Nothing was changed.\n",
            env.h("gone/deeper/d.txt")
        ),
    );
    assert!(!env.home.join("gone").exists());
    assert!(env.trash.join("files/d.txt").exists());
    assert_eq!(env.entries(), vec![]);
}

#[test]
fn restore_does_not_report_a_directory_it_did_not_create() {
    // `gone` exists again by the time restore looks; only the missing
    // `deeper` is reported.
    let env = Env::new();
    env.write("gone/deeper/d.txt", "d");
    env.run(&["set-down", &env.h("gone/deeper/d.txt")]);
    fs::remove_dir(env.home.join("gone/deeper")).unwrap();
    let out = env.run(&["restore", "d.txt", "--json"]);
    assert_ok(
        &out,
        &format!(
            "{{\"action\":\"restore\",\"id\":\"d.txt\",\"path\":\"{}\",\"created\":[\"{}\"]}}\n",
            env.h("gone/deeper/d.txt"),
            env.h("gone/deeper")
        ),
    );
}

#[test]
fn a_failed_second_entry_is_reported_too() {
    let env = Env::new();
    env.write("dir/locked/inner", "in");
    env.run(&["set-down", &env.h("dir")]);
    let locked = env.trash.join("files/dir/locked");

    // A socket whose queue has room for exactly one more datagram: lantea's
    // release entry fits, the release-incomplete entry then waits until the
    // socket is closed, and fails.
    let path = env.outside.join("one.socket");
    let sock = UnixDatagram::bind(&path).unwrap();
    // Each datagram comes from a fresh socket, so only the receiving queue
    // can be full, never the sender's buffer.
    let send_fresh = |data: &[u8]| {
        let s = UnixDatagram::unbound().unwrap();
        s.set_nonblocking(true).unwrap();
        s.send_to(data, &path)
    };
    while send_fresh(b"FILL=1\n").is_ok() {}
    let mut buf = [0u8; 64];
    sock.recv(&mut buf).unwrap();

    fs::set_permissions(&locked, fs::Permissions::from_mode(0o500)).unwrap();
    let child = env
        .command(&["release", "dir", "--yes"])
        .env("LANTEA_JOURNAL_SOCKET", &path)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    // Wait until the release entry is in the queue: a probe that does not fit
    // shows it. A probe that does fit is taken out again.
    loop {
        match send_fresh(b"PROBE=1\n") {
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
            Err(e) => panic!("probe: {e}"),
            Ok(_) => {
                sock.recv(&mut buf).unwrap();
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
        }
    }
    drop(sock);
    let out = child.wait_with_output().unwrap();
    fs::set_permissions(&locked, fs::Permissions::from_mode(0o700)).unwrap();
    assert_refused(
        &out,
        &format!(
            "{}\nThe Record could not be written: Connection refused (os error 111).\n",
            partial_eacces(&env)
        ),
    );
}

/// The message of a release that stopped at the read-only `dir/locked`.
fn partial_eacces(env: &Env) -> String {
    format!(
        "dir could not be released completely: Permission denied (os error 13). The release is in the Record, marked as incomplete. Some of it may already be deleted. To see what is held: lantea held. Check that you own {}.",
        env.t("files/dir/locked")
    )
}

// ---------------------------------------------------------------- third review fixes

#[test]
fn a_home_owned_by_another_user_is_refused() {
    let env = Env::new();
    let store_owner = fs::metadata("/nix/store").unwrap().uid();
    let our_uid = fs::metadata(&env.home).unwrap().uid();
    assert_ne!(
        store_owner, our_uid,
        "this test needs /nix/store owned by another user"
    );
    env.write("a.txt", "twelve bytes");
    let line = "Your home directory /nix/store belongs to another user, so lantea will not act on it. Run lantea as that user instead. Nothing was changed.\n";
    for args in [
        vec!["held"],
        vec!["set-down", &env.h("a.txt")],
        vec!["restore", "a.txt"],
        vec!["release", "a.txt", "--yes"],
    ] {
        let out = env
            .command(&args)
            .env("HOME", "/nix/store")
            .env_remove("XDG_DATA_HOME")
            .stdin(Stdio::null())
            .output()
            .unwrap();
        assert_refused(&out, line);
    }
    assert!(!Path::new("/nix/store/.local").exists());
    assert!(env.home.join("a.txt").exists());
    assert_eq!(env.entries(), vec![]);
}

#[test]
fn a_missing_home_is_refused_and_not_created() {
    let env = Env::new();
    let missing = env.outside.join("nohome");
    let out = env
        .command(&["set-down", &env.h("a.txt")])
        .env("HOME", &missing)
        .env_remove("XDG_DATA_HOME")
        .stdin(Stdio::null())
        .output()
        .unwrap();
    assert_refused(
        &out,
        &format!(
            "Your home directory {} does not exist, so nothing was changed.\n",
            missing.display()
        ),
    );
    assert!(!missing.exists());
}

#[test]
fn a_relative_home_is_refused() {
    let env = Env::new();
    let out = env
        .command(&["held"])
        .env("HOME", "some/where")
        .stdin(Stdio::null())
        .output()
        .unwrap();
    assert_refused(
        &out,
        "HOME is set to a relative path (some/where). Held needs an absolute path, so nothing was changed.\n",
    );
}

#[test]
fn a_symlink_on_the_way_to_held_is_refused() {
    for link in [".local", ".local/share", ".local/share/Trash"] {
        let env = Env::new();
        let target = env.outside.join("real");
        fs::create_dir_all(&target).unwrap();
        let at = env.home.join(link);
        fs::create_dir_all(at.parent().unwrap()).unwrap();
        symlink(&target, &at).unwrap();
        env.write("a.txt", "twelve bytes");
        for args in [vec!["held"], vec!["set-down", &env.h("a.txt")]] {
            assert_refused(
                &env.run(&args),
                &format!(
                    "{} is a symbolic link, so lantea will not use it for your Held items. Nothing was changed. Replace the link with a folder to use lantea.\n",
                    at.display()
                ),
            );
        }
        assert!(env.home.join("a.txt").exists());
        assert_eq!(fs::read_dir(&target).unwrap().count(), 0, "{link}");
    }
}

#[test]
fn restore_never_writes_into_held() {
    let env = Env::new();
    env.write("a.txt", "twelve bytes");
    env.run(&["set-down", &env.h("a.txt")]);
    env.entries();
    let inside = |p: &str| {
        format!(
            "{p} is inside your Held items, so a.txt was not restored there. Nothing was changed. To restore it elsewhere: lantea restore a.txt --to <path>\n"
        )
    };
    for to in [
        env.t("files/x"),
        env.t("info/x"),
        env.t("new/deeper/x"),
        env.t("files/a.txt.copy"),
    ] {
        assert_refused(&env.run(&["restore", "a.txt", "--to", &to]), &inside(&to));
    }
    // The Trash itself, which already exists.
    assert_refused(
        &env.run(&["restore", "a.txt", "--to", &env.trash.display().to_string()]),
        &inside(&env.trash.display().to_string()),
    );
    // A Path= that leads into Held.
    env.hold(
        "b.txt",
        "b",
        &format!("[Trash Info]\nPath={}\n{INFO_DATE}\n", env.t("files/b2")),
    );
    assert_refused(
        &env.run(&["restore", "b.txt"]),
        &format!(
            "{} is inside your Held items, so b.txt was not restored there. Nothing was changed. To restore it elsewhere: lantea restore b.txt --to <path>\n",
            env.t("files/b2")
        ),
    );
    assert!(!env.trash.join("new").exists());
    assert!(env.trash.join("files/a.txt").exists());
    assert_eq!(env.entries(), vec![]);
}

/// Fails with the cause when `unshare -Urm` is denied. These tests are never
/// skipped (decision 0019).
fn require_user_namespaces() {
    let probe = Command::new("unshare")
        .args(["-Urm", "true"])
        .stdin(Stdio::null())
        .output();
    let denied = match probe {
        Ok(out) if out.status.success() => return,
        Ok(out) => String::from_utf8_lossy(&out.stderr).trim().to_string(),
        Err(e) => format!("unshare could not be run ({e}); util-linux is needed"),
    };
    let read = |p: &str| {
        fs::read_to_string(p)
            .map(|v| v.trim().to_string())
            .unwrap_or_else(|_| "not present".to_string())
    };
    panic!(
        "This test needs unprivileged user namespaces (unshare -Urm), and they were denied: {denied}.\n\
         kernel.apparmor_restrict_unprivileged_userns = {} (1 means AppArmor blocks them; set it to 0, as CI does)\n\
         user.max_user_namespaces = {} (0 disables them)\n\
         kernel.unprivileged_userns_clone = {} (0 disables them on some kernels)",
        read("/proc/sys/kernel/apparmor_restrict_unprivileged_userns"),
        read("/proc/sys/user/max_user_namespaces"),
        read("/proc/sys/kernel/unprivileged_userns_clone"),
    );
}
