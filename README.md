# Oracle ERP MCP

Servidor MCP en Rust para consultar el diccionario técnico de Oracle Fusion
Cloud Financials y SCM mediante SQLite y FTS5.

## Estado

Incluye:

- Esquema versionado de tablas, columnas, referencias e índices.
- Clonación entre releases y selección de una versión activa.
- Búsqueda exacta y léxica con FTS5.
- Extractor configurable para índices HTML, JSON y XML del Oracle Help Center.
- Cálculo de delta entre dos releases.
- Transporte JSON-RPC 2.0 por stdin/stdout y las herramientas:
  `listar_modulos_y_tablas`, `buscar_estructura_tabla` y `sugerir_joins`.

Las fuentes oficiales y sus patrones están en
[`docs/oracle-table-sources.md`](docs/oracle-table-sources.md).

## Ejecutar

Requiere Rust estable (`cargo` y `rustc`) instalado.

```sh
cargo run --release
```

La ruta de SQLite se configura con `ORACLE_MCP_DATABASE`; por defecto es
`oracle-erp-mcp.sqlite`.

Los logs se escriben en stderr. stdout queda reservado para mensajes MCP.

## Ejemplo de inicialización

```sh
printf '%s\n' \
  '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}' \
  '{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}' \
  | cargo run --quiet
```

## Verificación

```sh
cargo fmt --all -- --check
cargo check
cargo test
```
