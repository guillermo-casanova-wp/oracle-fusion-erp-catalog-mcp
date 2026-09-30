use oracle_fusion_erp_catalog_mcp::{
    db::{CatalogTable, Database},
    sync::synchronize,
};

fn table(module: &str, name: &str) -> CatalogTable {
    CatalogTable {
        module: module.to_owned(),
        table_name: name.to_owned(),
        ..CatalogTable::default()
    }
}

#[test]
fn synchronization_merges_a_missing_module_into_an_existing_release() {
    let db = Database::in_memory().expect("database");
    let version_id = db.create_version("26B", true).expect("version");
    db.upsert_catalog_table(version_id, &table("SCM", "WSH_DELIVERY_DETAILS"))
        .expect("SCM table");

    synchronize(
        &db,
        "26B",
        vec![table("FINANCIALS", "AP_INVOICES_ALL")],
        true,
    )
    .expect("module merge");

    let modules = db.modules_for_version(version_id).expect("modules");
    assert!(modules.contains("SCM"));
    assert!(modules.contains("FINANCIALS"));
}

#[test]
fn synchronization_replaces_active_release_only_after_upgrade() {
    let db = Database::in_memory().expect("database");
    let version_id = db.create_version("26B", true).expect("version");
    db.upsert_catalog_table(version_id, &table("SCM", "OLD_TABLE"))
        .expect("old table");

    synchronize(&db, "26C", vec![table("SCM", "NEW_TABLE")], true).expect("upgrade");

    assert!(db.version_by_release("26B").expect("old release").is_none());
    assert!(db.version_by_release("26C").expect("new release").is_some());
}
