use std::{path::Path, time::Duration};

use tokio_rusqlite::{
    Connection,
    rusqlite::{self, OpenFlags},
};

use crate::error::StartupError;

/// Opens the existing database. It never creates a new file and never
/// runs a schema: `database/database.db` is the source of truth.
pub async fn open(path: &Path) -> Result<Connection, StartupError> {
    if !path.is_file() {
        return Err(StartupError::DatabaseMissing(path.to_path_buf()));
    }

    // No SQLITE_OPEN_CREATE: opening fails instead of creating an empty database.
    let flags = OpenFlags::SQLITE_OPEN_READ_WRITE
        | OpenFlags::SQLITE_OPEN_NO_MUTEX
        | OpenFlags::SQLITE_OPEN_URI;
    let conn = Connection::open_with_flags(path, flags).await?;
    conn.call(configure).await?;
    if !conn.call(migracion_001_aplicada).await? {
        // The app still works (it merges legacy duplicates when a cell is
        // edited), but the database does not prevent new duplicates yet.
        tracing::warn!(
            "migration 001 not applied: run `sqlite3 <db> < migrations/001_consumos_unique.sql` with the app stopped"
        );
    }
    tracing::info!(path = %path.display(), "database initialized");
    Ok(conn)
}

/// True when the UNIQUE (student, product, day) index exists.
fn migracion_001_aplicada(c: &mut rusqlite::Connection) -> rusqlite::Result<bool> {
    c.query_row(
        "SELECT EXISTS (SELECT 1 FROM sqlite_master WHERE type = 'index' AND name = 'ux_consumos_celda')",
        [],
        |r| r.get(0),
    )
}

fn configure(conn: &mut rusqlite::Connection) -> rusqlite::Result<()> {
    let journal_mode: String =
        conn.pragma_update_and_check(None, "journal_mode", "WAL", |row| row.get(0))?;
    conn.pragma_update(None, "synchronous", "NORMAL")?;
    conn.busy_timeout(Duration::from_millis(5000))?;
    // `PRAGMA foreign_key_check` returned no violations on 2026-11-13.
    conn.pragma_update(None, "foreign_keys", "ON")?;
    tracing::debug!(%journal_mode, "database pragmas applied");
    Ok(())
}

/// Cheap round trip used by the health check.
pub async fn ping(conn: &Connection) -> Result<(), tokio_rusqlite::Error> {
    conn.call(|c| c.query_row("SELECT 1", [], |_| Ok(()))).await
}

#[cfg(test)]
mod tests {
    use super::migracion_001_aplicada;
    use crate::test_support::{MIGRACION_001, conexion, conexion_heredada};

    #[test]
    fn detecta_si_la_migracion_001_esta_aplicada() {
        assert!(migracion_001_aplicada(&mut conexion()).unwrap());
        assert!(!migracion_001_aplicada(&mut conexion_heredada()).unwrap());
    }

    #[test]
    fn la_migracion_001_fusiona_duplicados_sin_cambiar_totales_y_es_idempotente() {
        let mut c = conexion_heredada();
        c.execute_batch(
            "INSERT INTO consumos (id_estudiante, id_producto, cantidad, precio_unitario_venta, fecha_consumo) VALUES
               (1, 1, 1, 1.5, '2026-04-12'),
               (1, 1, 2, 1.5, '2026-04-12'),
               (1, 2, 1, 5.0, '2026-04-12'),
               (2, 1, 1, 1.5, '2026-04-12');",
        )
        .unwrap();
        let resumen = |c: &tokio_rusqlite::rusqlite::Connection| -> (i64, f64, i64) {
            c.query_row(
                "SELECT COUNT(*), SUM(total_linea), SUM(cantidad) FROM consumos",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .unwrap()
        };
        assert_eq!(resumen(&c), (4, 11.0, 5));

        c.execute_batch(MIGRACION_001).unwrap();
        // One row less, same money and units; the merged row keeps the oldest id.
        assert_eq!(resumen(&c), (3, 11.0, 5));
        let fusionada: (i64, i64) = c
            .query_row(
                "SELECT id_consumo, cantidad FROM consumos WHERE id_estudiante = 1 AND id_producto = 1",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!(fusionada, (1, 3));
        assert!(migracion_001_aplicada(&mut c).unwrap());

        c.execute_batch(MIGRACION_001).unwrap();
        assert_eq!(resumen(&c), (3, 11.0, 5));
    }
}
