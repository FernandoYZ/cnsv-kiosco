use std::{env, fs, net::SocketAddr, path::PathBuf};

use serde::Deserialize;

use crate::error::StartupError;

const DEFAULT_CONFIG_PATH: &str = "config/default.toml";

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    #[serde(default)]
    pub backup: BackupConfig,
}

/// Daily database copies (see `backup.rs`).
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct BackupConfig {
    /// Folder for `database-YYYY-MM-DD.db` copies.
    pub directorio: PathBuf,
    /// How many daily copies to keep (older ones are deleted).
    pub conservar: usize,
}

impl Default for BackupConfig {
    fn default() -> Self {
        Self {
            directorio: PathBuf::from("database/backups"),
            conservar: 30,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    pub address: SocketAddr,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DatabaseConfig {
    pub path: PathBuf,
}

impl Config {
    /// Reads the TOML file (`KIOSCO_CONFIG` or `config/default.toml`)
    /// and then applies environment variable overrides.
    pub fn load() -> Result<Self, StartupError> {
        let path = env::var("KIOSCO_CONFIG").unwrap_or_else(|_| DEFAULT_CONFIG_PATH.to_owned());
        let raw = fs::read_to_string(&path).map_err(|source| StartupError::ConfigRead {
            path: path.clone(),
            source,
        })?;
        let mut config: Config =
            toml::from_str(&raw).map_err(|source| StartupError::ConfigParse { path, source })?;
        config.apply_env_overrides()?;
        Ok(config)
    }

    fn apply_env_overrides(&mut self) -> Result<(), StartupError> {
        if let Ok(value) = env::var("KIOSCO_SERVER_ADDRESS") {
            self.server.address = value.parse().map_err(|_| StartupError::InvalidEnv {
                name: "KIOSCO_SERVER_ADDRESS",
                value,
            })?;
        }
        if let Ok(value) = env::var("KIOSCO_DATABASE_PATH") {
            self.database.path = PathBuf::from(value);
        }
        if let Ok(value) = env::var("KIOSCO_BACKUP_DIR") {
            self.backup.directorio = PathBuf::from(value);
        }
        if self.backup.conservar == 0 {
            return Err(StartupError::InvalidEnv {
                name: "backup.conservar",
                value: "0 (must keep at least one copy)".to_owned(),
            });
        }
        Ok(())
    }
}
