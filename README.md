# Oracle Fusion ERP Catalog MCP

Oracle Fusion ERP Catalog MCP is a Rust MCP server for querying the Oracle
Fusion Cloud Financials and SCM technical dictionary through SQLite and FTS5.

## Why

Oracle ERP schemas are large, versioned, and difficult to search from an agent.
This server stores the published table metadata locally, keeps releases
separate, and exposes exact lookup, lexical search, structure, and join tools
over MCP. Official source patterns are documented in
[`docs/oracle-table-sources.md`](docs/oracle-table-sources.md).

## Disclaimer

Oracle and Oracle Fusion are trademarks of Oracle Corporation. This project
is not affiliated with, endorsed by, or sponsored by Oracle Corporation.

The source code is MIT licensed; content and metadata retrieved from Oracle Help
Center remain subject to Oracle's terms.

## Install

Install the latest release:

```sh
curl -fsSL \
  https://raw.githubusercontent.com/thegreatyamori/oracle-fusion-erp-catalog-mcp/main/scripts/install.sh \
  | sh
```

If you already installed it, update the binary with:

```sh
oracle-fusion-erp-catalog-mcp update
```

## Set up your agent

Run the command for the agent you use, then restart that agent:

| Agent | Setup |
| --- | --- |
| Cursor | `oracle-fusion-erp-catalog-mcp install cursor --binary "$HOME/.local/bin/oracle-fusion-erp-catalog-mcp"` |
| Claude Code | `oracle-fusion-erp-catalog-mcp install claude-code --binary "$HOME/.local/bin/oracle-fusion-erp-catalog-mcp"` |
| Codex CLI | `oracle-fusion-erp-catalog-mcp install codex --binary "$HOME/.local/bin/oracle-fusion-erp-catalog-mcp"` |
| OpenCode | `oracle-fusion-erp-catalog-mcp install opencode --binary "$HOME/.local/bin/oracle-fusion-erp-catalog-mcp"` |
| All agents | `oracle-fusion-erp-catalog-mcp install all --binary "$HOME/.local/bin/oracle-fusion-erp-catalog-mcp"` |

Use `--dry-run` to preview changes. Set `ORACLE_MCP_DATABASE` to override the
default database location. Without an override, the database is stored as
`catalog.sqlite` in the platform user-data directory:

- macOS: `~/Library/Application Support/oracle-fusion-erp-catalog-mcp/catalog.sqlite`
- Linux: `${XDG_DATA_HOME:-~/.local/share}/oracle-fusion-erp-catalog-mcp/catalog.sqlite`
- Windows: `%LOCALAPPDATA%\oracle-fusion-erp-catalog-mcp\catalog.sqlite`

## Sync

Synchronize both Financials and SCM from the Oracle Help Center:

```sh
oracle-fusion-erp-catalog-mcp sync --release 26B
```

Use `--module financials|scm`, `--no-activate`, or `--replace` as needed.
Synchronization activates newer releases and removes the previous active
release after success; a missing module can be merged into an existing release.
The database path is controlled by `ORACLE_MCP_DATABASE`.

## MCP usage

With no subcommand, the installed binary speaks JSON-RPC 2.0 over stdin/stdout.
Logs go to stderr, while stdout is reserved for MCP messages. Configure your
agent using the setup command above, or try the protocol directly:

```sh
printf '%s\n' \
  '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}' \
  '{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}' \
  | oracle-fusion-erp-catalog-mcp
```

Available tools:

| Tool | Description |
| --- | --- |
| `list_modules_and_tables` | Lista las tablas de la release activa, opcionalmente por módulo. |
| `search_table_structure` | Busca una tabla y devuelve su estructura técnica. |
| `suggest_joins` | Devuelve relaciones directas entre dos tablas. |
| `find_tables_by_column` | Encuentra tablas que contienen una columna. |
| `search_columns` | Busca columnas por nombre o descripción. |
| `find_related_tables` | Encuentra tablas relacionadas mediante claves foráneas. |
| `list_releases` | Lista las releases sincronizadas e identifica la activa. |

## Setup dev environment

Install stable Rust, then verify changes from a checkout. Operational and user
commands above use the installed binary.

```sh
make verify
make compile
make check
make test
```

For optional local coverage, install `cargo-llvm-cov` and run:

```sh
cargo llvm-cov --workspace --all-features --lcov --output-path lcov.info
```

