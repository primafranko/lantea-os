//! Held: the user's XDG Trash (decision 0006), and `set-down`, `held`,
//! `restore` and `release` on it (decision 0018).
//!
//! The Trash directories are opened once as directory handles and every
//! change works relative to a handle, without following symlinks. Paths built
//! as strings are only shown to the user, printed by `--explain` and recorded.
//! Text shown on the terminal has its control characters escaped (`esc`);
//! `--json` and the Record keep the raw text.

use std::collections::HashSet;
use std::env;
use std::ffi::{OsStr, OsString};
use std::fmt::Write as _;
use std::fs;
use std::io::{self, BufRead, IsTerminal, Read, Write};
use std::os::fd::{AsFd, AsRawFd, BorrowedFd, OwnedFd};
use std::os::unix::ffi::{OsStrExt, OsStringExt};
use std::path::{Component, Path, PathBuf};

use chrono::NaiveDateTime;
use percent_encoding::{AsciiSet, NON_ALPHANUMERIC, percent_decode_str, utf8_percent_encode};
use rustix::fs::{self as rfs, AtFlags, FileType, Mode, OFlags, RenameFlags, StatxFlags};
use rustix::io::Errno;
use serde::Serialize;

use crate::record::{Action, Entry};

/// The global flags that change what a command does.
pub struct Opts {
    pub json: bool,
    pub explain: bool,
    pub yes: bool,
}

/// Why a command failed or refused: one or more plain sentences for stderr,
/// unescaped. The command then exits 1.
pub struct Failure(Vec<String>);

impl From<String> for Failure {
    fn from(line: String) -> Self {
        Failure(vec![line])
    }
}

impl Failure {
    /// Prints each sentence on its own line, control characters escaped.
    pub fn print(&self) {
        for line in &self.0 {
            eprintln!("{}", esc(line));
        }
    }
}

pub type Outcome = Result<(), Failure>;

const NOTHING_CHANGED: &str = "Nothing was changed.";

/// The XDG Trash spec's `DeletionDate` format, local time.
const DATE_FORMAT: &str = "%Y-%m-%dT%H:%M:%S";

/// Info files larger than this are treated as unparsable.
const INFO_LIMIT: u64 = 64 * 1024;

/// Percent-encoding for `Path=`: everything except unreserved characters and `/`.
const PATH_ENCODE: &AsciiSet = &NON_ALPHANUMERIC
    .remove(b'-')
    .remove(b'.')
    .remove(b'_')
    .remove(b'~')
    .remove(b'/');

const DIR_FLAGS: OFlags = OFlags::RDONLY
    .union(OFlags::DIRECTORY)
    .union(OFlags::NOFOLLOW)
    .union(OFlags::CLOEXEC);

// ------------------------------------------------------------------ helpers

/// Shows each control character (U+0000–U+001F, U+007F, U+0080–U+009F) as
/// `\xHH`, and the backslash as `\x5c`, so untrusted text cannot change what
/// the terminal shows and the display stays unambiguous.
pub fn esc(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        let n = u32::from(c);
        if n < 0x20 || (0x7f..=0x9f).contains(&n) || c == '\\' {
            let _ = write!(out, "\\x{n:02x}");
        } else {
            out.push(c);
        }
    }
    out
}

/// Prints one line to stdout, escaped.
fn say(line: &str) {
    println!("{}", esc(line));
}

fn lossy(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

fn show(path: &Path) -> String {
    lossy(path.as_os_str().as_bytes())
}

/// Shell-quotes a word when it has characters outside `[A-Za-z0-9._+/-]`.
fn sh_quote(word: &[u8]) -> String {
    let text = lossy(word);
    if !text.is_empty()
        && text
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._+/-".contains(&b))
    {
        text
    } else {
        // `'"'"'` rather than `'\''`: no backslash, so escaping for the
        // terminal leaves the quoting readable.
        format!("'{}'", text.replace('\'', "'\"'\"'"))
    }
}

fn q(path: &Path) -> String {
    sh_quote(path.as_os_str().as_bytes())
}

fn with_suffix(name: &OsStr, suffix: &str) -> OsString {
    let mut s = name.to_os_string();
    s.push(suffix);
    s
}

fn info_name(id: &OsStr) -> OsString {
    with_suffix(id, ".trashinfo")
}

/// A held id is one file name: not empty, `.` or `..`, and without `/`.
fn valid_id(id: &OsStr) -> bool {
    let b = id.as_bytes();
    !b.is_empty() && b != b"." && b != b".." && !b.contains(&b'/')
}

fn unknown_id(id: &OsStr) -> String {
    format!(
        "Nothing is held as {}. To see what is held: lantea held",
        lossy(id.as_bytes())
    )
}

/// Makes a path absolute against the current directory, dropping `.`
/// components and trailing slashes.
fn absolute(path: &Path) -> io::Result<PathBuf> {
    let joined = if path.is_absolute() {
        path.to_path_buf()
    } else {
        env::current_dir()?.join(path)
    };
    Ok(joined.components().collect())
}

/// The real path of an open handle.
fn real_path(fd: BorrowedFd<'_>) -> io::Result<PathBuf> {
    fs::read_link(format!("/proc/self/fd/{}", fd.as_raw_fd()))
}

/// The real path of `path`, canonicalising its deepest existing ancestor.
fn lenient_real(path: &Path) -> PathBuf {
    let mut rest = Vec::new();
    let mut cur = path;
    loop {
        if let Ok(real) = fs::canonicalize(cur) {
            return rest.iter().rev().fold(real, |p, c| p.join(c));
        }
        match (cur.parent(), cur.file_name()) {
            (Some(parent), Some(name)) => {
                rest.push(name.to_os_string());
                cur = parent;
            }
            _ => return path.to_path_buf(),
        }
    }
}

/// What one `statx` call says about an entry, without following a symlink.
struct Node {
    kind: FileType,
    size: u64,
    /// Device and inode, to count hard links once.
    inode: (u32, u32, u64),
    /// The mount ID: a mount is never identified by `st_dev`.
    mount: u64,
}

fn statx_node(dir: BorrowedFd<'_>, name: &OsStr, flags: AtFlags) -> io::Result<Node> {
    let stx = rfs::statx(
        dir,
        name,
        flags | AtFlags::SYMLINK_NOFOLLOW,
        StatxFlags::BASIC_STATS | StatxFlags::MNT_ID,
    )?;
    if stx.stx_mask & StatxFlags::MNT_ID.bits() == 0 {
        return Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "the kernel does not report mount IDs",
        ));
    }
    Ok(Node {
        kind: FileType::from_raw_mode(u32::from(stx.stx_mode)),
        size: stx.stx_size,
        inode: (stx.stx_dev_major, stx.stx_dev_minor, stx.stx_ino),
        mount: stx.stx_mnt_id,
    })
}

/// The entry `name` in `dir`.
fn node(dir: BorrowedFd<'_>, name: &OsStr) -> io::Result<Node> {
    statx_node(dir, name, AtFlags::empty())
}

/// The directory `dir` itself.
fn node_of(dir: BorrowedFd<'_>) -> io::Result<Node> {
    statx_node(dir, OsStr::new(""), AtFlags::EMPTY_PATH)
}

/// The mount of `<T>/files`: the base of every mount check.
#[derive(Clone, Copy)]
struct HeldMount(u64);

impl HeldMount {
    fn of(files: BorrowedFd<'_>) -> io::Result<HeldMount> {
        Ok(HeldMount(node_of(files)?.mount))
    }

    /// For `--explain` before `<T>/files` exists: the mount of `path`'s
    /// deepest existing ancestor.
    fn lenient(path: &Path) -> io::Result<HeldMount> {
        let mut cur = path;
        loop {
            match rfs::open(cur, OFlags::PATH | OFlags::CLOEXEC, Mode::empty()) {
                Ok(fd) => return HeldMount::of(fd.as_fd()),
                Err(Errno::NOENT) => match cur.parent() {
                    Some(parent) => cur = parent,
                    None => return Err(Errno::NOENT.into()),
                },
                Err(e) => return Err(e.into()),
            }
        }
    }

    /// The one mount check: is this on the same mount as `<T>/files`?
    fn holds(self, n: &Node) -> bool {
        n.mount == self.0
    }
}

fn separate_mount() -> io::Error {
    io::Error::other("it is a separate mount")
}

fn entries(dir: BorrowedFd<'_>) -> io::Result<Vec<OsString>> {
    let mut names = Vec::new();
    for entry in rfs::Dir::read_from(dir)? {
        let name = entry?.file_name().to_bytes().to_vec();
        if name != b"." && name != b".." {
            names.push(OsString::from_vec(name));
        }
    }
    Ok(names)
}

/// The size of `name` in `dir`, as `du -s --apparent-size --block-size=1`
/// reports it: the `lstat` size of the item and of every entry under it,
/// counting symlinks and directories themselves and hard links once, never
/// following a symlink and never entering another mount. An item that is
/// itself on another mount than `<T>/files` has no size.
fn size_at(dir: BorrowedFd<'_>, name: &OsStr, base: HeldMount) -> io::Result<u64> {
    if !base.holds(&node(dir, name)?) {
        return Err(separate_mount());
    }
    tree_size(dir, name, base, &mut HashSet::new())
}

fn tree_size(
    dir: BorrowedFd<'_>,
    name: &OsStr,
    base: HeldMount,
    seen: &mut HashSet<(u32, u32, u64)>,
) -> io::Result<u64> {
    let n = node(dir, name)?;
    if !base.holds(&n) || !seen.insert(n.inode) {
        return Ok(0);
    }
    let mut total = n.size;
    if n.kind.is_dir() {
        let fd = rfs::openat(dir, name, DIR_FLAGS, Mode::empty())?;
        for child in entries(fd.as_fd())? {
            total += tree_size(fd.as_fd(), &child, base, seen)?;
        }
    }
    Ok(total)
}

/// The first entry at or under `name` in `dir` on another mount than
/// `<T>/files`, as a path under `shown`.
fn find_mount(
    dir: BorrowedFd<'_>,
    name: &OsStr,
    base: HeldMount,
    shown: &Path,
) -> io::Result<Option<PathBuf>> {
    let n = node(dir, name)?;
    if !base.holds(&n) {
        return Ok(Some(shown.to_path_buf()));
    }
    if n.kind.is_dir() {
        let fd = rfs::openat(dir, name, DIR_FLAGS, Mode::empty())?;
        for child in entries(fd.as_fd())? {
            if let Some(found) = find_mount(fd.as_fd(), &child, base, &shown.join(&child))? {
                return Ok(Some(found));
            }
        }
    }
    Ok(None)
}

/// Deletes `name` in `dir`, recursively for directories, through directory
/// handles, never following a symlink and never entering another mount than
/// `<T>/files`. An entry on another mount is left in place, with the
/// directories above it; the first such entry is put in `left`. Returns
/// whether all of it is gone.
fn remove_at(
    dir: BorrowedFd<'_>,
    name: &OsStr,
    base: HeldMount,
    shown: &Path,
    left: &mut Option<PathBuf>,
    failed_at: &mut Option<PathBuf>,
) -> io::Result<bool> {
    // On an error, the directory whose permissions matter: the parent for
    // looking up or unlinking an entry, the directory itself for reading it.
    let parent = shown.parent().unwrap_or(shown);
    let n = node(dir, name).inspect_err(|_| note(failed_at, parent))?;
    if !base.holds(&n) {
        left.get_or_insert_with(|| shown.to_path_buf());
        return Ok(false);
    }
    if n.kind.is_dir() {
        let fd = rfs::openat(dir, name, DIR_FLAGS, Mode::empty())
            .inspect_err(|_| note(failed_at, shown))?;
        let children = entries(fd.as_fd()).inspect_err(|_| note(failed_at, shown))?;
        let mut all = true;
        for child in children {
            all &= remove_at(
                fd.as_fd(),
                &child,
                base,
                &shown.join(&child),
                left,
                failed_at,
            )?;
        }
        if !all {
            return Ok(false);
        }
        rfs::unlinkat(dir, name, AtFlags::REMOVEDIR).inspect_err(|_| note(failed_at, parent))?;
    } else {
        rfs::unlinkat(dir, name, AtFlags::empty()).inspect_err(|_| note(failed_at, parent))?;
    }
    Ok(true)
}

/// Keeps the first path where removal failed.
fn note(failed_at: &mut Option<PathBuf>, path: &Path) {
    failed_at.get_or_insert_with(|| path.to_path_buf());
}

/// The final sentence for a reason with a known remedy, if there is one.
fn remedy(e: &io::Error, path: Option<&Path>) -> Option<String> {
    match e.kind() {
        io::ErrorKind::Unsupported => Some("Lantea needs Linux 5.8 or later.".to_string()),
        io::ErrorKind::PermissionDenied => path.map(|p| format!("Check that you own {}.", show(p))),
        _ => None,
    }
}

/// Appends a remedy as the message's final sentence.
fn with_remedy(message: String, remedy: Option<String>) -> String {
    match remedy {
        None => message,
        Some(r) if message.ends_with('.') => format!("{message} {r}"),
        Some(r) => format!("{message}. {r}"),
    }
}

/// `1536` → `1.5 KiB`; below 1024, `<n> B`. The unit is picked after
/// rounding, so 1048575 is `1.0 MiB`.
fn human_size(bytes: u64) -> String {
    if bytes < 1024 {
        return format!("{bytes} B");
    }
    let units = ["KiB", "MiB", "GiB", "TiB"];
    let mut value = bytes as f64 / 1024.0;
    let mut unit = 0;
    while unit < units.len() - 1 && (value * 10.0).round() / 10.0 >= 1024.0 {
        value /= 1024.0;
        unit += 1;
    }
    format!("{value:.1} {}", units[unit])
}

fn size_text(size: Option<u64>) -> String {
    size.map_or_else(|| "size unknown".to_string(), human_size)
}

fn print_json<T: Serialize>(value: &T) {
    println!(
        "{}",
        serde_json::to_string(value).expect("a report serialises")
    );
}

/// Sends the entry of an action that has already happened.
fn record(entry: &Entry<'_>) -> Result<(), String> {
    entry.send().map_err(|e| {
        format!(
            "The {} was done, but the Record could not be written: {e}.",
            entry.action.name()
        )
    })
}

// ------------------------------------------------------------------ the Trash

/// The user's home and Trash.
struct Place {
    /// The home's real path.
    home: PathBuf,
    /// `$XDG_DATA_HOME/Trash`, or `~/.local/share/Trash`.
    trash: PathBuf,
}

fn place() -> Result<Place, String> {
    let home = env::var_os("HOME").map(PathBuf::from).ok_or_else(|| {
        format!("Your home directory is not known, because HOME is not set. {NOTHING_CHANGED}")
    })?;
    if !home.is_absolute() {
        return Err(format!(
            "HOME is set to a relative path ({}). Held needs an absolute path, so nothing was changed.",
            show(&home)
        ));
    }
    let data = env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .filter(|d| d.is_absolute())
        .unwrap_or_else(|| home.join(".local/share"));
    let trash = data.join("Trash");
    let home_fd = open_home(&home)?;
    let real = real_path(home_fd.as_fd()).map_err(|e| {
        format!(
            "Your home directory {} could not be opened: {e}. {NOTHING_CHANGED}",
            show(&home)
        )
    })?;
    check_owner(&home, home_fd, &trash)?;
    Ok(Place { home: real, trash })
}

/// Opens the home: it must exist, be a directory and belong to the caller.
fn open_home(home: &Path) -> Result<OwnedFd, String> {
    let fd = match rfs::open(
        home,
        OFlags::RDONLY | OFlags::DIRECTORY | OFlags::CLOEXEC,
        Mode::empty(),
    ) {
        Ok(fd) => fd,
        Err(Errno::NOENT | Errno::NOTDIR) => {
            return Err(format!(
                "Your home directory {} does not exist, so nothing was changed.",
                show(home)
            ));
        }
        Err(e) => {
            return Err(format!(
                "Your home directory {} could not be opened: {}. {NOTHING_CHANGED}",
                show(home),
                io::Error::from(e)
            ));
        }
    };
    let st = rfs::fstat(&fd).map_err(|e| {
        format!(
            "Your home directory {} could not be opened: {}. {NOTHING_CHANGED}",
            show(home),
            io::Error::from(e)
        )
    })?;
    if st.st_uid != rustix::process::geteuid().as_raw() {
        return Err(format!(
            "Your home directory {} belongs to another user, so lantea will not act on it. Run lantea as that user instead. {NOTHING_CHANGED}",
            show(home)
        ));
    }
    Ok(fd)
}

/// Held must belong to the caller: `<T>` and every directory leading to it
/// below the home are owned by the caller's uid. Each is opened without
/// following a symlink, through handles from the home. Directories that don't
/// exist yet are fine; `set-down` creates them.
fn check_owner(home: &Path, home_fd: OwnedFd, trash: &Path) -> Result<(), String> {
    let uid = rustix::process::geteuid().as_raw();
    let failed = |e: io::Error| {
        format!(
            "Your Held items at {} could not be opened: {e}. {NOTHING_CHANGED}",
            show(trash)
        )
    };
    // From the home when <T> is below it; otherwise <T> alone.
    let (mut dir, mut path, below): (OwnedFd, PathBuf, Vec<&OsStr>) = match trash.strip_prefix(home)
    {
        Ok(rest) => (home_fd, home.to_path_buf(), rest.iter().collect()),
        Err(_) => {
            let (Some(parent), Some(name)) = (trash.parent(), trash.file_name()) else {
                return Ok(());
            };
            match rfs::open(
                parent,
                OFlags::RDONLY | OFlags::DIRECTORY | OFlags::CLOEXEC,
                Mode::empty(),
            ) {
                Ok(fd) => (fd, parent.to_path_buf(), vec![name]),
                Err(Errno::NOENT) => return Ok(()),
                Err(e) => return Err(failed(e.into())),
            }
        }
    };
    for name in below {
        path.push(name);
        dir = match rfs::openat(&dir, name, DIR_FLAGS, Mode::empty()) {
            Ok(fd) => fd,
            Err(Errno::NOENT) => return Ok(()),
            Err(e @ (Errno::LOOP | Errno::NOTDIR)) => {
                let link = rfs::statat(&dir, name, AtFlags::SYMLINK_NOFOLLOW)
                    .is_ok_and(|st| FileType::from_raw_mode(st.st_mode).is_symlink());
                if link {
                    return Err(format!(
                        "{} is a symbolic link, so lantea will not use it for your Held items. {NOTHING_CHANGED} Replace the link with a folder to use lantea.",
                        show(&path)
                    ));
                }
                return Err(failed(e.into()));
            }
            Err(e) => return Err(failed(e.into())),
        };
        if rfs::fstat(&dir).map_err(|e| failed(e.into()))?.st_uid != uid {
            return Err(format!(
                "Held at {} belongs to another user, so lantea will not act on it. Run lantea as that user instead. {NOTHING_CHANGED}",
                show(trash)
            ));
        }
    }
    Ok(())
}

/// Handles on `<T>/files` and `<T>/info`, and the mount of `<T>/files`;
/// `None` when they don't exist yet.
struct Trash {
    files: Option<OwnedFd>,
    info: Option<OwnedFd>,
    base: Option<HeldMount>,
}

impl Trash {
    /// Opens the Trash; with `create`, makes `<T>`, `<T>/files` and
    /// `<T>/info` (mode 700) when missing.
    fn open(path: &Path, create: bool) -> Result<Trash, String> {
        Self::try_open(path, create).map_err(|e| {
            format!(
                "Your Held items at {} could not be opened: {e}. {NOTHING_CHANGED}",
                show(path)
            )
        })
    }

    fn try_open(path: &Path, create: bool) -> io::Result<Trash> {
        if create {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            match rfs::mkdir(path, Mode::RWXU) {
                Ok(()) | Err(Errno::EXIST) => {}
                Err(e) => return Err(e.into()),
            }
        }
        let top = match rfs::open(path, DIR_FLAGS, Mode::empty()) {
            Ok(fd) => fd,
            Err(Errno::NOENT) if !create => {
                return Ok(Trash {
                    files: None,
                    info: None,
                    base: None,
                });
            }
            Err(e) => return Err(e.into()),
        };
        let sub = |name: &str| -> io::Result<Option<OwnedFd>> {
            if create {
                match rfs::mkdirat(&top, name, Mode::RWXU) {
                    Ok(()) | Err(Errno::EXIST) => {}
                    Err(e) => return Err(e.into()),
                }
            }
            match rfs::openat(&top, name, DIR_FLAGS, Mode::empty()) {
                Ok(fd) => Ok(Some(fd)),
                Err(Errno::NOENT) if !create => Ok(None),
                Err(e) => Err(e.into()),
            }
        };
        let files = sub("files")?;
        let base = files
            .as_ref()
            .map(|f| HeldMount::of(f.as_fd()))
            .transpose()?;
        Ok(Trash {
            files,
            info: sub("info")?,
            base,
        })
    }

    /// The handles and the mount of `<T>/files`, when both exist.
    fn handles(&self) -> Option<(BorrowedFd<'_>, BorrowedFd<'_>, HeldMount)> {
        Some((
            self.files.as_ref()?.as_fd(),
            self.info.as_ref()?.as_fd(),
            self.base?,
        ))
    }
}

/// What a `.trashinfo` says.
struct Info {
    /// `Path=`, percent-decoded and absolute. Otherwise untrusted.
    path: Vec<u8>,
    /// `DeletionDate=` as written.
    date_raw: String,
    date: NaiveDateTime,
}

fn parse_info(text: &[u8]) -> Option<Info> {
    let text = std::str::from_utf8(text).ok()?;
    let mut in_group = false;
    let mut path = None;
    let mut date = None;
    for line in text.lines() {
        if line.starts_with('[') {
            in_group = line == "[Trash Info]";
        } else if in_group {
            if let Some(v) = line.strip_prefix("Path=") {
                path.get_or_insert(v);
            } else if let Some(v) = line.strip_prefix("DeletionDate=") {
                date.get_or_insert(v);
            }
        }
    }
    let path: Vec<u8> = percent_decode_str(path?).collect();
    // The home Trash holds absolute paths only.
    if path.first() != Some(&b'/') {
        return None;
    }
    let date_raw = date?.to_string();
    Some(Info {
        path,
        date: NaiveDateTime::parse_from_str(&date_raw, DATE_FORMAT).ok()?,
        date_raw,
    })
}

/// Reads an info file: no symlink, no waiting on a FIFO, a regular file of at
/// most 64 KiB. Anything else is `None`, the same as unparsable.
fn read_info(info: BorrowedFd<'_>, id: &OsStr) -> Option<Info> {
    let fd = rfs::openat(
        info,
        info_name(id).as_os_str(),
        OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC,
        Mode::empty(),
    )
    .ok()?;
    let st = rfs::fstat(&fd).ok()?;
    if !FileType::from_raw_mode(st.st_mode).is_file() {
        return None;
    }
    let mut text = Vec::new();
    fs::File::from(fd)
        .take(INFO_LIMIT + 1)
        .read_to_end(&mut text)
        .ok()?;
    if text.len() as u64 > INFO_LIMIT {
        return None;
    }
    parse_info(&text)
}

/// One held item: its entry in `files/` and its parsable info file.
struct Item {
    id: OsString,
    info: Info,
    /// `None` when it cannot be measured.
    size: Option<u64>,
}

/// Loads a held item, or `None` when either half is missing or unparsable.
fn load_item(
    files: BorrowedFd<'_>,
    info: BorrowedFd<'_>,
    base: HeldMount,
    id: &OsStr,
) -> Option<Item> {
    rfs::statat(files, id, AtFlags::SYMLINK_NOFOLLOW).ok()?;
    Some(Item {
        id: id.to_os_string(),
        info: read_info(info, id)?,
        size: size_at(files, id, base).ok(),
    })
}

/// Opens the Trash without creating it and loads one item by id.
fn find(place: &Place, id: &OsStr) -> Result<(Trash, Item), String> {
    if !valid_id(id) {
        return Err(unknown_id(id));
    }
    let trash = Trash::open(&place.trash, false)?;
    let item = trash
        .handles()
        .and_then(|(files, info, base)| load_item(files, info, base, id))
        .ok_or_else(|| unknown_id(id))?;
    Ok((trash, item))
}

#[derive(Serialize)]
struct ActionReport<'a> {
    action: &'a str,
    id: String,
    path: String,
    size: Option<u64>,
}

// ------------------------------------------------------------------ set-down

pub fn set_down(opts: &Opts, arg: &Path) -> Outcome {
    let place = place()?;
    let mut given = absolute(arg).map_err(|e| {
        format!(
            "{} could not be set down: {e}. {NOTHING_CHANGED}",
            show(arg)
        )
    })?;
    let missing = |p: &Path| format!("{} does not exist. {NOTHING_CHANGED}", show(p));
    let outside = |p: &Path| {
        format!(
            "{} is outside the Tended Store, so it cannot be set down. {NOTHING_CHANGED}",
            show(p)
        )
    };
    let failed = |p: &Path, e: &dyn std::fmt::Display| {
        format!("{} could not be set down: {e}. {NOTHING_CHANGED}", show(p))
    };
    let shown = given.clone();

    // A path ending in `..` names a directory: resolve it fully first.
    if given.file_name().is_none() {
        given = fs::canonicalize(&given).map_err(|_| missing(&shown))?;
    }
    let (Some(parent), Some(name)) = (given.parent(), given.file_name()) else {
        return Err(outside(&shown).into());
    };

    // Canonicalise the parent, open it, and decide on the handle's real path.
    let parent_fd = fs::canonicalize(parent)
        .and_then(|p| rfs::open(&p, DIR_FLAGS, Mode::empty()).map_err(io::Error::from))
        .map_err(|e| match e.kind() {
            io::ErrorKind::NotFound | io::ErrorKind::NotADirectory => missing(&shown),
            _ => failed(&shown, &e),
        })?;
    let parent_real = real_path(parent_fd.as_fd()).map_err(|e| failed(&shown, &e))?;
    let item = match node(parent_fd.as_fd(), name) {
        Ok(n) => n,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Err(missing(&shown).into()),
        Err(e) => return Err(failed(&shown, &e).into()),
    };
    let target = parent_real.join(name);

    let trash = Trash::open(&place.trash, !opts.explain)?;
    let trash_real = lenient_real(&place.trash);
    if target == place.home || trash_real.starts_with(&target) || target.starts_with(&trash_real) {
        return Err(format!(
            "{} contains your Held items, so it cannot be set down. {NOTHING_CHANGED}",
            show(&shown)
        )
        .into());
    }
    if !target.starts_with(&place.home) {
        return Err(outside(&shown).into());
    }
    let base = match trash.base {
        Some(base) => Ok(base),
        None => HeldMount::lenient(&place.trash),
    }
    .map_err(|e| failed(&shown, &e))?;
    if !base.holds(&item) {
        return Err(different_fs(&shown).into());
    }
    if let Some(mount_point) =
        find_mount(parent_fd.as_fd(), name, base, &shown).map_err(|e| failed(&shown, &e))?
    {
        return Err(format!(
            "{} contains {}, which is a separate mount, so it cannot be set down. {NOTHING_CHANGED}",
            show(&shown),
            show(&mount_point)
        )
        .into());
    }

    let size = size_at(parent_fd.as_fd(), name, base).map_err(|e| failed(&shown, &e))?;
    let encoded = utf8_percent_encode_bytes(target.as_os_str().as_bytes());
    let date = chrono::Local::now().format(DATE_FORMAT).to_string();

    // Reserve an id: `name`, then `name.2`, `name.3`, … (XDG Trash spec).
    let taken = |dir: &Option<OwnedFd>, candidate: &OsStr| {
        dir.as_ref()
            .is_some_and(|d| rfs::statat(d, candidate, AtFlags::SYMLINK_NOFOLLOW).is_ok())
    };
    let mut n = 1u64;
    let (id, reserved) = loop {
        let candidate = if n == 1 {
            name.to_os_string()
        } else {
            with_suffix(name, &format!(".{n}"))
        };
        n += 1;
        if taken(&trash.files, &candidate) {
            continue;
        }
        let info_file = info_name(&candidate);
        if opts.explain {
            if taken(&trash.info, &info_file) {
                continue;
            }
            break (candidate, None);
        }
        let info = trash.info.as_ref().expect("created above");
        match rfs::openat(
            info,
            &info_file,
            OFlags::WRONLY | OFlags::CREATE | OFlags::EXCL | OFlags::NOFOLLOW | OFlags::CLOEXEC,
            Mode::RUSR | Mode::WUSR,
        ) {
            Ok(fd) => break (candidate, Some(fd)),
            Err(Errno::EXIST) => continue,
            Err(e) => return Err(failed(&shown, &e).into()),
        }
    };

    let id_text = lossy(id.as_bytes());
    let sentence = format!(
        "Set down {}. It is held as {id_text}; to bring it back: lantea restore {}",
        show(&target),
        sh_quote(id.as_bytes())
    );
    let entry = Entry {
        action: Action::SetDown,
        message: &sentence,
        held_id: id.as_bytes(),
        path: target.as_os_str().as_bytes(),
        size: Some(size),
    };

    let Some(reserved) = reserved else {
        let t = &place.trash;
        say(&format!(
            "mkdir -p -m 700 -- {} {}",
            q(&t.join("files")),
            q(&t.join("info"))
        ));
        // The format string is lantea's own; only the file name is escaped.
        // The percent-encoded path and the date have no control characters.
        println!(
            "printf '[Trash Info]\\nPath=%s\\nDeletionDate=%s\\n' '{encoded}' '{date}' > {}",
            esc(&q(&t.join("info").join(info_name(&id))))
        );
        say(&format!(
            "mv -n -- {} {}",
            q(&target),
            q(&t.join("files").join(&id))
        ));
        println!("{}", entry.explain());
        return Ok(());
    };

    let (files, info, _) = trash.handles().expect("created above");
    let info_file = info_name(&id);
    let undo = |e: &dyn std::fmt::Display| {
        let _ = rfs::unlinkat(info, &info_file, AtFlags::empty());
        failed(&shown, e)
    };
    let content = format!("[Trash Info]\nPath={encoded}\nDeletionDate={date}\n");
    fs::File::from(reserved)
        .write_all(content.as_bytes())
        .map_err(|e| undo(&e))?;
    match rfs::renameat_with(&parent_fd, name, files, &id, RenameFlags::NOREPLACE) {
        Ok(()) => {}
        Err(Errno::XDEV) => {
            let _ = rfs::unlinkat(info, &info_file, AtFlags::empty());
            return Err(different_fs(&shown).into());
        }
        Err(e) => return Err(undo(&e).into()),
    }

    if opts.json {
        print_json(&ActionReport {
            action: "set-down",
            id: id_text,
            path: show(&target),
            size: Some(size),
        });
    } else {
        say(&sentence);
    }
    Ok(record(&entry)?)
}

fn different_fs(shown: &Path) -> String {
    format!(
        "{} is on a separate mount from your Held items, so it cannot be set down. {NOTHING_CHANGED}",
        show(shown)
    )
}

fn utf8_percent_encode_bytes(bytes: &[u8]) -> String {
    // Percent-encode raw bytes: valid UTF-8 runs go through the encoder and
    // every other byte is written as %XX.
    let mut out = String::new();
    for chunk in bytes.utf8_chunks() {
        out.extend(utf8_percent_encode(chunk.valid(), PATH_ENCODE));
        for b in chunk.invalid() {
            let _ = write!(out, "%{b:02X}");
        }
    }
    out
}

// ------------------------------------------------------------------ held

#[derive(Serialize)]
struct HeldItem {
    id: String,
    path: String,
    deleted_at: String,
    size: Option<u64>,
}

#[derive(Serialize)]
struct HeldReport {
    items: Vec<HeldItem>,
    /// Info files whose item exists but which could not be read or parsed.
    unreadable: usize,
}

pub fn held(opts: &Opts) -> Outcome {
    let place = place()?;
    let t = q(&place.trash);
    if opts.explain {
        say(&format!("cat -- {t}/info/*.trashinfo"));
        say(&format!(
            "du -s --apparent-size --block-size=1 -- {t}/files/*"
        ));
        return Ok(());
    }
    let trash = Trash::open(&place.trash, false)?;
    let mut items = Vec::new();
    let mut unreadable = 0;
    if let Some((files, info, base)) = trash.handles() {
        let names = entries(info).map_err(|e| {
            format!(
                "Your Held items at {} could not be read: {e}.",
                show(&place.trash)
            )
        })?;
        for name in names {
            let Some(id) = name.as_bytes().strip_suffix(b".trashinfo") else {
                continue;
            };
            let id = OsStr::from_bytes(id);
            // An info file without its item is not counted.
            if !valid_id(id) || rfs::statat(files, id, AtFlags::SYMLINK_NOFOLLOW).is_err() {
                continue;
            }
            match load_item(files, info, base, id) {
                Some(item) => items.push(item),
                None => unreadable += 1,
            }
        }
    }
    items.sort_by(|a, b| b.info.date.cmp(&a.info.date).then_with(|| a.id.cmp(&b.id)));

    if opts.json {
        print_json(&HeldReport {
            items: items
                .iter()
                .map(|i| HeldItem {
                    id: lossy(i.id.as_bytes()),
                    path: lossy(&i.info.path),
                    deleted_at: i.info.date_raw.clone(),
                    size: i.size,
                })
                .collect(),
            unreadable,
        });
        return Ok(());
    }
    if items.is_empty() {
        say("Nothing is held.");
    } else {
        let noun = if items.len() == 1 { "item" } else { "items" };
        say(&format!("Held · {} {noun}", items.len()));
        for i in &items {
            say(&format!(
                "  {}  {}  {}  set down {}",
                lossy(i.id.as_bytes()),
                lossy(&i.info.path),
                size_text(i.size),
                i.info.date.format("%Y-%m-%d %H:%M")
            ));
        }
    }
    match unreadable {
        0 => {}
        1 => say("1 item in Held could not be read and is not shown."),
        n => say(&format!(
            "{n} items in Held could not be read and are not shown."
        )),
    }
    Ok(())
}

// ------------------------------------------------------------------ restore

#[derive(Serialize)]
struct RestoreReport {
    action: &'static str,
    id: String,
    path: String,
    created: Vec<String>,
}

/// True when every `/`-separated segment is neither `.` nor `..` and the
/// path is absolute.
fn plain_absolute(bytes: &[u8]) -> bool {
    bytes.first() == Some(&b'/')
        && bytes
            .split(|&b| b == b'/')
            .all(|seg| seg != b"." && seg != b"..")
}

pub fn restore(opts: &Opts, id: &OsStr, to: Option<&Path>) -> Outcome {
    let place = place()?;
    let (trash, item) = find(&place, id)?;
    let (files, info, base) = trash.handles().expect("an item was found");
    let id_text = lossy(id.as_bytes());
    let elsewhere = format!(
        "To restore it elsewhere: lantea restore {} --to <path>",
        sh_quote(id.as_bytes())
    );

    // The destination, and the refusal when it leads outside the home.
    let (dest, outside) = match to {
        Some(to) => {
            let dest = absolute(to).map_err(|e| {
                format!(
                    "{id_text} could not be restored to {}: {e}. {NOTHING_CHANGED}",
                    show(to)
                )
            })?;
            let outside = format!(
                "{} is outside the Tended Store, so {id_text} was not restored. {NOTHING_CHANGED}",
                show(&dest)
            );
            if dest.components().any(|c| c == Component::ParentDir) {
                return Err(outside.into());
            }
            (dest, outside)
        }
        None => {
            let outside = format!(
                "{id_text} was set down from {}, which is outside the Tended Store, so it was not restored there. {elsewhere}",
                lossy(&item.info.path)
            );
            if !plain_absolute(&item.info.path) {
                return Err(outside.into());
            }
            let dest: PathBuf = PathBuf::from(OsString::from_vec(item.info.path.clone()))
                .components()
                .collect();
            (dest, outside)
        }
    };
    let failed = |e: &dyn std::fmt::Display| {
        format!(
            "{id_text} could not be restored to {}: {e}. {NOTHING_CHANGED}",
            show(&dest)
        )
    };
    let (Some(parent), Some(name)) = (dest.parent(), dest.file_name()) else {
        return Err(outside.into());
    };

    // The deepest existing ancestor, and the directories missing below it.
    let mut missing: Vec<&OsStr> = Vec::new();
    let mut ancestor = parent;
    let ancestor_fd = loop {
        match rfs::open(
            ancestor,
            OFlags::RDONLY | OFlags::DIRECTORY | OFlags::CLOEXEC,
            Mode::empty(),
        ) {
            Ok(fd) => break fd,
            Err(Errno::NOENT) => match (ancestor.parent(), ancestor.file_name()) {
                (Some(up), Some(dir)) => {
                    missing.push(dir);
                    ancestor = up;
                }
                _ => return Err(outside.into()),
            },
            Err(e) => return Err(failed(&e).into()),
        }
    };
    let ancestor_real = real_path(ancestor_fd.as_fd()).map_err(|e| failed(&e))?;
    if !ancestor_real.starts_with(&place.home) {
        return Err(outside.into());
    }
    // Restore never writes into Held: the destination, resolved through its
    // deepest existing ancestor, must not be <T> or anything below it.
    let resolved = missing
        .iter()
        .rev()
        .fold(ancestor_real.clone(), |p, dir| p.join(dir))
        .join(name);
    if resolved.starts_with(lenient_real(&place.trash)) {
        return Err(format!(
            "{} is inside your Held items, so {id_text} was not restored there. {NOTHING_CHANGED} {elsewhere}",
            show(&dest)
        )
        .into());
    }
    let exists = format!(
        "{} already exists, so {id_text} was not restored. {elsewhere}",
        show(&dest)
    );
    if missing.is_empty() && rfs::statat(&ancestor_fd, name, AtFlags::SYMLINK_NOFOLLOW).is_ok() {
        return Err(exists.into());
    }
    let different = format!(
        "{} is on a separate mount from your Held items, so {id_text} was not restored. {NOTHING_CHANGED}",
        show(&dest)
    );
    let item_node = node(files, id).map_err(|e| failed(&e))?;
    let ancestor_node = node_of(ancestor_fd.as_fd()).map_err(|e| failed(&e))?;
    if !base.holds(&item_node) || !base.holds(&ancestor_node) {
        return Err(different.into());
    }
    missing.reverse();

    let sentence = format!("Restored {id_text} to {}.", show(&dest));
    let entry = Entry {
        action: Action::Restore,
        message: &sentence,
        held_id: id.as_bytes(),
        path: dest.as_os_str().as_bytes(),
        size: None,
    };
    let held_file = place.trash.join("files").join(id);
    let info_path = place.trash.join("info").join(info_name(id));

    if opts.explain {
        if !missing.is_empty() {
            say(&format!("mkdir -p -- {}", q(parent)));
        }
        say(&format!("mv -n -- {} {}", q(&held_file), q(&dest)));
        say(&format!("rm -- {}", q(&info_path)));
        println!("{}", entry.explain());
        return Ok(());
    }

    // Recreate missing parents (mode 755 minus the umask), top first. Only
    // directories mkdirat really created count; they are removed again,
    // deepest first, if the restore then fails.
    let mut created: Vec<PathBuf> = Vec::new();
    // For each created directory: the index of its parent in `levels`, and its name.
    let mut made: Vec<(usize, &OsStr)> = Vec::new();
    let mut levels: Vec<OwnedFd> = vec![ancestor_fd];
    let mut dir_path = ancestor.to_path_buf();
    let moved: Result<(), String> = 'act: {
        for dir in &missing {
            let parent = levels.len() - 1;
            match rfs::mkdirat(&levels[parent], *dir, Mode::from_raw_mode(0o755)) {
                Ok(()) => {
                    made.push((parent, dir));
                    created.push(dir_path.join(dir));
                }
                Err(Errno::EXIST) => {}
                Err(e) => break 'act Err(failed(&e)),
            }
            match rfs::openat(&levels[parent], *dir, DIR_FLAGS, Mode::empty()) {
                Ok(fd) => levels.push(fd),
                Err(e) => break 'act Err(failed(&e)),
            }
            dir_path.push(dir);
        }
        let target = levels.last().expect("the ancestor is open");
        match rfs::renameat_with(files, id, target, name, RenameFlags::NOREPLACE) {
            Ok(()) => Ok(()),
            Err(Errno::EXIST) => Err(exists),
            Err(Errno::XDEV) => Err(different),
            Err(e) => Err(failed(&e)),
        }
    };
    if let Err(message) = moved {
        return Err(undo_created(&levels, &made, &created, message).into());
    }
    // The item is restored now; an info file left behind is reported after
    // the output and the Record entry.
    let info_error = rfs::unlinkat(info, info_name(id).as_os_str(), AtFlags::empty())
        .err()
        .map(|e| {
            format!(
                "{id_text} was restored to {}, but {} could not be removed: {}. It may still be listed by lantea held.",
                show(&dest),
                show(&info_path),
                io::Error::from(e)
            )
        });

    if opts.json {
        print_json(&RestoreReport {
            action: "restore",
            id: id_text,
            path: show(&dest),
            created: created.iter().map(|p| show(p)).collect(),
        });
    } else {
        if let Some(deepest) = created.last() {
            say(&format!(
                "Recreated {}, which no longer existed.",
                show(deepest)
            ));
        }
        say(&sentence);
    }
    let lines: Vec<String> = info_error.into_iter().chain(record(&entry).err()).collect();
    if lines.is_empty() {
        Ok(())
    } else {
        Err(Failure(lines))
    }
}

/// Removes the directories a failed restore created, deepest first. When
/// some cannot be removed, the refusal names them in place of "Nothing was
/// changed."
fn undo_created(
    levels: &[OwnedFd],
    made: &[(usize, &OsStr)],
    created: &[PathBuf],
    message: String,
) -> String {
    let mut still_there = Vec::new();
    for ((parent, dir), path) in made.iter().zip(created).rev() {
        if rfs::unlinkat(&levels[*parent], *dir, AtFlags::REMOVEDIR).is_err() {
            still_there.push(show(path));
        }
    }
    if still_there.is_empty() {
        return message;
    }
    still_there.reverse();
    let left = format!(
        "The directories {} were created and are still there.",
        still_there.join(", ")
    );
    match message.strip_suffix(NOTHING_CHANGED) {
        Some(head) => format!("{head}{left}"),
        None => format!("{message} {left}"),
    }
}

// ------------------------------------------------------------------ release

pub fn release(opts: &Opts, id: &OsStr) -> Outcome {
    let place = place()?;
    let (trash, item) = find(&place, id)?;
    let (files, info, base) = trash.handles().expect("an item was found");
    let id_text = lossy(id.as_bytes());
    let path_text = lossy(&item.info.path);
    let sentence = format!("Released {id_text}. {path_text} has been permanently deleted.");
    let entry = Entry {
        action: Action::Release,
        message: &sentence,
        held_id: id.as_bytes(),
        path: &item.info.path,
        size: item.size,
    };
    let held_file = place.trash.join("files").join(id);
    let info_path = place.trash.join("info").join(info_name(id));

    // An item that is itself a mount is refused before anything is recorded
    // or deleted.
    let top = node(files, id).map_err(|e| {
        let files_dir = place.trash.join("files");
        with_remedy(
            format!("{id_text} could not be released: {e}. {NOTHING_CHANGED}"),
            remedy(&e, Some(&files_dir)),
        )
    })?;
    if !base.holds(&top) {
        return Err(format!(
            "{id_text} is a separate mount, so it cannot be released. {NOTHING_CHANGED}"
        )
        .into());
    }

    // The entry is written before anything is deleted (decision 0017).
    if opts.explain {
        println!("{}", entry.explain());
        say(&format!("rm -rf -- {} {}", q(&held_file), q(&info_path)));
        return Ok(());
    }

    if !opts.yes {
        let stdin = io::stdin();
        if !stdin.is_terminal() {
            return Err(format!(
                "Release asks before deleting anything, but there is no terminal to ask on. To release without being asked: lantea release {} --yes",
                sh_quote(id.as_bytes())
            )
            .into());
        }
        eprint!(
            "{}",
            esc(&format!(
                "Release {id_text} ({path_text}, {})? It will be permanently deleted. [y/N] ",
                size_text(item.size)
            ))
        );
        let _ = io::stderr().flush();
        let mut answer = String::new();
        let confirmed = match stdin.lock().read_line(&mut answer) {
            Ok(_) => {
                let a = answer.trim().to_ascii_lowercase();
                a == "y" || a == "yes"
            }
            Err(_) => false,
        };
        if !confirmed {
            return Err("Nothing was released.".to_string().into());
        }
    }

    // A release the Record cannot hold does not happen.
    entry.send().map_err(|e| {
        format!(
            "The Record could not be written, so {id_text} was not released: {e}. Nothing was deleted."
        )
    })?;

    // Delete, never entering another mount; what is left keeps its info file.
    let mut left = None;
    let mut failed_at = None;
    let removed = remove_at(files, id, base, &held_file, &mut left, &mut failed_at);
    let failure = match removed {
        Ok(true) => None,
        Ok(false) => Some((
            format!(
                "{} is a separate mount, so it was left in place",
                show(&left.unwrap_or_else(|| held_file.clone()))
            ),
            None,
        )),
        Err(e) => {
            let fix = remedy(&e, failed_at.as_deref());
            Some((e.to_string(), fix))
        }
    };
    if let Some((reason, fix)) = failure {
        let message = with_remedy(
            format!(
                "{id_text} could not be released completely: {reason}. The release is in the Record, marked as incomplete. Some of it may already be deleted. To see what is held: lantea held"
            ),
            fix,
        );
        let incomplete = Entry {
            action: Action::ReleaseIncomplete,
            message: &message,
            held_id: id.as_bytes(),
            path: &item.info.path,
            size: item.size,
        };
        let mut lines = vec![message.clone()];
        if let Err(e) = incomplete.send() {
            lines.push(format!("The Record could not be written: {e}."));
        }
        return Err(Failure(lines));
    }
    let info_error = rfs::unlinkat(info, info_name(id).as_os_str(), AtFlags::empty())
        .err()
        .map(|e| {
            let e = io::Error::from(e);
            let info_dir = place.trash.join("info");
            with_remedy(
                format!(
                    "{id_text} was released, but {} could not be removed: {e}. It may still be listed by lantea held.",
                    show(&info_path),
                ),
                remedy(&e, Some(&info_dir)),
            )
        });

    if opts.json {
        print_json(&ActionReport {
            action: "release",
            id: id_text,
            path: path_text,
            size: item.size,
        });
    } else {
        say(&sentence);
    }
    info_error.map_or(Ok(()), |line| Err(line.into()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    #[test]
    fn a_failed_cleanup_names_the_directories_left() {
        let tmp = tempfile::tempdir().unwrap();
        let base = fs::canonicalize(tmp.path()).unwrap();
        let top = rfs::open(&base, DIR_FLAGS, Mode::empty()).unwrap();
        rfs::mkdirat(&top, "a", Mode::RWXU).unwrap();
        let a = rfs::openat(&top, "a", DIR_FLAGS, Mode::empty()).unwrap();
        rfs::mkdirat(&a, "b", Mode::RWXU).unwrap();
        let levels = vec![top, a];
        let made = [(0, OsStr::new("a")), (1, OsStr::new("b"))];
        let created = [base.join("a"), base.join("a/b")];
        // `b` cannot be removed from a read-only `a`, so `a` stays too.
        fs::set_permissions(base.join("a"), fs::Permissions::from_mode(0o500)).unwrap();
        let message = undo_created(
            &levels,
            &made,
            &created,
            format!("x could not be restored to y: z. {NOTHING_CHANGED}"),
        );
        fs::set_permissions(base.join("a"), fs::Permissions::from_mode(0o700)).unwrap();
        assert_eq!(
            message,
            format!(
                "x could not be restored to y: z. The directories {}, {} were created and are still there.",
                base.join("a").display(),
                base.join("a/b").display()
            )
        );

        // When the cleanup works, the message is unchanged.
        let message = undo_created(&levels, &made, &created, "m. Nothing was changed.".into());
        assert_eq!(message, "m. Nothing was changed.");
        assert!(!base.join("a").exists());
    }
}
