The Record: a minimal writer in Phase 1, then sealing, retention and auditd rules in Phase 4.

The Phase 1 writer is `pkgs/lantea/cli/src/record.rs`. It sends one structured journald entry (identifier `lantea`) for each set-down, restore and release; the entry format is decision 0017. Sealing, retention, auditd and `lantea record` come in Phase 4.
