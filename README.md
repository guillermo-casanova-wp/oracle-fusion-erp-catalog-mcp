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
default database location.

## Sync

Synchronize both Financials and SCM from the Oracle Help Center:

```sh
oracle-fusion-erp-catalog-mcp sync --release 26B
```

Use `--module financials|scm`, `--no-activate`, or `--replace` as needed.
Synchronization activates newer releases and removes the previous active
release after success; a missing module can be merged into an existing release.
The database path is controlled by `ORACLE_MCP_DATABASE`.

## Release

Run the complete release workflow from an up-to-date `main` branch:

```sh
scripts/release.sh 0.2.0
```

The script validates the version, runs formatting, Clippy, and tests, triggers
the GitHub Actions release workflow, waits for completion, and prints the
published release URL. Preview the checks and workflow trigger without
publishing with:

```sh
scripts/release.sh 0.2.0 --dry-run
```

The GitHub workflow updates the package version, creates the tag, builds the
platform binaries, generates checksums, and publishes the GitHub release.

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
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test
cargo install cargo-llvm-cov
cargo llvm-cov --workspace --all-features --lcov --output-path lcov.info
```

GitHub Actions runs the same formatting, Clippy, test, and coverage checks on
pushes to `main` and pull requests. The generated `lcov.info` report is
available as the `rust-coverage-lcov` workflow artifact.
