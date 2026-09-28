# Fuentes oficiales de tablas Oracle Fusion Cloud

## Propósito

Este documento define las URLs oficiales que leerá el extractor del servidor MCP
para construir el diccionario técnico de Oracle Fusion Cloud Financials y Supply
Chain & Manufacturing (SCM).

Oracle no publica una única URL estable con todas las tablas. Cada release tiene
un índice `Tables and Views` y páginas individuales para cada tabla o vista. El
extractor debe leer primero el índice, descubrir los enlaces y después obtener
cada página individual.

## Financials

### Índice por release

```text
https://docs.oracle.com/en/cloud/saas/financials/{release}/oedmf/index.html
```

Ejemplos:

- [Financials 26B](https://docs.oracle.com/en/cloud/saas/financials/26b/oedmf/index.html)
- [Financials 26C](https://docs.oracle.com/en/cloud/saas/financials/26c/oedmf/index.html)
- [Financials 26D](https://docs.oracle.com/en/cloud/saas/financials/26d/oedmf/index.html)

### Página de tabla o vista

El enlace individual descubierto en el índice sigue normalmente este patrón:

```text
https://docs.oracle.com/en/cloud/saas/financials/{release}/oedmf/{slug}-{id}.html
```

Ejemplos verificables en 26B:

- [GL_BALANCES](https://docs.oracle.com/en/cloud/saas/financials/26b/oedmf/glbalances-24959.html)
- [FA_ADDITIONS_B](https://docs.oracle.com/en/cloud/saas/financials/26b/oedmf/faadditionsb-6728.html)
- [XLA_DESCRIPTIONS_B](https://docs.oracle.com/en/cloud/saas/financials/26b/oedmf/xladescriptionsb-16402.html)
- [GL_SETS_OF_BOOKS](https://docs.oracle.com/en/cloud/saas/financials/26b/oedmf/glsetsofbooks-6695.html)
- [ZX_LINES_V](https://docs.oracle.com/en/cloud/saas/financials/26b/oedmf/zxlinesv-5233.html)

Cada página puede contener descripción, tipo de objeto, columnas, claves,
foreign keys, índices y, para vistas, la consulta SQL.

## Supply Chain & Manufacturing (SCM)

### Índice por release

```text
https://docs.oracle.com/en/cloud/saas/supply-chain-and-manufacturing/{release}/oedsc/index.html
```

Ejemplos:

- [SCM 26B overview](https://docs.oracle.com/en/cloud/saas/supply-chain-and-manufacturing/26b/oedsc/overview.html)
- [SCM 26C overview](https://docs.oracle.com/en/cloud/saas/supply-chain-and-manufacturing/26c/oedsc/overview.html)
- [SCM 25D index](https://docs.oracle.com/en/cloud/saas/supply-chain-and-manufacturing/25d/oedsc/index.html)

### Página de tabla o vista

El enlace individual descubierto en el índice sigue normalmente este patrón:

```text
https://docs.oracle.com/en/cloud/saas/supply-chain-and-manufacturing/{release}/oedsc/{slug}-{id}.html
```

Ejemplos verificables en 26B:

- [INV_ONHAND_SUP_SUMMARY_V](https://docs.oracle.com/en/cloud/saas/supply-chain-and-manufacturing/26b/oedsc/invonhandsupsummaryv-7500.html)
- [MSC_BOMS_V](https://docs.oracle.com/en/cloud/saas/supply-chain-and-manufacturing/26b/oedsc/mscbomsv-4644.html)

El índice SCM documenta tablas físicas, columnas, claves primarias e índices,
además de vistas y sus consultas SQL.

## Regla de descubrimiento para el extractor

1. Resolver `{release}` desde la configuración de sincronización, por ejemplo
   `26b`.
2. Descargar el índice de Financials y/o SCM.
3. Extraer únicamente enlaces bajo el mismo release y guía (`oedmf` u `oedsc`).
4. Descargar cada página enlazada y normalizar tablas, columnas, constraints e
   índices.
5. Guardar la URL original y el release junto con cada entidad para mantener
   trazabilidad.
6. Rechazar enlaces fuera de `docs.oracle.com` o fuera del prefijo del release.

## Fuente histórica OER

Oracle indica que el contenido anterior de Oracle Enterprise Repository fue
trasladado a Oracle Help Center, My Oracle Support o Setup and Maintenance:

- [Redirección oficial de OER](https://www.oracle.com/webfolder/technetwork/docs/HTML/oer-redirect.html)

Por tanto, `https://oracle.com[VERSION]/api/html/` no debe tratarse como una
URL operativa confirmada. El adaptador debe conservarla como configuración
opcional, pero usar los índices de Oracle Help Center como fuente por defecto.

## Fuentes que no deben confundirse

- Las guías `oedmf` y `oedsc` describen tablas y vistas de Fusion Cloud para
  consultas técnicas.
- Las guías de **Fusion ERP Analytics** y **Fusion SCM Analytics** describen
  modelos analíticos, no necesariamente las tablas físicas disponibles para
  BI Publisher.
- El extractor debe almacenar el tipo de objeto (`TABLE` o `VIEW`) y no
  presentar una vista como si fuera una tabla física.
