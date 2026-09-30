# Oracle ERP MCP

Rust MCP server for querying the Oracle Fusion Cloud Financials and SCM
technical dictionary through SQLite and FTS5.

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

The installer downloads a GitHub Release binary without sudo and places it in
`$HOME/.local/bin` (on this machine, `/Users/jer.ioet/.local/bin`). The
repository does not yet have a configured GitHub remote, so provide the future
owner and repository:

```sh
GITHUB_OWNER=YOUR_ORG GITHUB_REPO=YOUR_REPO \
  sh scripts/install.sh --version 0.1.0
```

You can also pass `--owner`, `--repo`, `--version`, `--install-dir`, or
`--asset`; run `sh scripts/install.sh --help` for details. Ensure
`$HOME/.local/bin` is on `PATH`, then register the server with an agent:

```sh
oracle-fusion-erp-catalog-mcp install all \
  --binary "$HOME/.local/bin/oracle-fusion-erp-catalog-mcp" \
  --database "$PWD/oracle-fusion-erp-catalog-mcp.sqlite"
```

Replace `all` with `cursor`, `claude-code`, `codex`, or `opencode`. Use
`--dry-run` to preview configuration changes.

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
agent using the `install` command above, or try the protocol directly:

```sh
printf '%s\n' \
  '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}' \
  '{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}' \
  | oracle-fusion-erp-catalog-mcp
```

Available tools are `list_modules_and_tables`, `search_table_structure`, and
`suggest_joins`.

## Setup dev environment

Install stable Rust, then build and test from a checkout. Cargo commands are
for development only; operational and user commands above use the installed
binary.

```sh
cargo build --release
cargo fmt --all -- --check
cargo check
cargo test
```
