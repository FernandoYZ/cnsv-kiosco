//! Grouped queries for the main weekly view. Each one covers every active
//! student of a grade in a single query to avoid N+1 round trips.

use std::collections::HashMap;

use chrono::NaiveDate;
use tokio_rusqlite::rusqlite::{self, Connection, params};

pub struct LineaSemana {
    pub id_estudiante: i64,
    pub fecha: NaiveDate,
    pub nombre_producto: String,
    pub cantidad: i64,
    pub total_linea: f64,
}

/// ObtenerDeudasAnterioresBatch: consumption minus payments before `fecha`.
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
         WHERE e.esta_activo = 1 AND e.id_grado = ?2
         GROUP BY e.id_estudiante",
    )?;
    stmt.query_map(params![fecha, id_grado], |r| Ok((r.get(0)?, r.get(1)?)))?
        .collect()
}

/// ObtenerPagosSemanaBatch: payments between two dates (inclusive) per student.
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
         WHERE e.esta_activo = 1 AND e.id_grado = ?3
         GROUP BY e.id_estudiante",
    )?;
    stmt.query_map(params![inicio, fin, id_grado], |r| {
        Ok((r.get(0)?, r.get(1)?))
    })?
    .collect()
}

/// ObtenerConsumosSemana restricted to one grade, with product names, so the
/// table can show what each student took every day.
pub fn lineas_semana(
    c: &Connection,
    id_grado: i64,
    inicio: NaiveDate,
    fin: NaiveDate,
) -> rusqlite::Result<Vec<LineaSemana>> {
    let mut stmt = c.prepare_cached(
        "SELECT c.id_estudiante, c.fecha_consumo, p.nombre, c.cantidad, c.total_linea
         FROM consumos c
         JOIN estudiantes e ON e.id_estudiante = c.id_estudiante
         JOIN productos p   ON p.id_producto = c.id_producto
         WHERE c.fecha_consumo BETWEEN ?1 AND ?2
           AND e.esta_activo = 1 AND e.id_grado = ?3
         ORDER BY c.id_estudiante, c.fecha_consumo, p.nombre",
    )?;
    stmt.query_map(params![inicio, fin, id_grado], |r| {
        Ok(LineaSemana {
            id_estudiante: r.get(0)?,
            fecha: r.get(1)?,
            nombre_producto: r.get(2)?,
            cantidad: r.get(3)?,
            total_linea: r.get(4)?,
        })
    })?
    .collect()
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;

    use super::{deudas_anteriores, pagos_entre};
    use crate::test_support::conexion;

    fn fecha(texto: &str) -> NaiveDate {
        NaiveDate::parse_from_str(texto, "%Y-%m-%d").unwrap()
    }

    #[test]
    fn la_deuda_anterior_resta_los_pagos_previos_a_la_semana() {
        let c = conexion();
        c.execute_batch(
            "INSERT INTO consumos (id_estudiante, id_producto, cantidad, precio_unitario_venta, fecha_consumo) VALUES
               (1, 1, 2, 1.5, '2026-10-20'),
               (1, 2, 1, 5.0, '2026-10-27');
             INSERT INTO pagos (id_estudiante, monto, fecha_pago) VALUES
               (1, 1.0, '2026-10-19'),
               (1, 2.0, '2026-10-26'),
               (2, 4.0, '2026-10-19');",
        )
        .unwrap();

        // Ana: 3.00 consumed and 1.00 paid before the week; the rest is this week.
        let grado3 = deudas_anteriores(&c, 3, fecha("2026-10-26")).unwrap();
        assert_eq!(grado3.get(&1), Some(&2.0));
        assert!(!grado3.contains_key(&3), "inactive students are excluded");

        // Luis paid without consuming: negative debt (credit).
        let grado7 = deudas_anteriores(&c, 7, fecha("2026-10-26")).unwrap();
        assert_eq!(grado7.get(&2), Some(&-4.0));

        let pagos = pagos_entre(&c, 3, fecha("2026-10-26"), fecha("2026-11-01")).unwrap();
        assert_eq!(pagos.get(&1), Some(&2.0));
    }
}
