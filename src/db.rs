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
    tracing::info!(path = %path.display(), "database ready");
    Ok(conn)
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
