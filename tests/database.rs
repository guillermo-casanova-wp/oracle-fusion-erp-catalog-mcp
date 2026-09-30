use oracle_fusion_erp_catalog_mcp::db::{CatalogColumn, CatalogReference, CatalogTable, Database};

fn table(name: &str, description: &str) -> CatalogTable {
    CatalogTable {
        module: "FINANCIALS".to_owned(),
        table_name: name.to_owned(),
        description: Some(description.to_owned()),
        columns: vec![CatalogColumn {
            column_name: "ID".to_owned(),
            data_type: "NUMBER".to_owned(),
            length: Some(18),
            nullable: false,
            description: Some("Identifier".to_owned()),
        }],
        ..CatalogTable::default()
    }
}

#[test]
fn active_version_queries_return_structure_and_search_results() {
    let db = Database::in_memory().expect("database");
    let version_id = db.create_version("26B", true).expect("version");
    db.upsert_catalog_table(
        version_id,
        &table("AP_INVOICES_ALL", "Accounts payable invoice headers"),
    )
    .expect("table");

    let structure = db
        .table_structure("ap_invoices_all")
        .expect("structure query")
        .expect("structure");
    assert_eq!(structure.table.table_name, "AP_INVOICES_ALL");
    assert_eq!(structure.columns[0].column_name, "ID");

    let matches = db.search_tables("invoices", 10).expect("search query");
    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].table_name, "AP_INVOICES_ALL");
}

#[test]
fn queries_are_limited_to_the_active_release() {
    let db = Database::in_memory().expect("database");
    let old_id = db.create_version("26B", false).expect("old version");
    db.upsert_catalog_table(old_id, &table("OLD_TABLE", "Old release"))
        .expect("old table");
    let active_id = db.create_version("26C", true).expect("active version");
    db.upsert_catalog_table(active_id, &table("CURRENT_TABLE", "Current release"))
        .expect("current table");

    let tables = db.list_modules_and_tables(None).expect("list query");
    assert_eq!(tables.len(), 1);
    assert_eq!(tables[0].table_name, "CURRENT_TABLE");
    assert!(db
        .table_structure("OLD_TABLE")
        .expect("structure query")
        .is_none());
}

#[test]
fn discovers_columns_relationships_and_releases() {
    let db = Database::in_memory().expect("database");
    let version_id = db.create_version("26B", true).expect("version");
    db.upsert_catalog_table(
        version_id,
        &table("PO_HEADERS_ALL", "Purchase order headers"),
    )
    .expect("parent table");

    let mut lines = table("PO_LINES_ALL", "Purchase order lines");
    lines.columns.push(CatalogColumn {
        column_name: "HEADER_ID".to_owned(),
        data_type: "NUMBER".to_owned(),
        length: None,
        nullable: false,
        description: Some("Purchase order header identifier".to_owned()),
    });
    lines.references.push(CatalogReference {
        target_table: "PO_HEADERS_ALL".to_owned(),
        source_column: "HEADER_ID".to_owned(),
        target_column: "ID".to_owned(),
        constraint_name: Some("PO_LINES_HEADER_FK".to_owned()),
    });
    db.upsert_catalog_table(version_id, &lines)
        .expect("child table");

    let by_column = db
        .find_tables_by_column("header_id", None, 10)
        .expect("column lookup");
    assert_eq!(by_column[0].table_name, "PO_LINES_ALL");

    let column_matches = db
        .search_columns("header identifier", None, 10)
        .expect("column search");
    assert_eq!(column_matches[0].column.column_name, "HEADER_ID");

    let related = db
        .find_related_tables("PO_HEADERS_ALL", 1, 10)
        .expect("related tables");
    assert_eq!(related[0].table.table_name, "PO_LINES_ALL");
    assert_eq!(related[0].depth, 1);

    let releases = db.list_versions().expect("release listing");
    assert_eq!(releases[0].version.release_code, "26B");
    assert!(releases[0].version.active);
    assert_eq!(releases[0].table_count, 2);
    assert_eq!(releases[0].modules, vec!["FINANCIALS"]);
}
