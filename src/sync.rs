use crate::db::{CatalogTable, Database};
use anyhow::{anyhow, Context, Result};
use indicatif::ProgressBar;
use reqwest::Client;
use scraper::{Html, Selector};
use serde::Deserialize;
use std::{collections::BTreeSet, time::Duration};
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

    pub fn label(self) -> &'static str {
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
            client: Client::builder()
                .user_agent("oracle-fusion-erp-catalog-mcp/0.1")
                .timeout(Duration::from_secs(30))
                .build()?,
        })
    }

    pub async fn extract(&self, source: &OracleSource) -> Result<Vec<CatalogTable>> {
        self.extract_with_progress(source, None).await
    }

    pub async fn extract_with_progress(
        &self,
        source: &OracleSource,
        progress: Option<&ProgressBar>,
    ) -> Result<Vec<CatalogTable>> {
        let response = self
            .client
            .get(source.index_url.clone())
            .send()
            .await
            .with_context(|| format!("downloading {}", source.index_url))?
            .error_for_status()?;
        let content_type = response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .unwrap_or("")
            .to_ascii_lowercase();
        let bytes = response.bytes().await?;
        if content_type.contains("json") || bytes.first() == Some(&b'{') {
            let document: JsonCatalog = serde_json::from_slice(&bytes)?;
            return Ok(document.tables);
        }
        if content_type.contains("xml") || bytes.starts_with(b"<?xml") {
            return parse_xml_catalog(&bytes);
        }
        self.extract_html_guide(&bytes, source, progress).await
    }

    async fn extract_html_guide(
        &self,
        initial_payload: &[u8],
        source: &OracleSource,
        progress: Option<&ProgressBar>,
    ) -> Result<Vec<CatalogTable>> {
        let next_selector = Selector::parse(r#"link[rel="next"]"#)
            .map_err(|error| anyhow!("invalid selector: {error}"))?;
        let prefix = format!(
            "/en/cloud/saas/{}/{}/{}/",
            source.module.path(),
            source.release,
            source.module.guide()
        );
        let mut current_url = source.index_url.clone();
        let mut current_payload = initial_payload.to_vec();
        let mut seen = BTreeSet::new();
        let mut tables = Vec::new();

        for page_number in 0..10_000 {
            if !seen.insert(current_url.as_str().to_owned()) {
                break;
            }
            if let Some(table) = parse_table_page(&current_payload, &current_url, source)? {
                tables.push(table);
            }
            if page_number > 0 && page_number % 100 == 0 {
                let message = format!(
                    "processed {} Oracle guide pages and {} tables",
                    page_number,
                    tables.len()
                );
                if let Some(progress) = progress {
                    progress.set_message(message);
                } else {
                    eprintln!("{message}");
                }
            }

            let document = Html::parse_document(
                std::str::from_utf8(&current_payload).context("HTML page is not UTF-8")?,
            );
            let Some(href) = document
                .select(&next_selector)
                .next()
                .and_then(|link| link.value().attr("href"))
            else {
                break;
            };
            let next_url = current_url.join(href)?;
            if next_url.host_str() != Some("docs.oracle.com")
                || !next_url.path().starts_with(&prefix)
            {
                break;
            }
            current_payload = self
                .client
                .get(next_url.clone())
                .send()
                .await
                .with_context(|| format!("downloading {}", next_url))?
                .error_for_status()?
                .bytes()
                .await?
                .to_vec();
            current_url = next_url;
        }
        Ok(tables)
    }
}

fn parse_table_page(
    payload: &[u8],
    page_url: &Url,
    source: &OracleSource,
) -> Result<Option<CatalogTable>> {
    let html = std::str::from_utf8(payload).context("HTML page is not UTF-8")?;
    let document = Html::parse_document(html);
    let columns_selector = Selector::parse(r#"table[summary="Columns"]"#)
        .map_err(|error| anyhow!("invalid selector: {error}"))?;
    let Some(columns_table) = document.select(&columns_selector).next() else {
        return Ok(None);
    };
    let title_selector = Selector::parse("h1 .chapterstart")
        .map_err(|error| anyhow!("invalid selector: {error}"))?;
    let Some(title) = document.select(&title_selector).next() else {
        return Ok(None);
    };
    let table_name = text_content(title).to_ascii_uppercase();
    if table_name.is_empty() {
        return Ok(None);
    }

    let description_selector = Selector::parse(r#"meta[name="description"]"#)
        .map_err(|error| anyhow!("invalid selector: {error}"))?;
    let description = document
        .select(&description_selector)
        .next()
        .and_then(|meta| meta.value().attr("content"))
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned);
    let row_selector =
        Selector::parse("tbody tr").map_err(|error| anyhow!("invalid selector: {error}"))?;
    let cell_selector =
        Selector::parse("td").map_err(|error| anyhow!("invalid selector: {error}"))?;
    let mut columns = Vec::new();
    for row in columns_table.select(&row_selector) {
        let cells: Vec<_> = row.select(&cell_selector).collect();
        if cells.len() < 6 {
            continue;
        }
        let column_name = text_content(cells[0]).to_ascii_uppercase();
        if column_name.is_empty() {
            continue;
        }
        let length_text = text_content(cells[2]);
        let precision_text = text_content(cells[3]);
        let length = length_text
            .parse()
            .ok()
            .or_else(|| precision_text.parse().ok());
        columns.push(crate::db::CatalogColumn {
            column_name,
            data_type: text_content(cells[1]),
            length,
            nullable: !text_content(cells[4]).eq_ignore_ascii_case("yes"),
            description: non_empty_text(cells[5]),
        });
    }

    let references = parse_foreign_keys(&document, &row_selector, &cell_selector)?;
    let indexes = parse_indexes(&document, &row_selector, &cell_selector)?;
    Ok(Some(CatalogTable {
        module: source.module.label().to_owned(),
        table_name,
        description,
        source_url: Some(page_url.to_string()),
        object_type: Some("TABLE".to_owned()),
        columns,
        references,
        indexes,
    }))
}

fn parse_foreign_keys(
    document: &Html,
    row_selector: &Selector,
    cell_selector: &Selector,
) -> Result<Vec<crate::db::CatalogReference>> {
    let foreign_keys_selector = Selector::parse(r#"table[summary="Foreign Keys"]"#)
        .map_err(|error| anyhow!("invalid selector: {error}"))?;
    let Some(foreign_keys_table) = document.select(&foreign_keys_selector).next() else {
        return Ok(Vec::new());
    };
    let mut references = Vec::new();
    for row in foreign_keys_table.select(row_selector) {
        let cells: Vec<_> = row.select(cell_selector).collect();
        if cells.len() < 3 {
            continue;
        }
        let target_table = text_content(cells[1]).to_ascii_uppercase();
        let source_column = text_content(cells[2]).to_ascii_uppercase();
        if target_table.is_empty() || source_column.is_empty() {
            continue;
        }
        references.push(crate::db::CatalogReference {
            target_table,
            source_column,
            target_column: None,
            constraint_name: None,
        });
    }
    Ok(references)
}

fn parse_indexes(
    document: &Html,
    row_selector: &Selector,
    cell_selector: &Selector,
) -> Result<Vec<crate::db::CatalogIndex>> {
    let indexes_selector = Selector::parse(r#"table[summary="Indexes"]"#)
        .map_err(|error| anyhow!("invalid selector: {error}"))?;
    let Some(indexes_table) = document.select(&indexes_selector).next() else {
        return Ok(Vec::new());
    };
    let mut indexes = Vec::new();
    for row in indexes_table.select(row_selector) {
        let cells: Vec<_> = row.select(cell_selector).collect();
        if cells.len() < 4 {
            continue;
        }
        let index_name = text_content(cells[0]);
        if index_name.is_empty() {
            continue;
        }
        indexes.push(crate::db::CatalogIndex {
            index_name,
            indexed_columns: text_content(cells[3])
                .split(',')
                .map(str::trim)
                .filter(|column| !column.is_empty())
                .map(str::to_owned)
                .collect(),
            is_unique: text_content(cells[1]).eq_ignore_ascii_case("unique"),
        });
    }
    Ok(indexes)
}

fn text_content(element: scraper::ElementRef<'_>) -> String {
    element
        .text()
        .collect::<Vec<_>>()
        .join(" ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn non_empty_text(element: scraper::ElementRef<'_>) -> Option<String> {
    let text = text_content(element);
    (!text.is_empty()).then_some(text)
}

fn parse_xml_catalog(payload: &[u8]) -> Result<Vec<CatalogTable>> {
    #[derive(Debug, Deserialize)]
    struct XmlCatalog {
        #[serde(rename = "table", default)]
        tables: Vec<CatalogTable>,
    }
    let document: XmlCatalog = quick_xml::de::from_reader(payload)?;
    Ok(document.tables)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct ReleaseCode {
    year: u16,
    cycle: u8,
}

fn parse_release_code(value: &str) -> Result<ReleaseCode> {
    let normalized = value.trim().to_ascii_uppercase();
    if normalized.len() < 2 {
        return Err(anyhow!("invalid release code: {value}"));
    }
    let (year, cycle) = normalized.split_at(normalized.len() - 1);
    if !year.chars().all(|character| character.is_ascii_digit())
        || !cycle.as_bytes()[0].is_ascii_uppercase()
    {
        return Err(anyhow!("invalid release code: {value}"));
    }
    Ok(ReleaseCode {
        year: year.parse()?,
        cycle: cycle.as_bytes()[0] - b'A',
    })
}

fn is_superior_release(candidate: &str, current: &str) -> Result<bool> {
    Ok(parse_release_code(candidate)? > parse_release_code(current)?)
}

pub fn synchronize(
    db: &Database,
    release: &str,
    tables: Vec<CatalogTable>,
    activate: bool,
) -> Result<i64> {
    synchronize_with_progress(db, release, tables, activate, |_, _| {})
}

pub fn synchronize_with_progress<F>(
    db: &Database,
    release: &str,
    mut tables: Vec<CatalogTable>,
    activate: bool,
    mut on_progress: F,
) -> Result<i64>
where
    F: FnMut(usize, usize),
{
    let release = release.trim().to_ascii_uppercase();
    parse_release_code(&release)?;
    let previous = db.active_version()?;
    let target = db.version_by_release(&release)?;
    let incoming_modules: BTreeSet<_> = tables.iter().map(|table| table.module.clone()).collect();
    if incoming_modules.is_empty() {
        return Err(anyhow!("sync produced no modules"));
    }

    let version_id = if let Some(existing) = target {
        let existing_modules = db.modules_for_version(existing.id)?;
        if incoming_modules
            .iter()
            .any(|module| existing_modules.contains(module))
        {
            return Err(anyhow!(
                "release {release} already contains one of the synchronized modules"
            ));
        }
        if let Some(previous) = &previous {
            if !is_superior_release(&release, &previous.release_code)? && previous.id != existing.id
            {
                return Err(anyhow!(
                    "release {release} is not superior to active release {}",
                    previous.release_code
                ));
            }
        }
        existing.id
    } else if let Some(previous) = &previous {
        if !is_superior_release(&release, &previous.release_code)? {
            return Err(anyhow!(
                "release {release} is not superior to active release {}",
                previous.release_code
            ));
        }
        db.clone_version(previous.id, &release)?
    } else {
        db.create_version(&release, activate)?
    };
    for table in &mut tables {
        table.table_name = table.table_name.to_ascii_uppercase();
        let mut skeleton = table.clone();
        skeleton.columns.clear();
        skeleton.references.clear();
        skeleton.indexes.clear();
        db.upsert_catalog_table(version_id, &skeleton)?;
    }
    let total_tables = tables.len();
    for (index, table) in tables.iter().enumerate() {
        db.upsert_catalog_table(version_id, table)?;
        on_progress(index + 1, total_tables);
    }
    db.rebuild_fts(version_id)?;
    if activate {
        db.activate_version(version_id)?;
        if let Some(previous) = previous {
            if previous.id != version_id && is_superior_release(&release, &previous.release_code)? {
                db.delete_version_by_release(&previous.release_code)?;
            }
        }
    }
    Ok(version_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn catalog_table(module: &str, table_name: &str) -> CatalogTable {
        CatalogTable {
            module: module.to_owned(),
            table_name: table_name.to_owned(),
            ..CatalogTable::default()
        }
    }

    #[test]
    fn orders_oracle_releases() {
        assert!(is_superior_release("26C", "26B").expect("release"));
        assert!(is_superior_release("27A", "26C").expect("release"));
        assert!(!is_superior_release("26B", "26C").expect("release"));
        assert_eq!(
            parse_release_code("26B").expect("release"),
            ReleaseCode { year: 26, cycle: 1 }
        );
    }

    #[test]
    fn rejects_non_superior_release() {
        let db = Database::in_memory().expect("in-memory SQLite");
        db.create_version("26C", true).expect("release");
        let error = synchronize(&db, "26B", vec![catalog_table("SCM", "OLDER_TABLE")], true)
            .expect_err("downgrade");
        assert!(error.to_string().contains("not superior"));
    }

    #[test]
    fn merges_missing_module_into_existing_release() {
        let db = Database::in_memory().expect("in-memory SQLite");
        let version_id = db.create_version("26B", true).expect("release");
        db.upsert_catalog_table(version_id, &catalog_table("SCM", "SCM_TABLE"))
            .expect("SCM table");

        synchronize(
            &db,
            "26B",
            vec![catalog_table("FINANCIALS", "FINANCIALS_TABLE")],
            true,
        )
        .expect("module merge");

        let modules = db.modules_for_version(version_id).expect("modules");
        assert!(modules.contains("SCM"));
        assert!(modules.contains("FINANCIALS"));
    }

    #[test]
    fn prunes_previous_active_release_after_upgrade() {
        let db = Database::in_memory().expect("in-memory SQLite");
        let version_id = db.create_version("26B", true).expect("release");
        db.upsert_catalog_table(version_id, &catalog_table("SCM", "OLD_TABLE"))
            .expect("old table");

        synchronize(&db, "26C", vec![catalog_table("SCM", "NEW_TABLE")], true).expect("upgrade");

        assert!(db.version_by_release("26B").expect("old release").is_none());
        assert!(db.version_by_release("26C").expect("new release").is_some());
    }

    #[test]
    fn keeps_previous_release_when_not_activating() {
        let db = Database::in_memory().expect("in-memory SQLite");
        let version_id = db.create_version("26B", true).expect("release");
        db.upsert_catalog_table(version_id, &catalog_table("SCM", "OLD_TABLE"))
            .expect("old table");

        synchronize(&db, "26C", vec![catalog_table("SCM", "NEW_TABLE")], false)
            .expect("inactive upgrade");

        assert!(db.version_by_release("26B").expect("old release").is_some());
        assert!(
            !db.version_by_release("26C")
                .expect("new release")
                .expect("new version")
                .active
        );
    }

    #[test]
    fn reports_progress_for_each_imported_table() {
        let db = Database::in_memory().expect("in-memory SQLite");
        let tables = vec![
            catalog_table("SCM", "FIRST_TABLE"),
            catalog_table("SCM", "SECOND_TABLE"),
        ];
        let mut progress = Vec::new();

        synchronize_with_progress(&db, "26B", tables, true, |completed, total| {
            progress.push((completed, total));
        })
        .expect("synchronize");

        assert_eq!(progress, vec![(1, 2), (2, 2)]);
    }

    #[test]
    fn parses_oracle_table_page() {
        let html = br#"
            <html>
              <head>
                <meta name="description" content="Receipt shipment lines">
              </head>
              <body>
                <h1><span class="chapterstart">RCV_SHIPMENT_LINES</span></h1>
                <table summary="Columns">
                  <tbody>
                    <tr>
                      <td>SHIPMENT_LINE_ID</td><td>NUMBER</td><td></td><td>18</td>
                      <td>Yes</td><td>Primary key</td><td></td>
                    </tr>
                  </tbody>
                </table>
                <table summary="Foreign Keys">
                  <tbody>
                    <tr>
                      <td>RCV_SHIPMENT_LINES</td><td>rcv_shipments</td>
                      <td>SHIPMENT_ID</td>
                    </tr>
                  </tbody>
                </table>
                <table summary="Indexes">
                  <tbody>
                    <tr>
                      <td>RCV_SHIPMENT_LINES_U1</td><td>Unique</td><td>Default</td>
                      <td>SHIPMENT_LINE_ID</td><td></td>
                    </tr>
                  </tbody>
                </table>
              </body>
            </html>
        "#;
        let source = OracleSource::help_center(OracleModule::Scm, "26B").expect("source");
        let page_url = Url::parse(
            "https://docs.oracle.com/en/cloud/saas/supply-chain-and-manufacturing/26b/oedsc/rcvshipmentlines-24402.html",
        )
        .expect("page URL");
        let table = parse_table_page(html, &page_url, &source)
            .expect("page parse")
            .expect("table page");

        assert_eq!(table.table_name, "RCV_SHIPMENT_LINES");
        assert_eq!(table.columns[0].column_name, "SHIPMENT_LINE_ID");
        assert_eq!(table.columns[0].length, Some(18));
        assert!(!table.columns[0].nullable);
        assert_eq!(table.references.len(), 1);
        assert_eq!(table.references[0].target_table, "RCV_SHIPMENTS");
        assert_eq!(table.references[0].source_column, "SHIPMENT_ID");
        assert_eq!(table.references[0].target_column, None);
        assert_eq!(table.indexes[0].index_name, "RCV_SHIPMENT_LINES_U1");
        assert!(table.indexes[0].is_unique);
    }
}
