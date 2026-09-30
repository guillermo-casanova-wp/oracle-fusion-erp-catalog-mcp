use crate::paths::ensure_parent_directory;
use rusqlite::{params, Connection, OptionalExtension, Result as SqlResult};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, path::Path};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Version {
    pub id: i64,
    pub release_code: String,
    pub synced_at: String,
    pub active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TableRecord {
    pub id: i64,
    pub version_id: i64,
    pub module: String,
    pub table_name: String,
    pub description: Option<String>,
    pub source_url: Option<String>,
    pub object_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ColumnRecord {
    pub id: i64,
    pub table_id: i64,
    pub column_name: String,
    pub data_type: String,
    pub length: Option<i64>,
    pub nullable: bool,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ReferenceRecord {
    pub id: i64,
    pub source_table_id: i64,
    pub source_column: String,
    pub target_table_id: i64,
    pub target_column: Option<String>,
    pub constraint_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct IndexRecord {
    pub id: i64,
    pub table_id: i64,
    pub index_name: String,
    pub indexed_columns: String,
    pub is_unique: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableStructure {
    pub table: TableRecord,
    pub columns: Vec<ColumnRecord>,
    pub outgoing_references: Vec<ReferenceRecord>,
    pub incoming_references: Vec<ReferenceRecord>,
    pub indexes: Vec<IndexRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnSearchResult {
    pub table: TableRecord,
    pub column: ColumnRecord,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelatedTable {
    pub table: TableRecord,
    pub source_table: String,
    pub source_column: String,
    pub target_table: String,
    pub target_column: Option<String>,
    pub constraint_name: Option<String>,
    pub depth: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReleaseSummary {
    pub version: Version,
    pub modules: Vec<String>,
    pub table_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CatalogTable {
    pub module: String,
    pub table_name: String,
    pub description: Option<String>,
    pub source_url: Option<String>,
    pub object_type: Option<String>,
    pub columns: Vec<CatalogColumn>,
    pub references: Vec<CatalogReference>,
    pub indexes: Vec<CatalogIndex>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CatalogColumn {
    pub column_name: String,
    pub data_type: String,
    pub length: Option<i64>,
    pub nullable: bool,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CatalogReference {
    pub target_table: String,
    pub source_column: String,
    pub target_column: Option<String>,
    pub constraint_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CatalogIndex {
    pub index_name: String,
    pub indexed_columns: Vec<String>,
    pub is_unique: bool,
}

pub struct Database {
    connection: Connection,
}

impl Database {
    pub fn open(path: impl AsRef<Path>) -> SqlResult<Self> {
        let path = path.as_ref();
        if path != Path::new(":memory:") {
            ensure_parent_directory(path)
                .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))?;
        }
        let connection = Connection::open(path)?;
        connection.execute_batch("PRAGMA foreign_keys = ON;")?;
        let db = Self { connection };
        db.migrate()?;
        Ok(db)
    }

    pub fn in_memory() -> SqlResult<Self> {
        Self::open(":memory:")
    }

    fn migrate_legacy_schema(&self) -> SqlResult<bool> {
        let mut migrated = false;
        for (old_name, new_name) in [
            ("tabla_versiones", "versions"),
            ("tablas", "tables"),
            ("columnas", "columns"),
            ("referencias", "foreign_key_references"),
            ("indices", "indexes"),
        ] {
            let old_exists = self.table_exists(old_name)?;
            let new_exists = self.table_exists(new_name)?;
            if old_exists && !new_exists {
                self.connection
                    .execute_batch(&format!("ALTER TABLE {old_name} RENAME TO {new_name};"))?;
                migrated = true;
            }
        }

        for (table, old_name, new_name) in [
            ("versions", "fecha_sincronizacion", "synced_at"),
            ("versions", "activo_bool", "active_bool"),
            ("tables", "modulo", "module"),
            ("tables", "nombre_tabla", "table_name"),
            ("tables", "descripcion", "description"),
            ("columns", "tabla_id", "table_id"),
            ("columns", "nombre_columna", "column_name"),
            ("columns", "tipo_datos", "data_type"),
            ("columns", "longitud", "length"),
            ("columns", "descripcion", "description"),
            (
                "foreign_key_references",
                "tabla_origen_id",
                "source_table_id",
            ),
            ("foreign_key_references", "columna_origen", "source_column"),
            (
                "foreign_key_references",
                "tabla_destino_id",
                "target_table_id",
            ),
            ("foreign_key_references", "columna_destino", "target_column"),
            (
                "foreign_key_references",
                "nombre_constraint",
                "constraint_name",
            ),
            ("indexes", "tabla_id", "table_id"),
            ("indexes", "nombre_indice", "index_name"),
            ("indexes", "columnas_indexadas", "indexed_columns"),
            ("indexes", "es_unico", "is_unique"),
        ] {
            if self.column_exists(table, old_name)? && !self.column_exists(table, new_name)? {
                self.connection.execute_batch(&format!(
                    "ALTER TABLE {table} RENAME COLUMN {old_name} TO {new_name};"
                ))?;
                migrated = true;
            }
        }

        if self.table_exists("tablas_fts")? {
            self.connection.execute_batch("DROP TABLE tablas_fts;")?;
            migrated = true;
        }
        if migrated {
            self.connection.execute_batch(
                "DROP TABLE IF EXISTS tables_fts;
                 DROP INDEX IF EXISTS idx_tablas_version_modulo;
                 DROP INDEX IF EXISTS idx_columnas_tabla;",
            )?;
        }
        Ok(migrated)
    }

    fn table_exists(&self, name: &str) -> SqlResult<bool> {
        self.connection.query_row(
            "SELECT EXISTS(
                SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1
            )",
            params![name],
            |row| row.get(0),
        )
    }

    fn column_exists(&self, table: &str, name: &str) -> SqlResult<bool> {
        let mut statement = self
            .connection
            .prepare(&format!("PRAGMA table_info({table})"))?;
        let mut rows = statement.query([])?;
        while let Some(row) = rows.next()? {
            if row.get::<_, String>(1)? == name {
                return Ok(true);
            }
        }
        Ok(false)
    }

    fn column_is_not_null(&self, table: &str, name: &str) -> SqlResult<bool> {
        let mut statement = self
            .connection
            .prepare(&format!("PRAGMA table_info({table})"))?;
        let mut rows = statement.query([])?;
        while let Some(row) = rows.next()? {
            if row.get::<_, String>(1)? == name {
                return row.get(3);
            }
        }
        Ok(false)
    }

    fn migrate_nullable_target_column(&self) -> SqlResult<bool> {
        if !self.table_exists("foreign_key_references")?
            || !self.column_is_not_null("foreign_key_references", "target_column")?
        {
            return Ok(false);
        }
        self.connection.execute_batch(
            "BEGIN;
             ALTER TABLE foreign_key_references RENAME TO foreign_key_references_legacy;
             CREATE TABLE foreign_key_references (
                 id INTEGER PRIMARY KEY,
                 source_table_id INTEGER NOT NULL REFERENCES tables(id) ON DELETE CASCADE,
                 source_column TEXT NOT NULL,
                 target_table_id INTEGER NOT NULL REFERENCES tables(id) ON DELETE CASCADE,
                 target_column TEXT,
                 constraint_name TEXT,
                 UNIQUE(source_table_id, source_column, target_table_id, target_column)
             );
             INSERT INTO foreign_key_references
                 (id, source_table_id, source_column, target_table_id, target_column, constraint_name)
             SELECT id, source_table_id, source_column, target_table_id, target_column, constraint_name
             FROM foreign_key_references_legacy;
             DROP TABLE foreign_key_references_legacy;
             COMMIT;",
        )?;
        Ok(true)
    }

    fn migrate(&self) -> SqlResult<()> {
        let rebuild_fts = self.migrate_legacy_schema()?;
        self.migrate_nullable_target_column()?;
        self.connection.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS versions (
                id INTEGER PRIMARY KEY,
                release_code TEXT NOT NULL UNIQUE,
                synced_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                active_bool INTEGER NOT NULL DEFAULT 0 CHECK (active_bool IN (0, 1))
            );
            CREATE TABLE IF NOT EXISTS tables (
                id INTEGER PRIMARY KEY,
                version_id INTEGER NOT NULL REFERENCES versions(id) ON DELETE CASCADE,
                module TEXT NOT NULL,
                table_name TEXT NOT NULL,
                description TEXT,
                source_url TEXT,
                object_type TEXT,
                UNIQUE(version_id, table_name)
            );
            CREATE TABLE IF NOT EXISTS columns (
                id INTEGER PRIMARY KEY,
                table_id INTEGER NOT NULL REFERENCES tables(id) ON DELETE CASCADE,
                column_name TEXT NOT NULL,
                data_type TEXT NOT NULL,
                length INTEGER,
                nullable INTEGER NOT NULL DEFAULT 1 CHECK (nullable IN (0, 1)),
                description TEXT,
                UNIQUE(table_id, column_name)
            );
            CREATE TABLE IF NOT EXISTS foreign_key_references (
                id INTEGER PRIMARY KEY,
                source_table_id INTEGER NOT NULL REFERENCES tables(id) ON DELETE CASCADE,
                source_column TEXT NOT NULL,
                target_table_id INTEGER NOT NULL REFERENCES tables(id) ON DELETE CASCADE,
                target_column TEXT,
                constraint_name TEXT,
                UNIQUE(source_table_id, source_column, target_table_id, target_column)
            );
            CREATE TABLE IF NOT EXISTS indexes (
                id INTEGER PRIMARY KEY,
                table_id INTEGER NOT NULL REFERENCES tables(id) ON DELETE CASCADE,
                index_name TEXT NOT NULL,
                indexed_columns TEXT NOT NULL,
                is_unique INTEGER NOT NULL DEFAULT 0 CHECK (is_unique IN (0, 1)),
                UNIQUE(table_id, index_name)
            );
            CREATE VIRTUAL TABLE IF NOT EXISTS tables_fts USING fts5(
                table_name,
                table_description,
                column_names,
                column_descriptions,
                table_id UNINDEXED,
                version_id UNINDEXED,
                tokenize = 'unicode61 remove_diacritics 2'
            );
            CREATE INDEX IF NOT EXISTS idx_tables_version_module
                ON tables(version_id, module);
            CREATE INDEX IF NOT EXISTS idx_columns_table
                ON columns(table_id);
            "#,
        )?;
        if rebuild_fts {
            let version_ids: Vec<i64> = self
                .connection
                .prepare("SELECT id FROM versions")?
                .query_map([], |row| row.get(0))?
                .collect::<SqlResult<Vec<_>>>()?;
            for version_id in version_ids {
                self.rebuild_fts(version_id)?;
            }
        }
        Ok(())
    }

    pub fn create_version(&self, release_code: &str, active: bool) -> SqlResult<i64> {
        if active {
            self.connection
                .execute("UPDATE versions SET active_bool = 0", [])?;
        }
        self.connection.execute(
            "INSERT INTO versions (release_code, active_bool) VALUES (?1, ?2)",
            params![release_code, active],
        )?;
        Ok(self.connection.last_insert_rowid())
    }

    pub fn active_version(&self) -> SqlResult<Option<Version>> {
        self.connection
            .query_row(
                "SELECT id, release_code, synced_at, active_bool
                 FROM versions WHERE active_bool = 1 LIMIT 1",
                [],
                |row| {
                    Ok(Version {
                        id: row.get(0)?,
                        release_code: row.get(1)?,
                        synced_at: row.get(2)?,
                        active: row.get(3)?,
                    })
                },
            )
            .optional()
    }

    pub fn version_by_release(&self, release_code: &str) -> SqlResult<Option<Version>> {
        self.connection
            .query_row(
                "SELECT id, release_code, synced_at, active_bool
                 FROM versions WHERE release_code = ?1",
                params![release_code],
                |row| {
                    Ok(Version {
                        id: row.get(0)?,
                        release_code: row.get(1)?,
                        synced_at: row.get(2)?,
                        active: row.get(3)?,
                    })
                },
            )
            .optional()
    }

    pub fn modules_for_version(&self, version_id: i64) -> SqlResult<BTreeSet<String>> {
        let mut statement = self
            .connection
            .prepare("SELECT DISTINCT module FROM tables WHERE version_id = ?1")?;
        let modules = statement
            .query_map(params![version_id], |row| row.get(0))?
            .collect();
        modules
    }

    pub fn list_versions(&self) -> SqlResult<Vec<ReleaseSummary>> {
        let mut statement = self.connection.prepare(
            "SELECT id, release_code, synced_at, active_bool
             FROM versions ORDER BY id DESC",
        )?;
        let versions = statement
            .query_map([], |row| {
                Ok(Version {
                    id: row.get(0)?,
                    release_code: row.get(1)?,
                    synced_at: row.get(2)?,
                    active: row.get(3)?,
                })
            })?
            .collect::<SqlResult<Vec<_>>>()?;

        versions
            .into_iter()
            .map(|version| {
                let modules = self.modules_for_version(version.id)?;
                let table_count: usize = self.connection.query_row(
                    "SELECT COUNT(*) FROM tables WHERE version_id = ?1",
                    params![version.id],
                    |row| row.get(0),
                )?;
                Ok(ReleaseSummary {
                    version,
                    modules: modules.into_iter().collect(),
                    table_count,
                })
            })
            .collect()
    }

    pub fn delete_version_by_release(&self, release_code: &str) -> SqlResult<bool> {
        Ok(self.connection.execute(
            "DELETE FROM versions WHERE release_code = ?1",
            params![release_code],
        )? > 0)
    }

    pub fn activate_version(&self, version_id: i64) -> SqlResult<()> {
        self.connection.execute_batch("BEGIN IMMEDIATE")?;
        let result = (|| {
            self.connection
                .execute("UPDATE versions SET active_bool = 0", [])?;
            self.connection.execute(
                "UPDATE versions SET active_bool = 1 WHERE id = ?1",
                params![version_id],
            )?;
            Ok::<_, rusqlite::Error>(())
        })();
        match result {
            Ok(()) => self.connection.execute_batch("COMMIT"),
            Err(error) => {
                let _ = self.connection.execute_batch("ROLLBACK");
                Err(error)
            }
        }
    }

    pub fn clone_version(&self, source_id: i64, release_code: &str) -> SqlResult<i64> {
        let tx = self.connection.unchecked_transaction()?;
        tx.execute(
            "INSERT INTO versions (release_code, active_bool)
             VALUES (?1, 0)",
            params![release_code],
        )?;
        let target_id = tx.last_insert_rowid();
        tx.execute(
            "INSERT INTO tables
             (version_id, module, table_name, description, source_url, object_type)
             SELECT ?1, module, table_name, description, source_url, object_type
             FROM tables WHERE version_id = ?2",
            params![target_id, source_id],
        )?;
        tx.execute(
            "INSERT INTO columns
             (table_id, column_name, data_type, length, nullable, description)
             SELECT target.id, source.column_name, source.data_type, source.length,
                    source.nullable, source.description
             FROM columns source
             JOIN tables source_table ON source_table.id = source.table_id
             JOIN tables target ON target.version_id = ?1
                AND target.table_name = source_table.table_name
             WHERE source_table.version_id = ?2",
            params![target_id, source_id],
        )?;
        tx.execute(
            "INSERT INTO indexes
             (table_id, index_name, indexed_columns, is_unique)
             SELECT target.id, source.index_name, source.indexed_columns, source.is_unique
             FROM indexes source
             JOIN tables source_table ON source_table.id = source.table_id
             JOIN tables target ON target.version_id = ?1
                AND target.table_name = source_table.table_name
             WHERE source_table.version_id = ?2",
            params![target_id, source_id],
        )?;
        tx.execute(
            "INSERT INTO foreign_key_references
             (source_table_id, source_column, target_table_id, target_column, constraint_name)
             SELECT source_target.id, r.source_column, destination_target.id,
                    r.target_column, r.constraint_name
             FROM foreign_key_references r
             JOIN tables source_table ON source_table.id = r.source_table_id
             JOIN tables destination_table ON destination_table.id = r.target_table_id
             JOIN tables source_target ON source_target.version_id = ?1
                AND source_target.table_name = source_table.table_name
             JOIN tables destination_target ON destination_target.version_id = ?1
                AND destination_target.table_name = destination_table.table_name
             WHERE source_table.version_id = ?2 AND destination_table.version_id = ?2",
            params![target_id, source_id],
        )?;
        tx.commit()?;
        self.rebuild_fts(target_id)?;
        Ok(target_id)
    }

    fn upsert_catalog_table_tx(
        tx: &rusqlite::Transaction<'_>,
        version_id: i64,
        table: &CatalogTable,
    ) -> SqlResult<i64> {
        tx.execute(
            "INSERT INTO tables (version_id, module, table_name, description, source_url, object_type)
             VALUES (?1, ?2, upper(?3), ?4, ?5, ?6)
             ON CONFLICT(version_id, table_name) DO UPDATE SET
               module = excluded.module, description = excluded.description,
               source_url = excluded.source_url, object_type = excluded.object_type",
            params![
                version_id,
                table.module,
                table.table_name,
                table.description,
                table.source_url,
                table.object_type
            ],
        )?;
        let table_id: i64 = tx.query_row(
            "SELECT id FROM tables WHERE version_id = ?1 AND table_name = upper(?2)",
            params![version_id, table.table_name],
            |row| row.get(0),
        )?;

        tx.execute("DELETE FROM columns WHERE table_id = ?1", params![table_id])?;
        for column in &table.columns {
            tx.execute(
                "INSERT INTO columns
                 (table_id, column_name, data_type, length, nullable, description)
                 VALUES (?1, upper(?2), ?3, ?4, ?5, ?6)",
                params![
                    table_id,
                    column.column_name,
                    column.data_type,
                    column.length,
                    column.nullable,
                    column.description
                ],
            )?;
        }

        tx.execute(
            "DELETE FROM foreign_key_references WHERE source_table_id = ?1",
            params![table_id],
        )?;
        for reference in &table.references {
            let destination_id: Option<i64> = tx
                .query_row(
                    "SELECT id FROM tables WHERE version_id = ?1 AND table_name = upper(?2)",
                    params![version_id, reference.target_table],
                    |row| row.get(0),
                )
                .optional()?;
            if let Some(destination_id) = destination_id {
                tx.execute(
                    "INSERT OR IGNORE INTO foreign_key_references
                     (source_table_id, source_column, target_table_id, target_column, constraint_name)
                     VALUES (?1, upper(?2), ?3, upper(?4), ?5)",
                    params![
                        table_id,
                        reference.source_column,
                        destination_id,
                        reference.target_column,
                        reference.constraint_name
                    ],
                )?;
            }
        }

        tx.execute("DELETE FROM indexes WHERE table_id = ?1", params![table_id])?;
        for index in &table.indexes {
            tx.execute(
                "INSERT INTO indexes
                 (table_id, index_name, indexed_columns, is_unique)
                 VALUES (?1, upper(?2), ?3, ?4)",
                params![
                    table_id,
                    index.index_name,
                    index.indexed_columns.join(","),
                    index.is_unique
                ],
            )?;
        }
        Ok(table_id)
    }

    pub fn upsert_catalog_table(&self, version_id: i64, table: &CatalogTable) -> SqlResult<i64> {
        let tx = self.connection.unchecked_transaction()?;
        let table_id = Self::upsert_catalog_table_tx(&tx, version_id, table)?;
        tx.commit()?;
        self.rebuild_fts(version_id)?;
        Ok(table_id)
    }

    pub fn upsert_catalog_tables(&self, version_id: i64, tables: &[CatalogTable]) -> SqlResult<()> {
        let tx = self.connection.unchecked_transaction()?;
        for table in tables {
            Self::upsert_catalog_table_tx(&tx, version_id, table)?;
        }
        tx.commit()?;
        Ok(())
    }

    pub fn rebuild_fts(&self, version_id: i64) -> SqlResult<()> {
        self.connection.execute(
            "DELETE FROM tables_fts WHERE version_id = ?1",
            params![version_id],
        )?;
        self.connection.execute(
            "INSERT INTO tables_fts
             (table_name, table_description, column_names, column_descriptions, table_id, version_id)
             SELECT t.table_name, COALESCE(t.description, ''),
                    COALESCE(GROUP_CONCAT(c.column_name, ' '), ''),
                    COALESCE(GROUP_CONCAT(COALESCE(c.description, ''), ' '), ''),
                    t.id, t.version_id
             FROM tables t LEFT JOIN columns c ON c.table_id = t.id
             WHERE t.version_id = ?1 GROUP BY t.id",
            params![version_id],
        )?;
        Ok(())
    }

    pub fn list_modules_and_tables(&self, module: Option<&str>) -> SqlResult<Vec<TableRecord>> {
        let version = self
            .active_version()?
            .ok_or_else(|| rusqlite::Error::QueryReturnedNoRows)?;
        let mut statement = self.connection.prepare(
            "SELECT id, version_id, module, table_name, description, source_url, object_type
             FROM tables WHERE version_id = ?1
             AND (?2 IS NULL OR lower(module) = lower(?2))
             ORDER BY module, table_name",
        )?;
        let rows = statement.query_map(params![version.id, module], |row| {
            Ok(TableRecord {
                id: row.get(0)?,
                version_id: row.get(1)?,
                module: row.get(2)?,
                table_name: row.get(3)?,
                description: row.get(4)?,
                source_url: row.get(5)?,
                object_type: row.get(6)?,
            })
        })?;
        rows.collect()
    }

    pub fn search_tables(&self, query: &str, limit: usize) -> SqlResult<Vec<TableRecord>> {
        let version = self
            .active_version()?
            .ok_or_else(|| rusqlite::Error::QueryReturnedNoRows)?;
        let exact = query.trim().to_ascii_uppercase();
        let mut statement = self.connection.prepare(
            "SELECT t.id, t.version_id, t.module, t.table_name, t.description,
                    t.source_url, t.object_type
             FROM tables t WHERE t.version_id = ?1 AND t.table_name = ?2
             UNION ALL
             SELECT t.id, t.version_id, t.module, t.table_name, t.description,
                    t.source_url, t.object_type
             FROM tables_fts f JOIN tables t ON t.id = f.table_id
             WHERE f.version_id = ?1 AND tables_fts MATCH ?3 AND t.table_name <> ?2
             ORDER BY table_name LIMIT ?4",
        )?;
        let fts_query = format!("\"{}\"*", query.replace('"', " "));
        let rows = statement.query_map(params![version.id, exact, fts_query, limit], |row| {
            Ok(TableRecord {
                id: row.get(0)?,
                version_id: row.get(1)?,
                module: row.get(2)?,
                table_name: row.get(3)?,
                description: row.get(4)?,
                source_url: row.get(5)?,
                object_type: row.get(6)?,
            })
        })?;
        rows.collect()
    }

    pub fn search_columns(
        &self,
        query: &str,
        module: Option<&str>,
        limit: usize,
    ) -> SqlResult<Vec<ColumnSearchResult>> {
        let version = self
            .active_version()?
            .ok_or_else(|| rusqlite::Error::QueryReturnedNoRows)?;
        let pattern = format!("%{}%", query.trim().to_ascii_lowercase());
        let mut statement = self.connection.prepare(
            "SELECT t.id, t.version_id, t.module, t.table_name, t.description,
                    t.source_url, t.object_type,
                    c.id, c.table_id, c.column_name, c.data_type, c.length,
                    c.nullable, c.description
             FROM columns c
             JOIN tables t ON t.id = c.table_id
             WHERE t.version_id = ?1
               AND (?2 IS NULL OR lower(t.module) = lower(?2))
               AND (lower(c.column_name) LIKE ?3 OR lower(COALESCE(c.description, '')) LIKE ?3)
             ORDER BY CASE WHEN lower(c.column_name) = lower(?4) THEN 0 ELSE 1 END,
                      t.table_name, c.column_name
             LIMIT ?5",
        )?;
        let rows = statement.query_map(
            params![version.id, module, pattern, query.trim(), limit],
            |row| {
                Ok(ColumnSearchResult {
                    table: TableRecord {
                        id: row.get(0)?,
                        version_id: row.get(1)?,
                        module: row.get(2)?,
                        table_name: row.get(3)?,
                        description: row.get(4)?,
                        source_url: row.get(5)?,
                        object_type: row.get(6)?,
                    },
                    column: ColumnRecord {
                        id: row.get(7)?,
                        table_id: row.get(8)?,
                        column_name: row.get(9)?,
                        data_type: row.get(10)?,
                        length: row.get(11)?,
                        nullable: row.get(12)?,
                        description: row.get(13)?,
                    },
                })
            },
        )?;
        rows.collect()
    }

    pub fn find_tables_by_column(
        &self,
        column: &str,
        module: Option<&str>,
        limit: usize,
    ) -> SqlResult<Vec<TableRecord>> {
        let version = self
            .active_version()?
            .ok_or_else(|| rusqlite::Error::QueryReturnedNoRows)?;
        let exact = column.trim().to_ascii_uppercase();
        let prefix = format!("{exact}%");
        let mut statement = self.connection.prepare(
            "SELECT DISTINCT t.id, t.version_id, t.module, t.table_name,
                    t.description, t.source_url, t.object_type
             FROM tables t
             JOIN columns c ON c.table_id = t.id
             WHERE t.version_id = ?1
               AND (?2 IS NULL OR lower(t.module) = lower(?2))
               AND (c.column_name = ?3 OR c.column_name LIKE ?4)
             ORDER BY CASE WHEN c.column_name = ?3 THEN 0 ELSE 1 END,
                      t.table_name
             LIMIT ?5",
        )?;
        let rows =
            statement.query_map(params![version.id, module, exact, prefix, limit], |row| {
                Ok(TableRecord {
                    id: row.get(0)?,
                    version_id: row.get(1)?,
                    module: row.get(2)?,
                    table_name: row.get(3)?,
                    description: row.get(4)?,
                    source_url: row.get(5)?,
                    object_type: row.get(6)?,
                })
            })?;
        rows.collect()
    }

    pub fn table_structure(&self, table_name: &str) -> SqlResult<Option<TableStructure>> {
        let version = self
            .active_version()?
            .ok_or_else(|| rusqlite::Error::QueryReturnedNoRows)?;
        let table = self
            .connection
            .query_row(
                "SELECT id, version_id, module, table_name, description, source_url, object_type
                 FROM tables WHERE version_id = ?1 AND table_name = upper(?2)",
                params![version.id, table_name],
                |row| {
                    Ok(TableRecord {
                        id: row.get(0)?,
                        version_id: row.get(1)?,
                        module: row.get(2)?,
                        table_name: row.get(3)?,
                        description: row.get(4)?,
                        source_url: row.get(5)?,
                        object_type: row.get(6)?,
                    })
                },
            )
            .optional()?;
        let Some(table) = table else {
            return Ok(None);
        };
        let columns = self.query_columns(table.id)?;
        let outgoing_references = self.query_references("WHERE source_table_id = ?1", table.id)?;
        let incoming_references = self.query_references("WHERE target_table_id = ?1", table.id)?;
        let mut indexes = Vec::new();
        let mut statement = self.connection.prepare(
            "SELECT id, table_id, index_name, indexed_columns, is_unique
             FROM indexes WHERE table_id = ?1 ORDER BY index_name",
        )?;
        for row in statement.query_map(params![table.id], |row| {
            Ok(IndexRecord {
                id: row.get(0)?,
                table_id: row.get(1)?,
                index_name: row.get(2)?,
                indexed_columns: row.get(3)?,
                is_unique: row.get(4)?,
            })
        })? {
            indexes.push(row?);
        }
        Ok(Some(TableStructure {
            table,
            columns,
            outgoing_references,
            incoming_references,
            indexes,
        }))
    }

    fn query_columns(&self, table_id: i64) -> SqlResult<Vec<ColumnRecord>> {
        let mut statement = self.connection.prepare(
            "SELECT id, table_id, column_name, data_type, length, nullable, description
             FROM columns WHERE table_id = ?1 ORDER BY id",
        )?;
        let rows = statement
            .query_map(params![table_id], |row| {
                Ok(ColumnRecord {
                    id: row.get(0)?,
                    table_id: row.get(1)?,
                    column_name: row.get(2)?,
                    data_type: row.get(3)?,
                    length: row.get(4)?,
                    nullable: row.get(5)?,
                    description: row.get(6)?,
                })
            })?
            .collect();
        rows
    }

    fn query_references(&self, condition: &str, table_id: i64) -> SqlResult<Vec<ReferenceRecord>> {
        let sql = format!(
            "SELECT id, source_table_id, source_column, target_table_id,
                    target_column, constraint_name
             FROM foreign_key_references {} ORDER BY id",
            condition
        );
        let mut statement = self.connection.prepare(&sql)?;
        let rows = statement
            .query_map(params![table_id], |row| {
                Ok(ReferenceRecord {
                    id: row.get(0)?,
                    source_table_id: row.get(1)?,
                    source_column: row.get(2)?,
                    target_table_id: row.get(3)?,
                    target_column: row.get(4)?,
                    constraint_name: row.get(5)?,
                })
            })?
            .collect();
        rows
    }

    pub fn suggest_joins(&self, left: &str, right: &str) -> SqlResult<Vec<ReferenceRecord>> {
        let version = self
            .active_version()?
            .ok_or_else(|| rusqlite::Error::QueryReturnedNoRows)?;
        let mut statement = self.connection.prepare(
            "SELECT r.id, r.source_table_id, r.source_column, r.target_table_id,
                    r.target_column, r.constraint_name
             FROM foreign_key_references r
             JOIN tables source ON source.id = r.source_table_id
             JOIN tables target ON target.id = r.target_table_id
             WHERE source.version_id = ?1
               AND ((source.table_name = upper(?2) AND target.table_name = upper(?3))
                 OR (source.table_name = upper(?3) AND target.table_name = upper(?2)))
             ORDER BY r.id",
        )?;
        let rows = statement
            .query_map(params![version.id, left, right], |row| {
                Ok(ReferenceRecord {
                    id: row.get(0)?,
                    source_table_id: row.get(1)?,
                    source_column: row.get(2)?,
                    target_table_id: row.get(3)?,
                    target_column: row.get(4)?,
                    constraint_name: row.get(5)?,
                })
            })?
            .collect();
        rows
    }

    pub fn find_related_tables(
        &self,
        table_name: &str,
        max_depth: usize,
        limit: usize,
    ) -> SqlResult<Vec<RelatedTable>> {
        let version = self
            .active_version()?
            .ok_or_else(|| rusqlite::Error::QueryReturnedNoRows)?;
        let root: Option<i64> = self
            .connection
            .query_row(
                "SELECT id FROM tables WHERE version_id = ?1 AND table_name = upper(?2)",
                params![version.id, table_name],
                |row| row.get(0),
            )
            .optional()?;
        let Some(root) = root else {
            return Ok(Vec::new());
        };

        let mut statement = self.connection.prepare(
            "SELECT r.source_table_id, source.table_name, r.source_column,
                    r.target_table_id, target.table_name, r.target_column,
                    r.constraint_name
             FROM foreign_key_references r
             JOIN tables source ON source.id = r.source_table_id
             JOIN tables target ON target.id = r.target_table_id
             WHERE source.version_id = ?1
             ORDER BY r.id",
        )?;
        let references = statement
            .query_map(params![version.id], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, i64>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, Option<String>>(5)?,
                    row.get::<_, Option<String>>(6)?,
                ))
            })?
            .collect::<SqlResult<Vec<_>>>()?;

        let mut frontier = BTreeSet::from([root]);
        let mut visited = BTreeSet::from([root]);
        let mut results = Vec::new();
        for depth in 1..=max_depth {
            if frontier.is_empty() || results.len() >= limit {
                break;
            }
            let mut next = BTreeSet::new();
            for (
                source_id,
                source_name,
                source_column,
                target_id,
                target_name,
                target_column,
                constraint_name,
            ) in &references
            {
                let related_id = if frontier.contains(source_id) {
                    Some(*target_id)
                } else if frontier.contains(target_id) {
                    Some(*source_id)
                } else {
                    None
                };
                let Some(related_id) = related_id else {
                    continue;
                };
                if visited.contains(&related_id) {
                    continue;
                }
                let related_table = if related_id == *source_id {
                    self.table_by_id(*source_id)?
                } else {
                    self.table_by_id(*target_id)?
                };
                let Some(table) = related_table else {
                    continue;
                };
                results.push(RelatedTable {
                    table,
                    source_table: source_name.clone(),
                    source_column: source_column.clone(),
                    target_table: target_name.clone(),
                    target_column: target_column.clone(),
                    constraint_name: constraint_name.clone(),
                    depth,
                });
                visited.insert(related_id);
                next.insert(related_id);
                if results.len() >= limit {
                    break;
                }
            }
            frontier = next;
        }
        Ok(results)
    }

    fn table_by_id(&self, table_id: i64) -> SqlResult<Option<TableRecord>> {
        self.connection
            .query_row(
                "SELECT id, version_id, module, table_name, description, source_url, object_type
                 FROM tables WHERE id = ?1",
                params![table_id],
                |row| {
                    Ok(TableRecord {
                        id: row.get(0)?,
                        version_id: row.get(1)?,
                        module: row.get(2)?,
                        table_name: row.get(3)?,
                        description: row.get(4)?,
                        source_url: row.get(5)?,
                        object_type: row.get(6)?,
                    })
                },
            )
            .optional()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_table(name: &str) -> CatalogTable {
        CatalogTable {
            module: "Finance".to_owned(),
            table_name: name.to_owned(),
            description: Some("Test entity".to_owned()),
            source_url: Some("https://docs.oracle.com/example.html".to_owned()),
            object_type: Some("TABLE".to_owned()),
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
    fn creates_schema_and_retrieves_structure() {
        let db = Database::in_memory().expect("in-memory SQLite");
        let version_id = db.create_version("26B", true).expect("release");
        db.upsert_catalog_table(version_id, &sample_table("PO_HEADERS_ALL"))
            .expect("table");
        let structure = db
            .table_structure("PO_HEADERS_ALL")
            .expect("query")
            .expect("existing table");
        assert_eq!(structure.table.table_name, "PO_HEADERS_ALL");
        assert_eq!(structure.columns[0].data_type, "NUMBER");
    }

    #[test]
    fn prioritizes_exact_name_in_search() {
        let db = Database::in_memory().expect("in-memory SQLite");
        let version_id = db.create_version("26B", true).expect("release");
        db.upsert_catalog_table(version_id, &sample_table("AP_INVOICES_ALL"))
            .expect("table");
        let matches = db.search_tables("AP_INVOICES_ALL", 10).expect("search");
        assert_eq!(matches[0].table_name, "AP_INVOICES_ALL");
    }

    #[test]
    fn migrates_legacy_spanish_schema() {
        let connection = Connection::open_in_memory().expect("in-memory SQLite");
        connection
            .execute_batch(
                "
                CREATE TABLE tabla_versiones (
                    id INTEGER PRIMARY KEY,
                    release_code TEXT NOT NULL UNIQUE,
                    fecha_sincronizacion TEXT NOT NULL,
                    activo_bool INTEGER NOT NULL
                );
                CREATE TABLE tablas (
                    id INTEGER PRIMARY KEY,
                    version_id INTEGER NOT NULL,
                    modulo TEXT NOT NULL,
                    nombre_tabla TEXT NOT NULL,
                    descripcion TEXT,
                    source_url TEXT,
                    object_type TEXT
                );
                CREATE TABLE columnas (
                    id INTEGER PRIMARY KEY,
                    tabla_id INTEGER NOT NULL,
                    nombre_columna TEXT NOT NULL,
                    tipo_datos TEXT NOT NULL,
                    longitud INTEGER,
                    nullable INTEGER NOT NULL,
                    descripcion TEXT
                );
                CREATE TABLE referencias (
                    id INTEGER PRIMARY KEY,
                    tabla_origen_id INTEGER NOT NULL,
                    columna_origen TEXT NOT NULL,
                    tabla_destino_id INTEGER NOT NULL,
                    columna_destino TEXT NOT NULL,
                    nombre_constraint TEXT
                );
                CREATE TABLE indices (
                    id INTEGER PRIMARY KEY,
                    tabla_id INTEGER NOT NULL,
                    nombre_indice TEXT NOT NULL,
                    columnas_indexadas TEXT NOT NULL,
                    es_unico INTEGER NOT NULL
                );
                ",
            )
            .expect("legacy schema");
        let db = Database { connection };
        db.migrate().expect("schema migration");
        assert!(db.table_exists("versions").expect("versions table"));
        assert!(db
            .column_exists("tables", "table_name")
            .expect("table name"));
        assert!(db.column_exists("columns", "data_type").expect("data type"));
        assert!(db
            .table_exists("foreign_key_references")
            .expect("references table"));
    }
}
