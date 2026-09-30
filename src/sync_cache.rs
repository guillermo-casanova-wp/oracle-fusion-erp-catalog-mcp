use crate::{db::CatalogTable, paths, sync::OracleSource};
use anyhow::{Context, Result};
use flate2::{read::GzDecoder, write::GzEncoder, Compression};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
};

#[derive(Debug, Serialize, Deserialize)]
struct CachedCatalog {
    release: String,
    module: String,
    source_url: String,
    tables: Vec<CatalogTable>,
}

pub fn cache_path(source: &OracleSource) -> Result<PathBuf> {
    let directory = paths::sync_cache_directory()?;
    Ok(directory.join(format!(
        "{}-{}.json.gz",
        source.release,
        source.module.label().to_ascii_lowercase()
    )))
}

pub fn load(source: &OracleSource) -> Result<Option<Vec<CatalogTable>>> {
    let path = cache_path(source)?;
    let bytes = match fs::read(&path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error).context("could not read sync cache"),
    };

    let cached = match decode(&bytes) {
        Ok(cached) => cached,
        Err(_) => {
            let _ = fs::remove_file(path);
            return Ok(None);
        }
    };
    if cached.release != source.release
        || cached.module != source.module.label()
        || cached.source_url != source.index_url.as_str()
    {
        return Ok(None);
    }
    Ok(Some(cached.tables))
}

pub fn store(source: &OracleSource, tables: &[CatalogTable]) -> Result<PathBuf> {
    let path = cache_path(source)?;
    paths::ensure_parent_directory(&path)?;
    let cached = CachedCatalog {
        release: source.release.clone(),
        module: source.module.label().to_owned(),
        source_url: source.index_url.to_string(),
        tables: tables.to_vec(),
    };
    let bytes = encode(&cached)?;
    let temporary = path.with_file_name(format!(
        ".{}.sync-cache-{}",
        path.file_name()
            .and_then(|name| name.to_str())
            .map_or("catalog.json.gz", |name| name),
        std::process::id()
    ));
    let result = (|| -> std::io::Result<()> {
        let mut file = fs::File::create(&temporary)?;
        file.write_all(&bytes)?;
        file.sync_all()?;
        fs::rename(&temporary, &path)
    })()
    .context("could not write sync cache");
    if result.is_err() {
        let _ = fs::remove_file(temporary);
    }
    result?;
    Ok(path)
}

pub fn read_path(source: &OracleSource, path: &Path) -> Result<Vec<CatalogTable>> {
    let bytes = fs::read(path).context("could not read sync cache")?;
    let cached = decode(&bytes).context("could not decode sync cache")?;
    if cached.release != source.release
        || cached.module != source.module.label()
        || cached.source_url != source.index_url.as_str()
    {
        anyhow::bail!("sync cache metadata does not match source");
    }
    Ok(cached.tables)
}

fn encode(cached: &CachedCatalog) -> Result<Vec<u8>> {
    let json = serde_json::to_vec(cached).context("could not serialize sync cache")?;
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder
        .write_all(&json)
        .context("could not compress sync cache")?;
    encoder.finish().context("could not finish sync cache")
}

fn decode(bytes: &[u8]) -> Result<CachedCatalog> {
    let mut decoder = GzDecoder::new(bytes);
    let mut json = Vec::new();
    decoder
        .read_to_end(&mut json)
        .context("could not decompress sync cache")?;
    serde_json::from_slice(&json).context("could not parse sync cache")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sync::OracleModule;
    use tempfile::tempdir;

    #[test]
    fn stores_and_loads_cache_with_source_metadata() {
        let directory = tempdir().expect("temporary directory");
        let source = OracleSource::help_center(OracleModule::Hcm, "26B").expect("source");
        let tables = vec![CatalogTable {
            module: "HCM".to_owned(),
            table_name: "PER_ALL_PEOPLE_F".to_owned(),
            ..CatalogTable::default()
        }];
        let cached = CachedCatalog {
            release: source.release.clone(),
            module: source.module.label().to_owned(),
            source_url: source.index_url.to_string(),
            tables: tables.clone(),
        };
        let bytes = encode(&cached).expect("encode");
        let path = directory.path().join("cache.json.gz");
        fs::write(&path, bytes).expect("write");
        let loaded = read_path(&source, &path).expect("read");
        assert_eq!(loaded.len(), tables.len());
        assert_eq!(loaded[0].table_name, tables[0].table_name);
    }

    #[test]
    fn rejects_cache_from_another_source() {
        let source = OracleSource::help_center(OracleModule::Hcm, "26B").expect("source");
        let cached = CachedCatalog {
            release: "26B".to_owned(),
            module: "HCM".to_owned(),
            source_url: "https://example.invalid".to_owned(),
            tables: Vec::new(),
        };
        let bytes = encode(&cached).expect("encode");
        let directory = tempdir().expect("temporary directory");
        let path = directory.path().join("cache.json.gz");
        fs::write(&path, bytes).expect("write");
        assert!(read_path(&source, &path).is_err());
    }
}
