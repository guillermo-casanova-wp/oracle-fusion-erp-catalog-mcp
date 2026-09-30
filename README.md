# Oracle ERP MCP

Rust MCP server for querying the Oracle Fusion Cloud Financials and SCM
technical dictionary through SQLite and FTS5.

## Status

Includes:

- Versioned schema for tables, columns, references, and indexes.
- Cloning between releases and selection of an active version.
- Exact and lexical search with FTS5.
- Configurable extractor for HTML, JSON, and XML indexes from Oracle Help Center.
- Delta calculation between two releases.
- JSON-RPC 2.0 transport over stdin/stdout and the following tools:
  `list_modules_and_tables`, `search_table_structure`, and `suggest_joins`.

The official sources and their patterns are documented in
[`docs/oracle-table-sources.md`](docs/oracle-table-sources.md).

## Disclaimer

Oracle and Oracle Fusion are trademarks of Oracle Corporation. This project
is not affiliated with, endorsed by, or sponsored by Oracle Corporation.

The project source code is distributed under the MIT License. Content and
metadata retrieved from Oracle Help Center remain subject to their respective
copyright and usage terms.

## Running

Requires stable Rust (`cargo` and `rustc`) to be installed.

```sh
cargo run --release
```

The SQLite path is configured with `ORACLE_MCP_DATABASE`; the default is
`oracle-fusion-erp-catalog-mcp.sqlite`.

Logs are written to stderr. stdout is reserved for MCP messages.

## Agent installation

Register the server globally in Cursor, Claude Code, Codex CLI, or OpenCode:

```sh
cargo install --path .
oracle-fusion-erp-catalog-mcp install all --binary "$(command -v oracle-fusion-erp-catalog-mcp)" --database "$PWD/oracle-fusion-erp-catalog-mcp.sqlite"
```

Replace `all` with `cursor`, `claude-code`, `codex`, or `opencode` to select one
agent. Use `--dry-run` to preview changes. Installation creates missing parent
folders, preserves unrelated configuration, updates the server entry
idempotently, and atomically replaces changed files. The server is registered
with the `ORACLE_MCP_DATABASE` environment variable.

## Synchronization

Download and store a release from the Oracle Help Center:

```sh
cargo run -- sync --release 26B
```

By default, the command synchronizes both Financials and SCM into one release
and activates it. To synchronize only one module, replace an existing release,
or keep the release inactive:

```sh
cargo run -- sync --release 26B --module financials
cargo run -- sync --release 26B --no-activate
cargo run -- sync --release 26B --replace
```

If the active release is `26B`, syncing `26C` or any later release creates and
activates the newer release, then removes the previous active release after a
successful synchronization. A module missing from an existing release is
merged into that release, so SCM `26B` can be completed with Financials `26B`
without reloading SCM.

The CLI also provides:

```sh
target/release/oracle-fusion-erp-catalog-mcp --help
target/release/oracle-fusion-erp-catalog-mcp --version
target/release/oracle-fusion-erp-catalog-mcp sync --help
target/release/oracle-fusion-erp-catalog-mcp sync --version
```

## Initialization example

```sh
printf '%s\n' \
  '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}' \
  '{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}' \
  | cargo run --quiet
```

## Verification

```sh
cargo fmt --all -- --check
cargo check
cargo test
```
