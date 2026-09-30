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

Install the latest release without Rust or `sudo`:

```sh
curl -fsSL \
  https://raw.githubusercontent.com/thegreatyamori/oracle-fusion-erp-catalog-mcp/main/scripts/install.sh \
  | sh
```

The installer places the binary in `$HOME/.local/bin`, detects the operating
system and architecture, and asks where to register the MCP:

1. Cursor
2. Claude Code
3. Codex CLI
4. OpenCode
5. All agents
6. Binary only

For a non-interactive installation, pass the agent choice through `sh -s`:

```sh
curl -fsSL \
  https://raw.githubusercontent.com/thegreatyamori/oracle-fusion-erp-catalog-mcp/main/scripts/install.sh \
  | sh -s -- --agent cursor
```

Use `--agent none` to install only the binary. Other useful options include
`--version VERSION`, `--database PATH`, `--install-dir DIRECTORY`, and
`--asset NAME`. Forks can override the release source with `--owner` and
`--repo`.

The SQLite path is configured with `ORACLE_MCP_DATABASE`; the installer defaults
to `$PWD/oracle-fusion-erp-catalog-mcp.sqlite`.

Logs are written to stderr. stdout is reserved for MCP messages.

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

## Release packages

Releases are built by GitHub Actions for macOS and Linux on x86_64 and
aarch64. To publish a new version, open the `Release` workflow in GitHub
Actions, choose **Run workflow**, and enter a version such as `0.2.0`.

The workflow updates `Cargo.toml` and `Cargo.lock`, commits the release
version to `main`, creates the matching `v0.2.0` tag, builds the four binaries,
and publishes them with a `SHA256SUMS` file. Existing `v*` tags are also
supported, but their version must already match `Cargo.toml`.

The generated assets use the names consumed by the installer:

```text
oracle-fusion-erp-catalog-mcp-macos-x86_64
oracle-fusion-erp-catalog-mcp-macos-aarch64
oracle-fusion-erp-catalog-mcp-linux-x86_64
oracle-fusion-erp-catalog-mcp-linux-aarch64
```

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
