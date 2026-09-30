# Oracle Fusion ERP Catalog MCP — Agent Instructions

## Scope

These instructions apply to the entire project.

## Architecture

- `src/main.rs`: MCP JSON-RPC 2.0 transport over stdin/stdout.
- `src/db.rs`: models, SQLite schema, FTS5, and queries.
- `src/paths.rs`: platform-specific SQLite data paths and legacy fallback.
- `src/install.rs`: global MCP configuration adapters for supported agents.
- `src/sync.rs`: Oracle extraction, quarterly synchronization, and diffs.
- `docs/`: official sources and integration decisions.

## CLI

- Package and binary name: `oracle-fusion-erp-catalog-mcp`.
- Running without arguments starts the MCP server.
- `oracle-fusion-erp-catalog-mcp sync --release RELEASE` synchronizes Oracle data.
- `oracle-fusion-erp-catalog-mcp install AGENT` registers the server globally in
  Cursor, Claude Code, Codex CLI, or OpenCode.
- `--help` and `--version` are available at the top level and for subcommands.

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
- Resolve the default SQLite path through the platform user-data directory;
  `ORACLE_MCP_DATABASE` takes precedence. The default file is `catalog.sqlite`;
  the legacy `oracle-erp-mcp.sqlite` fallback remains temporary while existing
  data is being processed.
- Keep MCP stdout reserved for JSON-RPC. CLI progress and diagnostics belong
  on stderr.
- Installer changes must preserve unrelated agent configuration and be
  idempotent. Use `--dry-run` when testing configuration changes.

## Verification

After every code or configuration change, run the checks and create a commit:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```

Changes to the SQLite schema must include or update unit tests.
Do not run `cargo build --release` unless explicitly requested.
