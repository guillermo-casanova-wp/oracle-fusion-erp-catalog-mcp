# Oracle ERP MCP — Agent Instructions

## Scope

Estas instrucciones aplican a todo el proyecto.

## Arquitectura

- `src/main.rs`: transporte MCP JSON-RPC 2.0 por stdin/stdout.
- `src/db.rs`: modelos, esquema SQLite, FTS5 y consultas.
- `src/sync.rs`: extracción Oracle, sincronización trimestral y diff.
- `docs/`: fuentes oficiales y decisiones de integración.

## Convenciones

- Usar Rust estable, `cargo fmt` y `cargo test` antes de entregar cambios.
- Propagar errores con `Result`; el servidor no debe usar `panic!`, `unwrap()` ni
  `expect()` en el flujo de producción.
- Mantener stdout exclusivamente para mensajes MCP; enviar logs a stderr.
- No inventar nombres de tablas, columnas, constraints o URLs Oracle.
- Toda metadata debe conservar release, módulo y URL de origen cuando exista.
- Las consultas deben limitarse a la versión activa salvo que una función
  indique explícitamente otra versión.
- Sanitizar términos antes de construir consultas FTS5.

## Verificación

Ejecutar:

```sh
cargo fmt --all -- --check
cargo check
cargo test
```

Los cambios en el esquema SQLite deben incluir o actualizar pruebas unitarias.
