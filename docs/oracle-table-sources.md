# Official Oracle Fusion Cloud Table Sources

## Purpose

This document defines the official URLs that the MCP server extractor reads to
build the Oracle Fusion Cloud Financials and Supply Chain & Manufacturing (SCM)
technical dictionary.

Oracle does not publish a single stable URL containing all tables. Each release
has a `Tables and Views` index and individual pages for each table or view. The
extractor must read the index first, discover the links, and then fetch each
individual page.

## Financials

### Per-release index

```text
https://docs.oracle.com/en/cloud/saas/financials/{release}/oedmf/index.html
```

Examples:

- [Financials 26B](https://docs.oracle.com/en/cloud/saas/financials/26b/oedmf/index.html)
- [Financials 26C](https://docs.oracle.com/en/cloud/saas/financials/26c/oedmf/index.html)
- [Financials 26D](https://docs.oracle.com/en/cloud/saas/financials/26d/oedmf/index.html)

### Table or view page

The individual link discovered in the index usually follows this pattern:

```text
https://docs.oracle.com/en/cloud/saas/financials/{release}/oedmf/{slug}-{id}.html
```

Examples verified in 26B:

- [GL_BALANCES](https://docs.oracle.com/en/cloud/saas/financials/26b/oedmf/glbalances-24959.html)
- [FA_ADDITIONS_B](https://docs.oracle.com/en/cloud/saas/financials/26b/oedmf/faadditionsb-6728.html)
- [XLA_DESCRIPTIONS_B](https://docs.oracle.com/en/cloud/saas/financials/26b/oedmf/xladescriptionsb-16402.html)
- [GL_SETS_OF_BOOKS](https://docs.oracle.com/en/cloud/saas/financials/26b/oedmf/glsetsofbooks-6695.html)
- [ZX_LINES_V](https://docs.oracle.com/en/cloud/saas/financials/26b/oedmf/zxlinesv-5233.html)

Each page may contain a description, object type, columns, keys, foreign keys,
indexes, and, for views, the SQL query.

## Supply Chain & Manufacturing (SCM)

### Per-release index

```text
https://docs.oracle.com/en/cloud/saas/supply-chain-and-manufacturing/{release}/oedsc/index.html
```

Examples:

- [SCM 26B overview](https://docs.oracle.com/en/cloud/saas/supply-chain-and-manufacturing/26b/oedsc/overview.html)
- [SCM 26C overview](https://docs.oracle.com/en/cloud/saas/supply-chain-and-manufacturing/26c/oedsc/overview.html)
- [SCM 25D index](https://docs.oracle.com/en/cloud/saas/supply-chain-and-manufacturing/25d/oedsc/index.html)

### Table or view page

The individual link discovered in the index usually follows this pattern:

```text
https://docs.oracle.com/en/cloud/saas/supply-chain-and-manufacturing/{release}/oedsc/{slug}-{id}.html
```

Examples verified in 26B:

- [INV_ONHAND_SUP_SUMMARY_V](https://docs.oracle.com/en/cloud/saas/supply-chain-and-manufacturing/26b/oedsc/invonhandsupsummaryv-7500.html)
- [MSC_BOMS_V](https://docs.oracle.com/en/cloud/saas/supply-chain-and-manufacturing/26b/oedsc/mscbomsv-4644.html)

The SCM index documents physical tables, columns, primary keys, and indexes,
as well as views and their SQL queries.

## Extractor discovery rule

1. Resolve `{release}` from the synchronization configuration, for example
   `26b`.
2. Download the Financials and/or SCM index.
3. Extract only links under the same release and guide (`oedmf` or `oedsc`).
4. Download each linked page and normalize tables, columns, constraints, and
   indexes.
5. Store the original URL and release with each entity to maintain
   traceability.
6. Reject links outside `docs.oracle.com` or outside the release prefix.

## Historical OER source

Oracle states that the former Oracle Enterprise Repository content was moved to
Oracle Help Center, My Oracle Support, or Setup and Maintenance:

- [Official OER redirect](https://www.oracle.com/webfolder/technetwork/docs/HTML/oer-redirect.html)

Therefore, `https://oracle.com[VERSION]/api/html/` must not be treated as a
confirmed operational URL. The adapter should retain it as optional
configuration, but use the Oracle Help Center indexes as the default source.

## Sources that must not be confused

- The `oedmf` and `oedsc` guides describe Fusion Cloud tables and views for
  technical queries.
- The **Fusion ERP Analytics** and **Fusion SCM Analytics** guides describe
  analytical models, not necessarily the physical tables available for
  BI Publisher.
- The extractor must store the object type (`TABLE` or `VIEW`) and must not
  present a view as a physical table.
