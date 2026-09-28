use rusqlite::{params, Connection, OptionalExtension, Result as SqlResult};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Version {
    pub id: i64,
    pub release_code: String,
    pub fecha_sincronizacion: String,
    pub activo: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TableRecord {
    pub id: i64,
    pub version_id: i64,
    pub modulo: String,
    pub nombre_tabla: String,
    pub descripcion: Option<String>,
    pub source_url: Option<String>,
    pub object_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ColumnRecord {
    pub id: i64,
    pub tabla_id: i64,
    pub nombre_columna: String,
    pub tipo_datos: String,
    pub longitud: Option<i64>,
    pub nullable: bool,
    pub descripcion: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ReferenceRecord {
    pub id: i64,
    pub tabla_origen_id: i64,
    pub columna_origen: String,
    pub tabla_destino_id: i64,
    pub columna_destino: String,
    pub nombre_constraint: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct IndexRecord {
    pub id: i64,
    pub tabla_id: i64,
    pub nombre_indice: String,
    pub columnas_indexadas: String,
    pub es_unico: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableStructure {
    pub tabla: TableRecord,
    pub columnas: Vec<ColumnRecord>,
    pub referencias_salientes: Vec<ReferenceRecord>,
    pub referencias_entrantes: Vec<ReferenceRecord>,
    pub indices: Vec<IndexRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CatalogTable {
    pub modulo: String,
    pub nombre_tabla: String,
    pub descripcion: Option<String>,
    pub source_url: Option<String>,
    pub object_type: Option<String>,
    pub columnas: Vec<CatalogColumn>,
    pub referencias: Vec<CatalogReference>,
    pub indices: Vec<CatalogIndex>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CatalogColumn {
    pub nombre_columna: String,
    pub tipo_datos: String,
    pub longitud: Option<i64>,
    pub nullable: bool,
    pub descripcion: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CatalogReference {
    pub tabla_destino: String,
    pub columna_origen: String,
    pub columna_destino: String,
    pub nombre_constraint: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CatalogIndex {
    pub nombre_indice: String,
    pub columnas_indexadas: Vec<String>,
    pub es_unico: bool,
}

pub struct Database {
    connection: Connection,
}

impl Database {
    pub fn open(path: impl AsRef<Path>) -> SqlResult<Self> {
        let connection = Connection::open(path)?;
        connection.execute_batch("PRAGMA foreign_keys = ON;")?;
        let db = Self { connection };
        db.migrate()?;
        Ok(db)
    }

    pub fn in_memory() -> SqlResult<Self> {
        Self::open(":memory:")
    }

    fn migrate(&self) -> SqlResult<()> {
        self.connection.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS tabla_versiones (
                id INTEGER PRIMARY KEY,
                release_code TEXT NOT NULL UNIQUE,
                fecha_sincronizacion TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                activo_bool INTEGER NOT NULL DEFAULT 0 CHECK (activo_bool IN (0, 1))
            );
            CREATE TABLE IF NOT EXISTS tablas (
                id INTEGER PRIMARY KEY,
                version_id INTEGER NOT NULL REFERENCES tabla_versiones(id) ON DELETE CASCADE,
                modulo TEXT NOT NULL,
                nombre_tabla TEXT NOT NULL,
                descripcion TEXT,
                source_url TEXT,
                object_type TEXT,
                UNIQUE(version_id, nombre_tabla)
            );
            CREATE TABLE IF NOT EXISTS columnas (
                id INTEGER PRIMARY KEY,
                tabla_id INTEGER NOT NULL REFERENCES tablas(id) ON DELETE CASCADE,
                nombre_columna TEXT NOT NULL,
                tipo_datos TEXT NOT NULL,
                longitud INTEGER,
                nullable INTEGER NOT NULL DEFAULT 1 CHECK (nullable IN (0, 1)),
                descripcion TEXT,
                UNIQUE(tabla_id, nombre_columna)
            );
            CREATE TABLE IF NOT EXISTS referencias (
                id INTEGER PRIMARY KEY,
                tabla_origen_id INTEGER NOT NULL REFERENCES tablas(id) ON DELETE CASCADE,
                columna_origen TEXT NOT NULL,
                tabla_destino_id INTEGER NOT NULL REFERENCES tablas(id) ON DELETE CASCADE,
                columna_destino TEXT NOT NULL,
                nombre_constraint TEXT,
                UNIQUE(tabla_origen_id, columna_origen, tabla_destino_id, columna_destino)
            );
            CREATE TABLE IF NOT EXISTS indices (
                id INTEGER PRIMARY KEY,
                tabla_id INTEGER NOT NULL REFERENCES tablas(id) ON DELETE CASCADE,
                nombre_indice TEXT NOT NULL,
                columnas_indexadas TEXT NOT NULL,
                es_unico INTEGER NOT NULL DEFAULT 0 CHECK (es_unico IN (0, 1)),
                UNIQUE(tabla_id, nombre_indice)
            );
            CREATE VIRTUAL TABLE IF NOT EXISTS tablas_fts USING fts5(
                nombre_tabla,
                descripcion_tabla,
                nombres_columnas,
                descripciones_columnas,
                tabla_id UNINDEXED,
                version_id UNINDEXED,
                tokenize = 'unicode61 remove_diacritics 2'
            );
            CREATE INDEX IF NOT EXISTS idx_tablas_version_modulo
                ON tablas(version_id, modulo);
            CREATE INDEX IF NOT EXISTS idx_columnas_tabla
                ON columnas(tabla_id);
            "#,
        )
    }

    pub fn create_version(&self, release_code: &str, active: bool) -> SqlResult<i64> {
        if active {
            self.connection
                .execute("UPDATE tabla_versiones SET activo_bool = 0", [])?;
        }
        self.connection.execute(
            "INSERT INTO tabla_versiones (release_code, activo_bool) VALUES (?1, ?2)",
            params![release_code, active],
        )?;
        Ok(self.connection.last_insert_rowid())
    }

    pub fn active_version(&self) -> SqlResult<Option<Version>> {
        self.connection
            .query_row(
                "SELECT id, release_code, fecha_sincronizacion, activo_bool
                 FROM tabla_versiones WHERE activo_bool = 1 LIMIT 1",
                [],
                |row| {
                    Ok(Version {
                        id: row.get(0)?,
                        release_code: row.get(1)?,
                        fecha_sincronizacion: row.get(2)?,
                        activo: row.get(3)?,
                    })
                },
            )
            .optional()
    }

    pub fn version_by_release(&self, release_code: &str) -> SqlResult<Option<Version>> {
        self.connection
            .query_row(
                "SELECT id, release_code, fecha_sincronizacion, activo_bool
                 FROM tabla_versiones WHERE release_code = ?1",
                params![release_code],
                |row| {
                    Ok(Version {
                        id: row.get(0)?,
                        release_code: row.get(1)?,
                        fecha_sincronizacion: row.get(2)?,
                        activo: row.get(3)?,
                    })
                },
            )
            .optional()
    }

    pub fn version_by_id(&self, id: i64) -> SqlResult<Option<Version>> {
        self.connection
            .query_row(
                "SELECT id, release_code, fecha_sincronizacion, activo_bool
                 FROM tabla_versiones WHERE id = ?1",
                params![id],
                |row| {
                    Ok(Version {
                        id: row.get(0)?,
                        release_code: row.get(1)?,
                        fecha_sincronizacion: row.get(2)?,
                        activo: row.get(3)?,
                    })
                },
            )
            .optional()
    }

    pub fn activate_version(&self, version_id: i64) -> SqlResult<()> {
        self.connection.execute_batch("BEGIN IMMEDIATE")?;
        let result = (|| {
            self.connection
                .execute("UPDATE tabla_versiones SET activo_bool = 0", [])?;
            self.connection.execute(
                "UPDATE tabla_versiones SET activo_bool = 1 WHERE id = ?1",
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
            "INSERT INTO tabla_versiones (release_code, activo_bool)
             VALUES (?1, 0)",
            params![release_code],
        )?;
        let target_id = tx.last_insert_rowid();
        tx.execute(
            "INSERT INTO tablas
             (version_id, modulo, nombre_tabla, descripcion, source_url, object_type)
             SELECT ?1, modulo, nombre_tabla, descripcion, source_url, object_type
             FROM tablas WHERE version_id = ?2",
            params![target_id, source_id],
        )?;
        tx.execute(
            "INSERT INTO columnas
             (tabla_id, nombre_columna, tipo_datos, longitud, nullable, descripcion)
             SELECT target.id, source.nombre_columna, source.tipo_datos, source.longitud,
                    source.nullable, source.descripcion
             FROM columnas source
             JOIN tablas source_table ON source_table.id = source.tabla_id
             JOIN tablas target ON target.version_id = ?1
                AND target.nombre_tabla = source_table.nombre_tabla
             WHERE source_table.version_id = ?2",
            params![target_id, source_id],
        )?;
        tx.execute(
            "INSERT INTO indices
             (tabla_id, nombre_indice, columnas_indexadas, es_unico)
             SELECT target.id, source.nombre_indice, source.columnas_indexadas, source.es_unico
             FROM indices source
             JOIN tablas source_table ON source_table.id = source.tabla_id
             JOIN tablas target ON target.version_id = ?1
                AND target.nombre_tabla = source_table.nombre_tabla
             WHERE source_table.version_id = ?2",
            params![target_id, source_id],
        )?;
        tx.execute(
            "INSERT INTO referencias
             (tabla_origen_id, columna_origen, tabla_destino_id, columna_destino, nombre_constraint)
             SELECT source_target.id, r.columna_origen, destination_target.id,
                    r.columna_destino, r.nombre_constraint
             FROM referencias r
             JOIN tablas source_table ON source_table.id = r.tabla_origen_id
             JOIN tablas destination_table ON destination_table.id = r.tabla_destino_id
             JOIN tablas source_target ON source_target.version_id = ?1
                AND source_target.nombre_tabla = source_table.nombre_tabla
             JOIN tablas destination_target ON destination_target.version_id = ?1
                AND destination_target.nombre_tabla = destination_table.nombre_tabla
             WHERE source_table.version_id = ?2 AND destination_table.version_id = ?2",
            params![target_id, source_id],
        )?;
        tx.commit()?;
        self.rebuild_fts(target_id)?;
        Ok(target_id)
    }

    pub fn upsert_catalog_table(&self, version_id: i64, table: &CatalogTable) -> SqlResult<i64> {
        let tx = self.connection.unchecked_transaction()?;
        tx.execute(
            "INSERT INTO tablas (version_id, modulo, nombre_tabla, descripcion, source_url, object_type)
             VALUES (?1, ?2, upper(?3), ?4, ?5, ?6)
             ON CONFLICT(version_id, nombre_tabla) DO UPDATE SET
               modulo = excluded.modulo, descripcion = excluded.descripcion,
               source_url = excluded.source_url, object_type = excluded.object_type",
            params![
                version_id,
                table.modulo,
                table.nombre_tabla,
                table.descripcion,
                table.source_url,
                table.object_type
            ],
        )?;
        let table_id: i64 = tx.query_row(
            "SELECT id FROM tablas WHERE version_id = ?1 AND nombre_tabla = upper(?2)",
            params![version_id, table.nombre_tabla],
            |row| row.get(0),
        )?;

        tx.execute(
            "DELETE FROM columnas WHERE tabla_id = ?1",
            params![table_id],
        )?;
        for column in &table.columnas {
            tx.execute(
                "INSERT INTO columnas
                 (tabla_id, nombre_columna, tipo_datos, longitud, nullable, descripcion)
                 VALUES (?1, upper(?2), ?3, ?4, ?5, ?6)",
                params![
                    table_id,
                    column.nombre_columna,
                    column.tipo_datos,
                    column.longitud,
                    column.nullable,
                    column.descripcion
                ],
            )?;
        }

        tx.execute(
            "DELETE FROM referencias WHERE tabla_origen_id = ?1",
            params![table_id],
        )?;
        for reference in &table.referencias {
            let destination_id: Option<i64> = tx
                .query_row(
                    "SELECT id FROM tablas WHERE version_id = ?1 AND nombre_tabla = upper(?2)",
                    params![version_id, reference.tabla_destino],
                    |row| row.get(0),
                )
                .optional()?;
            if let Some(destination_id) = destination_id {
                tx.execute(
                    "INSERT OR IGNORE INTO referencias
                     (tabla_origen_id, columna_origen, tabla_destino_id, columna_destino, nombre_constraint)
                     VALUES (?1, upper(?2), ?3, upper(?4), ?5)",
                    params![
                        table_id,
                        reference.columna_origen,
                        destination_id,
                        reference.columna_destino,
                        reference.nombre_constraint
                    ],
                )?;
            }
        }

        tx.execute("DELETE FROM indices WHERE tabla_id = ?1", params![table_id])?;
        for index in &table.indices {
            tx.execute(
                "INSERT INTO indices
                 (tabla_id, nombre_indice, columnas_indexadas, es_unico)
                 VALUES (?1, upper(?2), ?3, ?4)",
                params![
                    table_id,
                    index.nombre_indice,
                    index.columnas_indexadas.join(","),
                    index.es_unico
                ],
            )?;
        }
        tx.commit()?;
        self.rebuild_fts(version_id)?;
        Ok(table_id)
    }

    pub fn rebuild_fts(&self, version_id: i64) -> SqlResult<()> {
        self.connection.execute(
            "DELETE FROM tablas_fts WHERE version_id = ?1",
            params![version_id],
        )?;
        self.connection.execute(
            "INSERT INTO tablas_fts
             (nombre_tabla, descripcion_tabla, nombres_columnas, descripciones_columnas, tabla_id, version_id)
             SELECT t.nombre_tabla, COALESCE(t.descripcion, ''),
                    COALESCE(GROUP_CONCAT(c.nombre_columna, ' '), ''),
                    COALESCE(GROUP_CONCAT(COALESCE(c.descripcion, ''), ' '), ''),
                    t.id, t.version_id
             FROM tablas t LEFT JOIN columnas c ON c.tabla_id = t.id
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
            "SELECT id, version_id, modulo, nombre_tabla, descripcion, source_url, object_type
             FROM tablas WHERE version_id = ?1
             AND (?2 IS NULL OR lower(modulo) = lower(?2))
             ORDER BY modulo, nombre_tabla",
        )?;
        let rows = statement.query_map(params![version.id, module], |row| {
            Ok(TableRecord {
                id: row.get(0)?,
                version_id: row.get(1)?,
                modulo: row.get(2)?,
                nombre_tabla: row.get(3)?,
                descripcion: row.get(4)?,
                source_url: row.get(5)?,
                object_type: row.get(6)?,
            })
        })?;
        rows.collect()
    }

    pub fn tables_for_version(&self, version_id: i64) -> SqlResult<Vec<TableRecord>> {
        let mut statement = self.connection.prepare(
            "SELECT id, version_id, modulo, nombre_tabla, descripcion, source_url, object_type
             FROM tablas WHERE version_id = ?1 ORDER BY nombre_tabla",
        )?;
        let rows = statement
            .query_map(params![version_id], |row| {
                Ok(TableRecord {
                    id: row.get(0)?,
                    version_id: row.get(1)?,
                    modulo: row.get(2)?,
                    nombre_tabla: row.get(3)?,
                    descripcion: row.get(4)?,
                    source_url: row.get(5)?,
                    object_type: row.get(6)?,
                })
            })?
            .collect();
        rows
    }

    pub fn table_structure_in_version(
        &self,
        version_id: i64,
        table_name: &str,
    ) -> SqlResult<Option<TableStructure>> {
        let table = self
            .connection
            .query_row(
                "SELECT id, version_id, modulo, nombre_tabla, descripcion, source_url, object_type
                 FROM tablas WHERE version_id = ?1 AND nombre_tabla = upper(?2)",
                params![version_id, table_name],
                |row| {
                    Ok(TableRecord {
                        id: row.get(0)?,
                        version_id: row.get(1)?,
                        modulo: row.get(2)?,
                        nombre_tabla: row.get(3)?,
                        descripcion: row.get(4)?,
                        source_url: row.get(5)?,
                        object_type: row.get(6)?,
                    })
                },
            )
            .optional()?;
        let Some(table) = table else {
            return Ok(None);
        };
        let columnas = self.query_columns(table.id)?;
        let referencias_salientes =
            self.query_references("WHERE tabla_origen_id = ?1", table.id)?;
        let referencias_entrantes =
            self.query_references("WHERE tabla_destino_id = ?1", table.id)?;
        let mut indices = Vec::new();
        let mut statement = self.connection.prepare(
            "SELECT id, tabla_id, nombre_indice, columnas_indexadas, es_unico
             FROM indices WHERE tabla_id = ?1 ORDER BY nombre_indice",
        )?;
        for row in statement.query_map(params![table.id], |row| {
            Ok(IndexRecord {
                id: row.get(0)?,
                tabla_id: row.get(1)?,
                nombre_indice: row.get(2)?,
                columnas_indexadas: row.get(3)?,
                es_unico: row.get(4)?,
            })
        })? {
            indices.push(row?);
        }
        Ok(Some(TableStructure {
            tabla: table,
            columnas,
            referencias_salientes,
            referencias_entrantes,
            indices,
        }))
    }

    pub fn search_tables(&self, query: &str, limit: usize) -> SqlResult<Vec<TableRecord>> {
        let version = self
            .active_version()?
            .ok_or_else(|| rusqlite::Error::QueryReturnedNoRows)?;
        let exact = query.trim().to_ascii_uppercase();
        let mut statement = self.connection.prepare(
            "SELECT t.id, t.version_id, t.modulo, t.nombre_tabla, t.descripcion,
                    t.source_url, t.object_type
             FROM tablas t WHERE t.version_id = ?1 AND t.nombre_tabla = ?2
             UNION ALL
             SELECT t.id, t.version_id, t.modulo, t.nombre_tabla, t.descripcion,
                    t.source_url, t.object_type
             FROM tablas_fts f JOIN tablas t ON t.id = f.tabla_id
             WHERE f.version_id = ?1 AND tablas_fts MATCH ?3 AND t.nombre_tabla <> ?2
             ORDER BY nombre_tabla LIMIT ?4",
        )?;
        let fts_query = format!("\"{}\"*", query.replace('"', " "));
        let rows = statement.query_map(params![version.id, exact, fts_query, limit], |row| {
            Ok(TableRecord {
                id: row.get(0)?,
                version_id: row.get(1)?,
                modulo: row.get(2)?,
                nombre_tabla: row.get(3)?,
                descripcion: row.get(4)?,
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
                "SELECT id, version_id, modulo, nombre_tabla, descripcion, source_url, object_type
                 FROM tablas WHERE version_id = ?1 AND nombre_tabla = upper(?2)",
                params![version.id, table_name],
                |row| {
                    Ok(TableRecord {
                        id: row.get(0)?,
                        version_id: row.get(1)?,
                        modulo: row.get(2)?,
                        nombre_tabla: row.get(3)?,
                        descripcion: row.get(4)?,
                        source_url: row.get(5)?,
                        object_type: row.get(6)?,
                    })
                },
            )
            .optional()?;
        let Some(table) = table else {
            return Ok(None);
        };
        let columnas = self.query_columns(table.id)?;
        let referencias_salientes =
            self.query_references("WHERE tabla_origen_id = ?1", table.id)?;
        let referencias_entrantes =
            self.query_references("WHERE tabla_destino_id = ?1", table.id)?;
        let mut indices = Vec::new();
        let mut statement = self.connection.prepare(
            "SELECT id, tabla_id, nombre_indice, columnas_indexadas, es_unico
             FROM indices WHERE tabla_id = ?1 ORDER BY nombre_indice",
        )?;
        for row in statement.query_map(params![table.id], |row| {
            Ok(IndexRecord {
                id: row.get(0)?,
                tabla_id: row.get(1)?,
                nombre_indice: row.get(2)?,
                columnas_indexadas: row.get(3)?,
                es_unico: row.get(4)?,
            })
        })? {
            indices.push(row?);
        }
        Ok(Some(TableStructure {
            tabla: table,
            columnas,
            referencias_salientes,
            referencias_entrantes,
            indices,
        }))
    }

    fn query_columns(&self, table_id: i64) -> SqlResult<Vec<ColumnRecord>> {
        let mut statement = self.connection.prepare(
            "SELECT id, tabla_id, nombre_columna, tipo_datos, longitud, nullable, descripcion
             FROM columnas WHERE tabla_id = ?1 ORDER BY id",
        )?;
        let rows = statement
            .query_map(params![table_id], |row| {
                Ok(ColumnRecord {
                    id: row.get(0)?,
                    tabla_id: row.get(1)?,
                    nombre_columna: row.get(2)?,
                    tipo_datos: row.get(3)?,
                    longitud: row.get(4)?,
                    nullable: row.get(5)?,
                    descripcion: row.get(6)?,
                })
            })?
            .collect();
        rows
    }

    fn query_references(&self, condition: &str, table_id: i64) -> SqlResult<Vec<ReferenceRecord>> {
        let sql = format!(
            "SELECT id, tabla_origen_id, columna_origen, tabla_destino_id,
                    columna_destino, nombre_constraint
             FROM referencias {} ORDER BY id",
            condition
        );
        let mut statement = self.connection.prepare(&sql)?;
        let rows = statement
            .query_map(params![table_id], |row| {
                Ok(ReferenceRecord {
                    id: row.get(0)?,
                    tabla_origen_id: row.get(1)?,
                    columna_origen: row.get(2)?,
                    tabla_destino_id: row.get(3)?,
                    columna_destino: row.get(4)?,
                    nombre_constraint: row.get(5)?,
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
            "SELECT r.id, r.tabla_origen_id, r.columna_origen, r.tabla_destino_id,
                    r.columna_destino, r.nombre_constraint
             FROM referencias r
             JOIN tablas source ON source.id = r.tabla_origen_id
             JOIN tablas target ON target.id = r.tabla_destino_id
             WHERE source.version_id = ?1
               AND ((source.nombre_tabla = upper(?2) AND target.nombre_tabla = upper(?3))
                 OR (source.nombre_tabla = upper(?3) AND target.nombre_tabla = upper(?2)))
             ORDER BY r.id",
        )?;
        let rows = statement
            .query_map(params![version.id, left, right], |row| {
                Ok(ReferenceRecord {
                    id: row.get(0)?,
                    tabla_origen_id: row.get(1)?,
                    columna_origen: row.get(2)?,
                    tabla_destino_id: row.get(3)?,
                    columna_destino: row.get(4)?,
                    nombre_constraint: row.get(5)?,
                })
            })?
            .collect();
        rows
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_table(name: &str) -> CatalogTable {
        CatalogTable {
            modulo: "Finance".to_owned(),
            nombre_tabla: name.to_owned(),
            descripcion: Some("Entidad de prueba".to_owned()),
            source_url: Some("https://docs.oracle.com/example.html".to_owned()),
            object_type: Some("TABLE".to_owned()),
            columnas: vec![CatalogColumn {
                nombre_columna: "ID".to_owned(),
                tipo_datos: "NUMBER".to_owned(),
                longitud: Some(18),
                nullable: false,
                descripcion: Some("Identificador".to_owned()),
            }],
            ..CatalogTable::default()
        }
    }

    #[test]
    fn crea_esquema_y_recupera_estructura() {
        let db = Database::in_memory().expect("sqlite en memoria");
        let version_id = db.create_version("26B", true).expect("release");
        db.upsert_catalog_table(version_id, &sample_table("PO_HEADERS_ALL"))
            .expect("tabla");
        let structure = db
            .table_structure("PO_HEADERS_ALL")
            .expect("consulta")
            .expect("tabla existente");
        assert_eq!(structure.tabla.nombre_tabla, "PO_HEADERS_ALL");
        assert_eq!(structure.columnas[0].tipo_datos, "NUMBER");
    }

    #[test]
    fn prioriza_nombre_exacto_en_busqueda() {
        let db = Database::in_memory().expect("sqlite en memoria");
        let version_id = db.create_version("26B", true).expect("release");
        db.upsert_catalog_table(version_id, &sample_table("AP_INVOICES_ALL"))
            .expect("tabla");
        let matches = db.search_tables("AP_INVOICES_ALL", 10).expect("búsqueda");
        assert_eq!(matches[0].nombre_tabla, "AP_INVOICES_ALL");
    }
}
