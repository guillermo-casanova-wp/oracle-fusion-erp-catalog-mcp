use anyhow::{anyhow, Result};
use directories::BaseDirs;
use std::{
    env,
    path::{Path, PathBuf},
};

const APPLICATION_DIRECTORY: &str = "oracle-fusion-erp-catalog-mcp";
const DATABASE_FILENAME: &str = "catalog.sqlite";

pub fn database_path() -> Result<PathBuf> {
    if let Some(value) = env::var_os("ORACLE_MCP_DATABASE").filter(|value| !value.is_empty()) {
        return Ok(PathBuf::from(value));
    }

    let base_dirs =
        BaseDirs::new().ok_or_else(|| anyhow!("could not determine user data directory"))?;
    let platform_path = base_dirs
        .data_dir()
        .join(APPLICATION_DIRECTORY)
        .join(DATABASE_FILENAME);
    Ok(platform_path)
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
