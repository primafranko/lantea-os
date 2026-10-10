//! The Phase 1 Record writer: one structured journald entry per action,
//! sent over journald's native protocol. The entry format is decision 0017.

use std::io;
use std::os::unix::net::UnixDatagram;
use std::path::PathBuf;

/// journald's native protocol socket. Fixed: the installed binary has no
/// override (decision 0017).
const JOURNAL_SOCKET: &str = "/run/systemd/journal/socket";

/// An action that leaves an entry in the Record.
#[derive(Clone, Copy)]
pub enum Action {
    SetDown,
    Restore,
    Release,
    /// A release whose deletion failed partway, after its entry was written.
    ReleaseIncomplete,
}

impl Action {
    /// The `LANTEA_ACTION` value.
    pub fn name(self) -> &'static str {
        match self {
            Action::SetDown => "set-down",
            Action::Restore => "restore",
            Action::Release => "release",
            Action::ReleaseIncomplete => "release-incomplete",
        }
    }

    /// The fixed `MESSAGE_ID` of the action (decision 0017).
    fn message_id(self) -> &'static str {
        match self {
            Action::SetDown => "2667c9a683a44b908958fc443423992e",
            Action::Restore => "74f4239d420c461791598bcc3b4991cc",
            Action::Release => "9095fa701fd14a6c94a119136543dbea",
            Action::ReleaseIncomplete => "a2a943364ecc42288bf2ab9c44ae4db8",
        }
    }

    /// `PRIORITY`: notice, or warning for an incomplete release.
    fn priority(self) -> &'static str {
        match self {
            Action::ReleaseIncomplete => "4",
            _ => "5",
        }
    }
}

/// One Record entry.
pub struct Entry<'a> {
    pub action: Action,
    /// The sentence the command printed, without its newline, unescaped.
    pub message: &'a str,
    pub held_id: &'a [u8],
    /// The original path for set-down and release, the destination for restore.
    pub path: &'a [u8],
    /// Bytes, for set-down and release only.
    pub size: Option<u64>,
}

impl Entry<'_> {
    /// The entry's fields, in the order they are sent and explained.
    fn fields(&self) -> Vec<(&'static str, Vec<u8>)> {
        let mut fields = vec![
            ("MESSAGE", self.message.as_bytes().to_vec()),
            ("MESSAGE_ID", self.action.message_id().as_bytes().to_vec()),
            ("PRIORITY", self.action.priority().as_bytes().to_vec()),
            ("SYSLOG_IDENTIFIER", b"lantea".to_vec()),
            ("LANTEA_RECORD_VERSION", b"1".to_vec()),
            ("LANTEA_ACTION", self.action.name().as_bytes().to_vec()),
            ("LANTEA_HELD_ID", self.held_id.to_vec()),
            ("LANTEA_PATH", self.path.to_vec()),
        ];
        if let Some(size) = self.size {
            fields.push(("LANTEA_SIZE", size.to_string().into_bytes()));
        }
        fields
    }

    /// The `logger --journald` block that `--explain` prints.
    pub fn explain(&self) -> String {
        let mut out = String::from("logger --journald <<'EOF'\n");
        for (key, value) in self.fields() {
            out.push_str(key);
            out.push('=');
            // Escaped, so a value cannot end the heredoc or add a field.
            out.push_str(&crate::held::esc(&String::from_utf8_lossy(&value)));
            out.push('\n');
        }
        out.push_str("EOF");
        out
    }

    /// The datagram in journald's native protocol. A value containing a
    /// newline uses the binary form: `KEY\n<u64 LE length><value>\n`.
    fn encode(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        for (key, value) in self.fields() {
            buf.extend_from_slice(key.as_bytes());
            if value.contains(&b'\n') {
                buf.push(b'\n');
                buf.extend_from_slice(&(value.len() as u64).to_le_bytes());
            } else {
                buf.push(b'=');
            }
            buf.extend_from_slice(&value);
            buf.push(b'\n');
        }
        buf
    }

    /// Sends the entry to journald.
    pub fn send(&self) -> io::Result<()> {
        let socket = UnixDatagram::unbound()?;
        socket.send_to(&self.encode(), socket_path())?;
        Ok(())
    }
}

#[cfg(not(feature = "test-journal-socket"))]
fn socket_path() -> PathBuf {
    PathBuf::from(JOURNAL_SOCKET)
}

/// Test builds only: the tests redirect the Record to their own socket.
#[cfg(feature = "test-journal-socket")]
fn socket_path() -> PathBuf {
    std::env::var_os("LANTEA_JOURNAL_SOCKET")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(JOURNAL_SOCKET))
}
