# Oracle Fusion ERP Catalog MCP — Agent Instructions

## Scope

These instructions apply to the entire project.

## Architecture

- `src/main.rs`: MCP JSON-RPC 2.0 transport over stdin/stdout.
- `src/db.rs`: models, SQLite schema, FTS5, and queries.
- `src/paths.rs`: platform-specific SQLite data paths.
- `src/install.rs`: global MCP configuration adapters for supported agents.
- `src/sync.rs`: Oracle extraction, quarterly synchronization, batching, and diffs.
- `src/sync_cache.rs`: compressed parsed-catalog cache for release/module retries.
- `src/catalog.rs`: download, checksum validation, decompression, and installation
  of published SQLite catalogs.
- `docs/`: official sources and integration decisions.

## CLI

- Package and binary name: `oracle-fusion-erp-catalog-mcp`.
- Running without arguments starts the MCP server.
- `oracle-fusion-erp-catalog-mcp sync --release RELEASE` synchronizes Oracle data.
- `oracle-fusion-erp-catalog-mcp catalog install --release RELEASE` installs a
  pre-generated catalog from GitHub Releases without synchronizing Oracle.
- `oracle-fusion-erp-catalog-mcp install AGENT` registers the server globally in
  Cursor, Claude Code, Codex CLI, or OpenCode.
- `scripts/release.sh --bump patch|minor|major` updates Cargo, commits, tags,
  and pushes a release; the tag triggers the GitHub Actions release workflow.
- `make catalog-release RELEASE=26B DATABASE=PATH` publishes a locally generated
  catalog as the separate `catalog-26B` GitHub Release. Catalog generation is
  intentionally local and must not run during binary installation.
- The `Makefile` provides shortcuts for verification and release automation;
  use the binary subcommands directly for synchronization and installation.
- `--help` and `--version` are available at the top level and for subcommands.

## Git workflow

- Never work directly on `main` or the production branch.
- Create a focused branch for every change, using a descriptive name such as
  `feat/platform-db-paths` or `fix/mcp-config`.
- Review the complete diff before merging, including tests, configuration, and
  documentation changes.
- Run the required verification commands on the branch before requesting a
  merge.
- Merge to the production branch only after review and successful verification.
- Keep commits focused and use Conventional Commit messages.

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
  `ORACLE_MCP_DATABASE` takes precedence. The default file is `catalog.sqlite`.
- Store parsed synchronization caches under the application data directory in
  `sync-cache/`; invalidate them when release, module, source URL, or cache
  format changes. `ORACLE_MCP_SYNC_PARALLELISM` controls extraction concurrency
  and defaults to `2`.
- `--module all` includes Financials, SCM, and HCM. Financials uses the
  versioned `oedmf` guide, SCM uses the versioned `oedsc` guide, and HCM uses
  the cumulative `human-resources/oedmh` guide without a release segment.
- Binary update and installation flows must ignore `catalog-*` releases and
  select only semver `v*` releases.
- Keep MCP stdout reserved for JSON-RPC. CLI progress and diagnostics belong
  on stderr.
- Installer changes must preserve unrelated agent configuration and be
  idempotent. Use `--dry-run` when testing configuration changes.
- Release scripts must not build release binaries locally; GitHub Actions owns
  version updates, tags, platform builds, checksums, and publication.

## Verification

After every code or configuration change, run the checks and create a commit:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```

Changes to the SQLite schema must include or update unit tests.
Do not run `cargo build --release` unless explicitly requested.
