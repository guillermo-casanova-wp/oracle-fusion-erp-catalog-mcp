use anyhow::{bail, Context, Result};
use flate2::read::GzDecoder;
use futures::StreamExt;
use indicatif::{ProgressBar, ProgressStyle};
use reqwest::Client;
use serde::Deserialize;
use sha2::{Digest, Sha256};
#[cfg(test)]
use std::io::{Cursor, Read};
use std::{
    fs,
    io::Write,
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
        .timeout(Duration::from_secs(10 * 60))
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

    let compressed_path = compressed_path(database_path);
    let download_result = download_asset(
        &client,
        &format!("{base_url}/{}", manifest.asset),
        &compressed_path,
        &manifest.sha256,
    )
    .await;
    if let Err(error) = download_result {
        let _ = fs::remove_file(&compressed_path);
        return Err(error);
    }

    let install_result = replace_database_from_gzip(database_path, &compressed_path);
    let _ = fs::remove_file(&compressed_path);
    install_result?;
    Ok(true)
}

async fn download_asset(
    client: &Client,
    url: &str,
    destination: &Path,
    expected_checksum: &str,
) -> Result<()> {
    let response = client
        .get(url)
        .send()
        .await
        .context("could not download catalog asset")?
        .error_for_status()
        .context("catalog asset returned an error")?;
    let total = response.content_length();
    let progress = total.map_or_else(ProgressBar::new_spinner, ProgressBar::new);
    progress.set_style(total.map_or_else(ProgressStyle::default_spinner, |_| {
        ProgressStyle::with_template("{prefix} {bar:40.cyan/blue} {bytes}/{total_bytes} {eta}")
            .unwrap_or_else(|_| ProgressStyle::default_bar())
            .progress_chars("##-")
    }));
    progress.set_prefix("Downloading catalog");

    let result = async {
        crate::paths::ensure_parent_directory(destination)?;
        let mut file =
            fs::File::create(destination).context("could not create catalog download file")?;
        let mut checksum = Sha256::new();
        let mut stream = response.bytes_stream();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.context("could not read catalog asset")?;
            file.write_all(&chunk)
                .context("could not write catalog download file")?;
            checksum.update(&chunk);
            progress.inc(chunk.len() as u64);
        }
        file.sync_all()
            .context("could not flush catalog download file")?;
        let actual = format!("{:x}", checksum.finalize());
        verify_checksum(&actual, expected_checksum)?;
        Ok::<(), anyhow::Error>(())
    }
    .await;
    progress.finish_and_clear();
    result
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

fn verify_checksum(actual: &str, expected: &str) -> Result<()> {
    if actual != expected.to_ascii_lowercase() {
        bail!("downloaded catalog failed SHA-256 verification");
    }
    Ok(())
}

#[cfg(test)]
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

fn compressed_path(path: &Path) -> PathBuf {
    let filename = path
        .file_name()
        .and_then(|name| name.to_str())
        .map_or("catalog.sqlite", |name| name);
    path.with_file_name(format!(".{filename}.gz-download-{}", std::process::id()))
}

fn replace_database_from_gzip(destination: &Path, compressed: &Path) -> Result<()> {
    crate::paths::ensure_parent_directory(destination)?;
    let temporary = temporary_path(destination);
    let result = (|| -> Result<()> {
        let input = fs::File::open(compressed).context("could not open catalog download")?;
        let mut decoder = GzDecoder::new(input);
        let mut output =
            fs::File::create(&temporary).context("could not create catalog temporary file")?;
        let written =
            std::io::copy(&mut decoder, &mut output).context("could not decompress catalog")?;
        if written == 0 {
            bail!("decompressed catalog is empty");
        }
        output
            .sync_all()
            .context("could not flush catalog temporary file")?;
        fs::rename(&temporary, destination).context("could not replace local catalog")?;
        Ok(())
    })();
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
        let compressed = directory.path().join("catalog.sqlite.gz");
        let file = fs::File::create(&compressed).expect("compressed file");
        let mut encoder = GzEncoder::new(file, Compression::default());
        encoder.write_all(b"database").expect("gzip input");
        encoder.finish().expect("gzip output");
        replace_database_from_gzip(&path, &compressed).expect("replace");
        assert_eq!(fs::read(path).expect("database"), b"database");
    }

    #[test]
    fn verifies_checksum() {
        let bytes = b"catalog";
        let checksum = format!("{:x}", Sha256::digest(bytes));
        verify_checksum(&checksum, &checksum).expect("checksum");
    }

    #[test]
    fn rejects_invalid_checksum() {
        let error =
            verify_checksum(&"a".repeat(64), &"0".repeat(64)).expect_err("checksum failure");
        assert!(error.to_string().contains("SHA-256"));
    }
}
