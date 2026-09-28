use crate::db::{CatalogTable, Database, TableStructure};
use anyhow::{anyhow, Context, Result};
use reqwest::Client;
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use url::Url;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OracleModule {
    Financials,
    Scm,
}

impl OracleModule {
    fn path(self) -> &'static str {
        match self {
            Self::Financials => "financials",
            Self::Scm => "supply-chain-and-manufacturing",
        }
    }

    fn guide(self) -> &'static str {
        match self {
            Self::Financials => "oedmf",
            Self::Scm => "oedsc",
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Financials => "FINANCIALS",
            Self::Scm => "SCM",
        }
    }
}

#[derive(Debug, Clone)]
pub struct OracleSource {
    pub module: OracleModule,
    pub release: String,
    pub index_url: Url,
}

impl OracleSource {
    pub fn help_center(module: OracleModule, release: &str) -> Result<Self> {
        let release = release.to_ascii_lowercase();
        let url = format!(
            "https://docs.oracle.com/en/cloud/saas/{}/{}/{}/index.html",
            module.path(),
            release,
            module.guide()
        );
        Ok(Self {
            module,
            release,
            index_url: Url::parse(&url)?,
        })
    }
}

#[derive(Debug, Deserialize)]
struct JsonCatalog {
    #[serde(default)]
    tables: Vec<CatalogTable>,
}

pub struct OracleExtractor {
    client: Client,
}

impl OracleExtractor {
    pub fn new() -> Result<Self> {
        Ok(Self {
            client: Client::builder().user_agent("oracle-erp-mcp/0.1").build()?,
        })
    }

    pub async fn extract(&self, source: &OracleSource) -> Result<Vec<CatalogTable>> {
        let response = self
            .client
            .get(source.index_url.clone())
            .send()
            .await
            .with_context(|| format!("descargando {}", source.index_url))?
            .error_for_status()?;
        let content_type = response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .unwrap_or("")
            .to_ascii_lowercase();
        let bytes = response.bytes().await?;
        self.parse_payload(&bytes, &content_type, source)
    }

    pub fn parse_payload(
        &self,
        payload: &[u8],
        content_type: &str,
        source: &OracleSource,
    ) -> Result<Vec<CatalogTable>> {
        if content_type.contains("json") || payload.first() == Some(&b'{') {
            let document: JsonCatalog = serde_json::from_slice(payload)?;
            return Ok(document.tables);
        }
        if content_type.contains("xml") || payload.starts_with(b"<?xml") {
            return parse_xml_catalog(payload);
        }
        parse_html_index(payload, source)
    }
}

fn parse_html_index(payload: &[u8], source: &OracleSource) -> Result<Vec<CatalogTable>> {
    let html = std::str::from_utf8(payload).context("índice HTML no es UTF-8")?;
    let document = Html::parse_document(html);
    let selector = Selector::parse("a").map_err(|error| anyhow!("selector inválido: {error}"))?;
    let prefix = format!(
        "/en/cloud/saas/{}/{}/{}/",
        source.module.path(),
        source.release,
        source.module.guide()
    );
    let mut seen = BTreeSet::new();
    let mut result = Vec::new();
    for anchor in document.select(&selector) {
        let Some(href) = anchor.value().attr("href") else {
            continue;
        };
        let Ok(url) = source.index_url.join(href) else {
            continue;
        };
        if url.host_str() != Some("docs.oracle.com")
            || !url.path().starts_with(&prefix)
            || !url.path().ends_with(".html")
            || !seen.insert(url.as_str().to_owned())
        {
            continue;
        }
        let Some(slug) = url.path().rsplit('/').next() else {
            continue;
        };
        let name = slug
            .trim_end_matches(".html")
            .rsplit_once('-')
            .map(|(value, _)| value)
            .unwrap_or(slug.trim_end_matches(".html"))
            .replace('-', "_")
            .to_ascii_uppercase();
        if name == "INDEX" || name == "OVERVIEW" {
            continue;
        }
        result.push(CatalogTable {
            modulo: source.module.label().to_owned(),
            nombre_tabla: name,
            descripcion: Some(anchor.text().collect::<String>().trim().to_owned())
                .filter(|value| !value.is_empty()),
            source_url: Some(url.to_string()),
            object_type: None,
            ..CatalogTable::default()
        });
    }
    Ok(result)
}

fn parse_xml_catalog(payload: &[u8]) -> Result<Vec<CatalogTable>> {
    #[derive(Debug, Deserialize)]
    struct XmlCatalog {
        #[serde(rename = "table", default)]
        tables: Vec<CatalogTable>,
    }
    Ok(quick_xml::de::from_reader(payload)?)
}

pub fn synchronize(
    db: &Database,
    release: &str,
    mut tables: Vec<CatalogTable>,
    activate: bool,
) -> Result<i64> {
    if db.version_by_release(release)?.is_some() {
        return Err(anyhow!("el release {release} ya existe"));
    }
    let version_id = if let Some(previous) = db.active_version()? {
        db.clone_version(previous.id, release)?
    } else {
        db.create_version(release, activate)?
    };
    for table in &mut tables {
        table.nombre_tabla = table.nombre_tabla.to_ascii_uppercase();
        let mut skeleton = table.clone();
        skeleton.columnas.clear();
        skeleton.referencias.clear();
        skeleton.indices.clear();
        db.upsert_catalog_table(version_id, &skeleton)?;
    }
    for table in &tables {
        db.upsert_catalog_table(version_id, table)?;
    }
    db.rebuild_fts(version_id)?;
    if activate {
        db.activate_version(version_id)?;
    }
    Ok(version_id)
}

#[derive(Debug, Clone, Serialize)]
pub struct DiffReport {
    pub from_release: String,
    pub to_release: String,
    pub new_tables: Vec<String>,
    pub removed_tables: Vec<String>,
    pub added_columns: Vec<String>,
    pub removed_columns: Vec<String>,
    pub changed_columns: Vec<String>,
}

pub fn diff_versions(db: &Database, from_id: i64, to_id: i64) -> Result<DiffReport> {
    let from = db
        .version_by_id(from_id)?
        .ok_or_else(|| anyhow!("versión origen inexistente"))?;
    let to = db
        .version_by_id(to_id)?
        .ok_or_else(|| anyhow!("versión destino inexistente"))?;
    let from_tables = db.tables_for_version(from_id)?;
    let to_tables = db.tables_for_version(to_id)?;
    let from_map: BTreeMap<_, _> = from_tables
        .iter()
        .map(|table| (table.nombre_tabla.clone(), table))
        .collect();
    let to_map: BTreeMap<_, _> = to_tables
        .iter()
        .map(|table| (table.nombre_tabla.clone(), table))
        .collect();
    let new_tables = to_map
        .keys()
        .filter(|name| !from_map.contains_key(*name))
        .cloned()
        .collect();
    let removed_tables = from_map
        .keys()
        .filter(|name| !to_map.contains_key(*name))
        .cloned()
        .collect();
    let mut report = DiffReport {
        from_release: from.release_code,
        to_release: to.release_code,
        new_tables,
        removed_tables,
        added_columns: Vec::new(),
        removed_columns: Vec::new(),
        changed_columns: Vec::new(),
    };
    for name in from_map.keys().filter(|name| to_map.contains_key(*name)) {
        let old = db
            .table_structure_in_version(from_id, name)?
            .ok_or_else(|| anyhow!("tabla origen inconsistente: {name}"))?;
        let new = db
            .table_structure_in_version(to_id, name)?
            .ok_or_else(|| anyhow!("tabla destino inconsistente: {name}"))?;
        compare_columns(name, &old, &new, &mut report);
    }
    Ok(report)
}

fn compare_columns(
    name: &str,
    old: &TableStructure,
    new: &TableStructure,
    report: &mut DiffReport,
) {
    let old_map: BTreeMap<_, _> = old
        .columnas
        .iter()
        .map(|column| (column.nombre_columna.clone(), column))
        .collect();
    let new_map: BTreeMap<_, _> = new
        .columnas
        .iter()
        .map(|column| (column.nombre_columna.clone(), column))
        .collect();
    for column in new_map.keys().filter(|key| !old_map.contains_key(*key)) {
        report.added_columns.push(format!("{name}.{column}"));
    }
    for column in old_map.keys().filter(|key| !new_map.contains_key(*key)) {
        report.removed_columns.push(format!("{name}.{column}"));
    }
    for column in old_map.keys().filter(|key| new_map.contains_key(*key)) {
        let old_column = old_map[column];
        let new_column = new_map[column];
        if old_column.tipo_datos != new_column.tipo_datos
            || old_column.longitud != new_column.longitud
            || old_column.nullable != new_column.nullable
        {
            report.changed_columns.push(format!(
                "{name}.{column}: {} -> {}",
                old_column.tipo_datos, new_column.tipo_datos
            ));
        }
    }
}
