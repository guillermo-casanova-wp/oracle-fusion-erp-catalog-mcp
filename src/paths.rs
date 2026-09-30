use anyhow::{anyhow, Result};
use directories::BaseDirs;
use std::{
    env,
    ffi::OsStr,
    path::{Path, PathBuf},
};

const APPLICATION_DIRECTORY: &str = "oracle-fusion-erp-catalog-mcp";
const DATABASE_FILENAME: &str = "catalog.sqlite";
const LEGACY_DATABASE_FILENAME: &str = "oracle-erp-mcp.sqlite";

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
    let legacy_path = env::current_dir()?.join(LEGACY_DATABASE_FILENAME);
    Ok(resolve_database_path(None, platform_path, legacy_path))
}

fn resolve_database_path(
    override_path: Option<&OsStr>,
    platform_path: PathBuf,
    legacy_path: PathBuf,
) -> PathBuf {
    if let Some(path) = override_path.filter(|path| !path.is_empty()) {
        return PathBuf::from(path);
    }
    if platform_path.exists() || !legacy_path.exists() {
        platform_path
    } else {
        legacy_path
    }
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
    use tempfile::tempdir;

    #[test]
    fn prefers_platform_path_when_no_legacy_database_exists() {
        let platform = PathBuf::from("/data/oracle-fusion-erp-catalog-mcp/catalog.sqlite");
        let legacy = PathBuf::from("/project/oracle-erp-mcp.sqlite");
        assert_eq!(
            resolve_database_path(None, platform.clone(), legacy),
            platform
        );
    }

    #[test]
    fn falls_back_to_existing_legacy_database() {
        let directory = tempdir().expect("tempdir");
        let platform = directory.path().join("platform/catalog.sqlite");
        let legacy = directory.path().join("oracle-erp-mcp.sqlite");
        std::fs::write(&legacy, b"legacy").expect("legacy database");
        assert_eq!(
            resolve_database_path(None, platform, legacy.clone()),
            legacy
        );
    }

    #[test]
    fn environment_override_wins() {
        let override_path = OsStr::new("/custom/catalog.sqlite");
        assert_eq!(
            resolve_database_path(
                Some(override_path),
                PathBuf::from("/platform/catalog.sqlite"),
                PathBuf::from("/legacy/catalog.sqlite"),
            ),
            PathBuf::from("/custom/catalog.sqlite")
        );
    }
}
