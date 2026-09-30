use anyhow::{bail, Context, Result};
use flate2::read::GzDecoder;
use reqwest::Client;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::{Cursor, Read, Write},
    path::{Path, PathBuf},
    time::Duration,
};

const OWNER: &str = "thegreatyamori";
const REPOSITORY: &str = "oracle-fusion-erp-catalog-mcp";

#[derive(Debug, Deserialize)]
struct CatalogManifest {
    release: String,
    asset: String,
    sha256: String,
}

pub async fn install(release: &str, database_path: &Path) -> Result<bool> {
    let release = release.trim().to_ascii_uppercase();
    if release.is_empty() {
        bail!("catalog release cannot be empty");
    }

    if database_has_release(database_path, &release) {
        return Ok(false);
    }

    let tag = format!("catalog-{release}");
    let base_url = format!("https://github.com/{OWNER}/{REPOSITORY}/releases/download/{tag}");
    let client = Client::builder()
        .user_agent(format!("{REPOSITORY}/{}", env!("CARGO_PKG_VERSION")))
        .timeout(Duration::from_secs(60))
        .build()
        .context("could not create catalog HTTP client")?;

    let manifest = client
        .get(format!("{base_url}/manifest.json"))
        .send()
        .await
        .context("could not download catalog manifest")?
        .error_for_status()
        .context("catalog manifest returned an error")?
        .json::<CatalogManifest>()
        .await
        .context("could not parse catalog manifest")?;
    validate_manifest(&manifest, &release)?;

    let compressed = client
        .get(format!("{base_url}/{}", manifest.asset))
        .send()
        .await
        .context("could not download catalog asset")?
        .error_for_status()
        .context("catalog asset returned an error")?
        .bytes()
        .await
        .context("could not read catalog asset")?;
    verify_checksum(&compressed, &manifest.sha256)?;

    let database = decompress(&compressed)?;
    replace_database(database_path, &database)?;
    Ok(true)
}

fn database_has_release(path: &Path, release: &str) -> bool {
    if !path.is_file() {
        return false;
    }
    crate::db::Database::open(path)
        .and_then(|database| database.active_version())
        .ok()
        .flatten()
        .is_some_and(|version| version.release_code == release)
}

fn validate_manifest(manifest: &CatalogManifest, release: &str) -> Result<()> {
    if manifest.release.to_ascii_uppercase() != release {
        bail!(
            "catalog manifest release {} does not match requested release {release}",
            manifest.release
        );
    }
    if Path::new(&manifest.asset)
        .file_name()
        .and_then(|name| name.to_str())
        != Some(manifest.asset.as_str())
    {
        bail!("catalog manifest contains an invalid asset name");
    }
    if manifest.sha256.len() != 64
        || !manifest
            .sha256
            .chars()
            .all(|character| character.is_ascii_hexdigit())
    {
        bail!("catalog manifest contains an invalid SHA-256 checksum");
    }
    Ok(())
}

fn verify_checksum(bytes: &[u8], expected: &str) -> Result<()> {
    let actual = format!("{:x}", Sha256::digest(bytes));
    if actual != expected.to_ascii_lowercase() {
        bail!("downloaded catalog failed SHA-256 verification");
    }
    Ok(())
}

fn decompress(bytes: &[u8]) -> Result<Vec<u8>> {
    let mut decoder = GzDecoder::new(Cursor::new(bytes));
    let mut database = Vec::new();
    decoder
        .read_to_end(&mut database)
        .context("could not decompress catalog")?;
    if database.is_empty() {
        bail!("decompressed catalog is empty");
    }
    Ok(database)
}

fn replace_database(path: &Path, database: &[u8]) -> Result<()> {
    crate::paths::ensure_parent_directory(path)?;
    let temporary = temporary_path(path);
    let result = write_and_replace(&temporary, path, database);
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

fn temporary_path(path: &Path) -> PathBuf {
    let filename = path
        .file_name()
        .and_then(|name| name.to_str())
        .map_or("catalog.sqlite", |name| name);
    path.with_file_name(format!(".{filename}.download-{}", std::process::id()))
}

fn write_and_replace(temporary: &Path, destination: &Path, database: &[u8]) -> Result<()> {
    let mut file =
        fs::File::create(temporary).context("could not create catalog temporary file")?;
    file.write_all(database)
        .context("could not write catalog temporary file")?;
    file.sync_all()
        .context("could not flush catalog temporary file")?;
    fs::rename(temporary, destination).context("could not replace local catalog")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use flate2::{write::GzEncoder, Compression};
    use tempfile::tempdir;

    #[test]
    fn validates_manifest_for_requested_release() {
        let manifest = CatalogManifest {
            release: "26B".to_owned(),
            asset: "catalog-26B.sqlite.gz".to_owned(),
            sha256: "a".repeat(64),
        };
        validate_manifest(&manifest, "26B").expect("manifest");
    }

    #[test]
    fn rejects_manifest_path_traversal() {
        let manifest = CatalogManifest {
            release: "26B".to_owned(),
            asset: "../catalog.sqlite.gz".to_owned(),
            sha256: "a".repeat(64),
        };
        assert!(validate_manifest(&manifest, "26B").is_err());
    }

    #[test]
    fn decompresses_catalog_asset() {
        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(b"SQLite database").expect("gzip input");
        let compressed = encoder.finish().expect("gzip output");
        assert_eq!(
            decompress(&compressed).expect("decompress"),
            b"SQLite database"
        );
    }

    #[test]
    fn replaces_database_atomically() {
        let directory = tempdir().expect("temporary directory");
        let path = directory.path().join("catalog.sqlite");
        replace_database(&path, b"database").expect("replace");
        assert_eq!(fs::read(path).expect("database"), b"database");
    }

    #[test]
    fn verifies_checksum() {
        let bytes = b"catalog";
        let checksum = format!("{:x}", Sha256::digest(bytes));
        verify_checksum(bytes, &checksum).expect("checksum");
    }

    #[test]
    fn rejects_invalid_checksum() {
        let error = verify_checksum(b"catalog", &"0".repeat(64)).expect_err("checksum failure");
        assert!(error.to_string().contains("SHA-256"));
    }
}
