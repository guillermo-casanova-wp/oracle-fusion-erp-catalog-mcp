# Oracle ERP MCP — Agent Instructions

## Scope

These instructions apply to the entire project.

## Architecture

- `src/main.rs`: MCP JSON-RPC 2.0 transport over stdin/stdout.
- `src/db.rs`: models, SQLite schema, FTS5, and queries.
- `src/sync.rs`: Oracle extraction, quarterly synchronization, and diffs.
- `docs/`: official sources and integration decisions.

## Conventions

- Use stable Rust, `cargo fmt`, and `cargo test` before delivering changes.
- Propagate errors with `Result`; the server must not use `panic!`, `unwrap()`, or
  `expect()` in production code paths.
- Keep stdout exclusively for MCP messages; send logs to stderr.
- Do not invent Oracle table names, column names, constraints, or URLs.
- All metadata must retain the release, module, and source URL when available.
- Queries must be limited to the active version unless a function explicitly
  specifies another version.
- Sanitize terms before building FTS5 queries.

## Verification

Run:

```sh
cargo fmt --all -- --check
cargo check
cargo test
```

Changes to the SQLite schema must include or update unit tests.
