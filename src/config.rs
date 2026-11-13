use std::{
    env, fs, io,
    net::{Ipv4Addr, SocketAddr},
    num::NonZeroUsize,
    path::{Path, PathBuf},
};

use serde::Deserialize;

use crate::error::StartupError;

const DEFAULT_CONFIG_PATH: &str = "config/default.toml";

/// Every value has a default built into the binary, so the server starts with
/// no config file at all; the TOML file and `KIOSCO_*` variables override it.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct Config {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub backup: BackupConfig,
    pub auth: AuthConfig,
}

/// Login cookies.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct AuthConfig {
    /// Adds `Secure` to cookies. Turn on in production (HTTPS behind Caddy).
    pub cookie_segura: bool,
}

/// Daily database copies (see `backup.rs`).
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct BackupConfig {
    /// Folder for `database-YYYY-MM-DD.db` copies.
    pub directorio: PathBuf,
    /// How many daily copies to keep (older ones are deleted).
    pub conservar: NonZeroUsize,
}

impl Default for BackupConfig {
    fn default() -> Self {
        Self {
            directorio: PathBuf::from("database/backups"),
            conservar: NonZeroUsize::new(30).expect("30 is non-zero"),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct ServerConfig {
    pub address: SocketAddr,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            address: SocketAddr::from((Ipv4Addr::LOCALHOST, 3200)),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct DatabaseConfig {
    /// Must already exist: the server never creates an empty database.
    pub path: PathBuf,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            path: PathBuf::from("database/database.db"),
        }
    }
}

impl Config {
    /// Built-in defaults, then the TOML file, then `KIOSCO_*` variables.
    ///
    /// `config/default.toml` is optional. A file named in `KIOSCO_CONFIG` is
    /// required: a wrong path there is a mistake, not a request for defaults.
    pub fn load() -> Result<Self, StartupError> {
        let mut config = match env::var("KIOSCO_CONFIG") {
            Ok(ruta) => leer(Path::new(&ruta), true)?,
            Err(_) => leer(Path::new(DEFAULT_CONFIG_PATH), false)?,
        };
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
        if let Ok(value) = env::var("KIOSCO_COOKIE_SECURE") {
            self.auth.cookie_segura = value.parse().map_err(|_| StartupError::InvalidEnv {
                name: "KIOSCO_COOKIE_SECURE",
                value,
            })?;
        }
        Ok(())
    }
}

/// Reads one TOML file over the defaults. A missing file is an error only when `obligatorio`.
fn leer(ruta: &Path, obligatorio: bool) -> Result<Config, StartupError> {
    let texto = match fs::read_to_string(ruta) {
        Ok(texto) => texto,
        Err(error) if error.kind() == io::ErrorKind::NotFound && !obligatorio => {
            return Ok(Config::default());
        }
        Err(source) => {
            return Err(StartupError::ConfigRead {
                path: ruta.display().to_string(),
                source,
            });
        }
    };
    toml::from_str(&texto).map_err(|source| StartupError::ConfigParse {
        path: ruta.display().to_string(),
        source,
    })
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::{Config, leer};
    use crate::error::StartupError;

    fn ruta_temporal(nombre: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("kiosco-config-{}-{nombre}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        dir.join("config.toml")
    }

    fn assert_valores_por_defecto(config: &Config) {
        assert_eq!(config.server.address.to_string(), "127.0.0.1:3200");
        assert_eq!(config.database.path, PathBuf::from("database/database.db"));
        assert_eq!(config.backup.directorio, PathBuf::from("database/backups"));
        assert_eq!(config.backup.conservar.get(), 30);
        assert!(!config.auth.cookie_segura);
    }

    #[test]
    fn sin_archivo_por_defecto_arranca_con_los_valores_del_binario() {
        let config = leer(&ruta_temporal("sin-archivo"), false).unwrap();
        assert_valores_por_defecto(&config);
    }

    #[test]
    fn un_archivo_pedido_explicitamente_que_no_existe_es_un_error() {
        let error = leer(&ruta_temporal("explicito"), true).unwrap_err();
        assert!(matches!(error, StartupError::ConfigRead { .. }), "{error}");
    }

    #[test]
    fn el_archivo_solo_cambia_lo_que_declara() {
        let ruta = ruta_temporal("parcial");
        std::fs::write(&ruta, "[server]\naddress = \"0.0.0.0:8080\"\n").unwrap();
        let config = leer(&ruta, true).unwrap();
        assert_eq!(config.server.address.to_string(), "0.0.0.0:8080");
        assert_eq!(config.database.path, PathBuf::from("database/database.db"));
        assert_eq!(config.backup.conservar.get(), 30);
    }

    #[test]
    fn el_toml_del_repo_coincide_con_los_valores_del_binario() {
        let ruta = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("config/default.toml");
        assert_valores_por_defecto(&leer(&ruta, true).unwrap());
    }
}
