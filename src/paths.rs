use anyhow::{anyhow, Result};
use directories::BaseDirs;
use std::{
    env,
    path::{Path, PathBuf},
};

const APPLICATION_DIRECTORY: &str = "oracle-fusion-erp-catalog-mcp";
const DATABASE_FILENAME: &str = "catalog.sqlite";
const UPDATE_CACHE_FILENAME: &str = "update-check.json";

pub fn data_directory() -> Result<PathBuf> {
    if let Some(value) = env::var_os("ORACLE_MCP_DATABASE").filter(|value| !value.is_empty()) {
        let database_path = PathBuf::from(value);
        return Ok(database_path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .map_or_else(|| PathBuf::from("."), Path::to_path_buf));
    }

    let base_dirs =
        BaseDirs::new().ok_or_else(|| anyhow!("could not determine user data directory"))?;
    Ok(base_dirs.data_dir().join(APPLICATION_DIRECTORY))
}

pub fn database_path() -> Result<PathBuf> {
    Ok(data_directory()?.join(DATABASE_FILENAME))
}

pub fn update_cache_path() -> Result<PathBuf> {
    Ok(data_directory()?.join(UPDATE_CACHE_FILENAME))
}

pub fn sync_cache_directory() -> Result<PathBuf> {
    Ok(data_directory()?.join("sync-cache"))
}

pub fn ensure_parent_directory(path: &Path) -> std::io::Result<()> {
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        std::fs::create_dir_all(parent)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn uses_platform_database_filename() {
        assert_eq!(DATABASE_FILENAME, "catalog.sqlite");
    }
}
