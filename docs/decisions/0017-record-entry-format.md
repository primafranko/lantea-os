# 0017 — The Record entry format

**Status:** Accepted · **Date:** 2026-10-10 · Amended 2026-10-10 (release order, `release-incomplete`, unknown sizes)

## Context

Decision 0005 brings a minimal Record writer in Phase 1: structured journald entries under the `lantea` identifier. Phase 4 adds sealing, retention, auditd and `lantea record`, and must be able to read every entry written before it. The format is fixed now so that Phase 4 has something stable to stay compatible with.

## Decision

Every Record entry is one journald entry, sent by `lantea` over journald's native protocol to `/run/systemd/journal/socket`, with exactly these fields:

| Field | Value |
|---|---|
| `MESSAGE` | One plain sentence saying what happened, the same sentence the command printed (for example `Set down /home/steward/a.txt. It is held as a.txt; to bring it back: lantea restore a.txt`). |
| `MESSAGE_ID` | A fixed 128-bit id per action, below. |
| `PRIORITY` | `5` (notice); `4` (warning) for `release-incomplete`. |
| `SYSLOG_IDENTIFIER` | `lantea` |
| `LANTEA_RECORD_VERSION` | `1` |
| `LANTEA_ACTION` | The action, below. |
| `LANTEA_HELD_ID` | The held item's id (its file name in `Trash/files`). |
| `LANTEA_PATH` | An absolute path: the original path for `set-down` and `release`, the destination for `restore`. |
| `LANTEA_SIZE` | Bytes, for `set-down` and `release` only; omitted when the size cannot be measured. |

Action ids (`MESSAGE_ID`):

| `LANTEA_ACTION` | `MESSAGE_ID` |
|---|---|
| `set-down` | `2667c9a683a44b908958fc443423992e` |
| `restore` | `74f4239d420c461791598bcc3b4991cc` |
| `release` | `9095fa701fd14a6c94a119136543dbea` |
| `release-incomplete` | `a2a943364ecc42288bf2ab9c44ae4db8` |

- **Who acted** is journald's trusted `_UID` (with `_PID`, `_COMM` and `_HOSTNAME`), which journald adds itself and a client cannot forge. Lantea writes no field naming the user.
- A value containing a newline is sent in journald's binary form.
- `set-down` and `restore` write their entry after the action succeeded. **`release` writes its entry first and deletes only if the write succeeded** (amended 2026-10-10): the glossary promises every release is logged, so a release the Record cannot hold does not happen. If deletion then fails partway, a second entry with `LANTEA_ACTION=release-incomplete` and `PRIORITY=4` says the release was incomplete, with the same `LANTEA_HELD_ID`, `LANTEA_PATH` and `LANTEA_SIZE`.
- A refusal, a declined release, `--explain` and read-only commands write no entry.
- If a `set-down` or `restore` entry cannot be sent, the action stands, and the command says so and exits 1. If a `release` entry cannot be sent, nothing is deleted, and the command says so and exits 1.
- The socket path is fixed. A test-only Cargo feature (`test-journal-socket`) lets the tests redirect it with `LANTEA_JOURNAL_SOCKET`; the shipped binary has no such override.

## Consequences

- Phase 4 reads entries with `LANTEA_RECORD_VERSION=1` as they are. A change to the fields means version `2`, and readers keep understanding `1`.
- New actions (for example the snapshot entries of task 004) add their own `LANTEA_ACTION` and `MESSAGE_ID` in a later record, without changing the fields above.
- Until Phase 4, any local user can write entries with `SYSLOG_IDENTIFIER=lantea` to the journal. `_UID` shows who did, so a forged entry is visible as such; Phase 4 restricts who may write them.
- Entries hold paths and file names, never file contents.
- A `release` entry can precede a deletion that then fails partway. The `release-incomplete` entry after it says so, so a reader of the Record sees both.
