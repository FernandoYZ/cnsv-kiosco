//! Daily database backups.
//!
//! Once per day the app writes `database-YYYY-MM-DD.db` with `VACUUM INTO`,
//! which produces a consistent, compact copy even while the app is writing.
//! Only the newest `conservar` copies are kept. The check runs at startup and
//! every hour, so a copy is made shortly after midnight (or at the first start
//! of the day).

use std::{
    fs,
    path::{Path, PathBuf},
    time::Duration,
};

use chrono::NaiveDate;
use tokio_rusqlite::{
    Connection,
    rusqlite::{self, Connection as SqliteConnection},
};

use crate::{config::BackupConfig, fechas};

const PREFIJO: &str = "database-";
const EXTENSION: &str = ".db";
const INTERVALO: Duration = Duration::from_secs(60 * 60);

#[derive(Debug, thiserror::Error)]
pub enum BackupError {
    #[error("sqlite: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
}

fn nombre(fecha: NaiveDate) -> String {
    format!("{PREFIJO}{}{EXTENSION}", fecha)
}

/// True only for files created by this module (`database-YYYY-MM-DD.db`).
fn es_copia(nombre: &str) -> bool {
    nombre
        .strip_prefix(PREFIJO)
        .and_then(|resto| resto.strip_suffix(EXTENSION))
        .is_some_and(|fecha| NaiveDate::parse_from_str(fecha, "%Y-%m-%d").is_ok())
}

/// Creates today's copy if it does not exist yet and prunes old copies.
/// Returns the path of the copy created now, if any.
pub fn respaldar(
    c: &SqliteConnection,
    directorio: &Path,
    hoy: NaiveDate,
    conservar: usize,
) -> Result<Option<PathBuf>, BackupError> {
    fs::create_dir_all(directorio)?;
    let destino = directorio.join(nombre(hoy));
    let creada = if destino.exists() {
        None
    } else {
        // Write to a temporary name first so a crash never leaves a
        // half-written file that looks like a valid daily copy.
        let temporal = directorio.join(format!("{}.tmp", nombre(hoy)));
        let _ = fs::remove_file(&temporal);
        c.execute("VACUUM INTO ?1", [temporal.to_string_lossy()])?;
        fs::rename(&temporal, &destino)?;
        Some(destino)
    };
    podar(directorio, conservar)?;
    Ok(creada)
}

/// Deletes the oldest daily copies beyond `conservar`. Other files are never touched.
fn podar(directorio: &Path, conservar: usize) -> Result<(), BackupError> {
    let mut copias: Vec<PathBuf> = fs::read_dir(directorio)?
        .filter_map(Result::ok)
        .map(|entrada| entrada.path())
        .filter(|ruta| {
            ruta.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(es_copia)
        })
        .collect();
    // ISO dates sort chronologically as text.
    copias.sort();
    let sobrantes = copias.len().saturating_sub(conservar);
    for ruta in &copias[..sobrantes] {
        fs::remove_file(ruta)?;
        tracing::info!(ruta = %ruta.display(), "old backup deleted");
    }
    Ok(())
}

/// Runs the backup check at startup and then every hour, in the background.
pub fn programar(db: Connection, config: BackupConfig) {
    tokio::spawn(async move {
        let mut intervalo = tokio::time::interval(INTERVALO);
        loop {
            intervalo.tick().await;
            let directorio = config.directorio.clone();
            let conservar = config.conservar.get();
            let hoy = fechas::hoy();
            match db
                .call(move |c| respaldar(c, &directorio, hoy, conservar))
                .await
            {
                Ok(Some(ruta)) => tracing::info!(ruta = %ruta.display(), "daily backup created"),
                Ok(None) => {}
                Err(err) => tracing::error!(error = %err, "daily backup failed"),
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use std::fs;

    use chrono::NaiveDate;
    use tokio_rusqlite::rusqlite::Connection;

    use super::{es_copia, respaldar};
    use crate::test_support::conexion;

    fn directorio_temporal(nombre: &str) -> std::path::PathBuf {
        let ruta =
            std::env::temp_dir().join(format!("kiosco-backup-{nombre}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&ruta);
        ruta
    }

    fn dia(n: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 11, n).unwrap()
    }

    #[test]
    fn crea_una_copia_restaurable_por_dia_y_no_la_repite() {
        let dir = directorio_temporal("diaria");
        let c = conexion();
        c.execute(
            "INSERT INTO pagos (id_estudiante, monto, fecha_pago) VALUES (1, 7.5, '2026-11-09')",
            [],
        )
        .unwrap();

        let creada = respaldar(&c, &dir, dia(13), 30).unwrap().expect("created");
        assert!(creada.ends_with("database-2026-11-13.db"));
        assert_eq!(
            respaldar(&c, &dir, dia(13), 30).unwrap(),
            None,
            "once per day"
        );

        // The copy opens on its own and holds the same data.
        let copia = Connection::open(&creada).unwrap();
        let integridad: String = copia
            .query_row("PRAGMA integrity_check", [], |r| r.get(0))
            .unwrap();
        assert_eq!(integridad, "ok");
        let monto: f64 = copia
            .query_row("SELECT monto FROM pagos", [], |r| r.get(0))
            .unwrap();
        assert_eq!(monto, 7.5);

        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn conserva_solo_las_copias_mas_recientes_y_no_toca_otros_archivos() {
        let dir = directorio_temporal("poda");
        let c = conexion();
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("notas.txt"), "keep me").unwrap();
        fs::write(dir.join("database-manual.db"), "keep me too").unwrap();

        for n in 1..=5 {
            respaldar(&c, &dir, dia(n), 3).unwrap();
        }
        let mut nombres: Vec<String> = fs::read_dir(&dir)
            .unwrap()
            .map(|e| e.unwrap().file_name().into_string().unwrap())
            .collect();
        nombres.sort();
        assert_eq!(
            nombres,
            vec![
                "database-2026-11-03.db",
                "database-2026-11-04.db",
                "database-2026-11-05.db",
                "database-manual.db",
                "notas.txt",
            ]
        );

        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn solo_reconoce_nombres_de_copia_validos() {
        assert!(es_copia("database-2026-11-13.db"));
        assert!(!es_copia("database-2026-13-40.db"));
        assert!(!es_copia("database-2026-11-13.db.tmp"));
        assert!(!es_copia("database.db"));
    }
}
