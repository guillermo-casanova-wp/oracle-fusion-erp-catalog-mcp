# Oracle ERP MCP

Rust MCP server for querying the Oracle Fusion Cloud Financials and SCM
technical dictionary through SQLite and FTS5.

## Estado

Includes:

- Versioned schema for tables, columns, references, and indexes.
- Cloning between releases and selection of an active version.
- Exact and lexical search with FTS5.
- Configurable extractor for HTML, JSON, and XML indexes from Oracle Help Center.
- Delta calculation between two releases.
- JSON-RPC 2.0 transport over stdin/stdout and the following tools:
  `listar_modulos_y_tablas`, `buscar_estructura_tabla` y `sugerir_joins`.

The official sources and their patterns are documented in
[`docs/oracle-table-sources.md`](docs/oracle-table-sources.md).

## Running

Requires stable Rust (`cargo` and `rustc`) to be installed.

```sh
cargo run --release
```

The SQLite path is configured with `ORACLE_MCP_DATABASE`; the default is
`oracle-erp-mcp.sqlite`.

Logs are written to stderr. stdout is reserved for MCP messages.

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
