//! Grouped queries for the main view. Each one returns a value per student in
//! a single query to avoid N+1 round trips.

use std::collections::HashMap;

use chrono::NaiveDate;
use tokio_rusqlite::rusqlite::{self, Connection, params};

/// ObtenerDeudasAnterioresBatch: consumption minus payments before `fecha`
/// for every active student (`id_grado = 0` → all grades).
pub fn deudas_anteriores(
    c: &Connection,
    id_grado: i64,
    fecha: NaiveDate,
) -> rusqlite::Result<HashMap<i64, f64>> {
    let mut stmt = c.prepare_cached(
        "SELECT
           e.id_estudiante,
           COALESCE(SUM(c.total_linea), 0) - COALESCE((
               SELECT SUM(p.monto) FROM pagos p
               WHERE p.id_estudiante = e.id_estudiante AND p.fecha_pago < ?1
           ), 0) AS deuda_anterior
         FROM estudiantes e
         LEFT JOIN consumos c ON e.id_estudiante = c.id_estudiante AND c.fecha_consumo < ?1
         WHERE e.esta_activo = 1 AND (?2 = 0 OR e.id_grado = ?2)
         GROUP BY e.id_estudiante",
    )?;

    stmt.query_map(params![fecha, id_grado], |r| Ok((r.get(0)?, r.get(1)?)))?
        .collect()
}

/// ObtenerPagosSemanaBatch: payments between two dates (inclusive) per active student.
pub fn pagos_entre(
    c: &Connection,
    id_grado: i64,
    inicio: NaiveDate,
    fin: NaiveDate,
) -> rusqlite::Result<HashMap<i64, f64>> {
    let mut stmt = c.prepare_cached(
        "SELECT e.id_estudiante,
                COALESCE(SUM(p.monto), 0) AS total_pagos
         FROM estudiantes e
         LEFT JOIN pagos p ON e.id_estudiante = p.id_estudiante
                          AND p.fecha_pago BETWEEN ?1 AND ?2
         WHERE e.esta_activo = 1 AND (?3 = 0 OR e.id_grado = ?3)
         GROUP BY e.id_estudiante",
    )?;

    stmt.query_map(params![inicio, fin, id_grado], |r| {
        Ok((r.get(0)?, r.get(1)?))
    })?
    .collect()
}

/// ObtenerConsumosSemana, aggregated in SQL: amount per (student, day).
pub fn consumo_por_dia(
    c: &Connection,
    inicio: NaiveDate,
    fin: NaiveDate,
) -> rusqlite::Result<HashMap<(i64, NaiveDate), f64>> {
    let mut stmt = c.prepare_cached(
        "SELECT id_estudiante, fecha_consumo, SUM(total_linea)
         FROM consumos
         WHERE fecha_consumo BETWEEN ?1 AND ?2
         GROUP BY id_estudiante, fecha_consumo",
    )?;

    stmt.query_map(params![inicio, fin], |r| {
        Ok(((r.get(0)?, r.get(1)?), r.get(2)?))
    })?
    .collect()
}
