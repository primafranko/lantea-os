---
paths:
  - "**/*.rs"
  - "**/Cargo.toml"
---
# Rust

<!-- Managed by AiHarness (aih sync). Change it in the harness repo. -->

- WHEN an operation can fail → return `Result` with context (`anyhow` in binaries, `thiserror` in libraries); no `unwrap()`/`expect()` outside tests, except where infallibility is proven in a comment. <!-- id=RS-1 n=0 since=2026-10 check=none -->
- WHEN clippy warns → fix the code; `#[allow(...)]` only on the smallest item, with a comment why, never crate-wide. <!-- id=RS-2 n=0 since=2026-10 check=hook:stop-guard -->
- WHEN adding a dependency → check it is needed and maintained, prefer std, keep `Cargo.lock` committed; no git dependencies without a pinned `rev`. <!-- id=RS-3 n=0 since=2026-10 check=none -->
- WHEN running an external program → `std::process::Command` with each argument separate, never a shell string; check the exit status and include stderr in the error. <!-- id=RS-4 n=0 since=2026-10 check=none -->
- WHEN tempted to write `unsafe` → don't, unless the spec allows it; then document the invariant in a `// SAFETY:` comment. <!-- id=RS-5 n=0 since=2026-10 check=none -->
- WHEN producing machine-readable output → serialize a typed struct with serde; never build JSON by string formatting. <!-- id=RS-6 n=0 since=2026-10 check=none -->
- WHEN writing tests → unit tests beside the code (`#[cfg(test)]`), command-line behaviour in `tests/` against the built binary, asserting exact output and exit codes. <!-- id=RS-7 n=0 since=2026-10 check=none -->
