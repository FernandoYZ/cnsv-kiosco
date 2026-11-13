use chrono::NaiveDate;
use tokio_rusqlite::rusqlite::{self, Connection, params};

use super::Pago;

/// ObtenerPagosSemanaDetalle: newest first.
pub fn de_estudiante_entre(
    c: &Connection,
    id_estudiante: i64,
    inicio: NaiveDate,
    fin: NaiveDate,
) -> rusqlite::Result<Vec<Pago>> {
    let mut stmt = c.prepare_cached(
        "SELECT id_pago, monto, fecha_pago
         FROM pagos
         WHERE id_estudiante = ?1 AND fecha_pago BETWEEN ?2 AND ?3
         ORDER BY fecha_pago DESC",
    )?;

    stmt.query_map(params![id_estudiante, inicio, fin], |r| {
        Ok(Pago {
            id: r.get(0)?,
            monto: r.get(1)?,
            fecha: r.get(2)?,
        })
    })?
    .collect()
}

/// ObtenerDeudaAnterior, payments part.
pub fn total_antes_de(
    c: &Connection,
    id_estudiante: i64,
    fecha: NaiveDate,
) -> rusqlite::Result<f64> {
    c.query_row(
        "SELECT COALESCE(SUM(monto), 0) FROM pagos
         WHERE id_estudiante = ?1 AND fecha_pago < ?2",
        params![id_estudiante, fecha],
        |r| r.get(0),
    )
}

/// RegistrarPago.
pub fn registrar(
    c: &Connection,
    id_estudiante: i64,
    monto: f64,
    fecha: NaiveDate,
) -> rusqlite::Result<i64> {
    c.execute(
        "INSERT INTO pagos (id_estudiante, monto, fecha_pago) VALUES (?1, ?2, ?3)",
        params![id_estudiante, monto, fecha],
    )?;
    Ok(c.last_insert_rowid())
}

/// EliminarPago, scoped to its student so a URL cannot delete another student's payment.
pub fn eliminar(c: &Connection, id_pago: i64, id_estudiante: i64) -> rusqlite::Result<usize> {
    c.execute(
        "DELETE FROM pagos WHERE id_pago = ?1 AND id_estudiante = ?2",
        params![id_pago, id_estudiante],
    )
}
