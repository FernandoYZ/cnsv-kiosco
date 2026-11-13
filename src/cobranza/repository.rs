use chrono::NaiveDate;
use tokio_rusqlite::rusqlite::{self, Connection};

use crate::estudiantes::{
    Estudiante,
    repository::{COLUMNAS, DESDE, mapear},
};

pub struct Saldo {
    pub estudiante: Estudiante,
    /// All consumption minus all payments.
    pub saldo: f64,
    pub ultimo_pago: Option<NaiveDate>,
}

/// Current balance of every active student (`id_grado = 0` → all grades),
/// highest debt first.
pub fn saldos(c: &Connection, id_grado: i64) -> rusqlite::Result<Vec<Saldo>> {
    let sql = format!(
        "SELECT {COLUMNAS},
                COALESCE((SELECT SUM(c.total_linea) FROM consumos c
                          WHERE c.id_estudiante = e.id_estudiante), 0)
              - COALESCE((SELECT SUM(p.monto) FROM pagos p
                          WHERE p.id_estudiante = e.id_estudiante), 0) AS saldo,
                (SELECT MAX(p.fecha_pago) FROM pagos p
                 WHERE p.id_estudiante = e.id_estudiante) AS ultimo_pago
         {DESDE}
         WHERE e.esta_activo = 1 AND (?1 = 0 OR e.id_grado = ?1)
         ORDER BY saldo DESC, e.apellidos, e.nombres"
    );
    let mut stmt = c.prepare_cached(&sql)?;
    stmt.query_map([id_grado], |r| {
        Ok(Saldo {
            estudiante: mapear(r)?,
            saldo: r.get(6)?,
            ultimo_pago: r.get(7)?,
        })
    })?
    .collect()
}

#[cfg(test)]
mod tests {
    use super::saldos;
    use crate::test_support::conexion;

    #[test]
    fn el_saldo_es_todo_lo_consumido_menos_todo_lo_pagado_y_ordena_por_deuda() {
        let c = conexion();
        c.execute_batch(
            "INSERT INTO consumos (id_estudiante, id_producto, cantidad, precio_unitario_venta, fecha_consumo) VALUES
               (1, 2, 2, 5.0, '2026-10-20'),
               (2, 1, 1, 1.5, '2026-10-21');
             INSERT INTO pagos (id_estudiante, monto, fecha_pago) VALUES
               (1, 4.0, '2026-10-19'),
               (1, 1.0, '2026-10-26');",
        )
        .unwrap();

        let todos = saldos(&c, 0).unwrap();
        let resumen: Vec<(i64, f64)> = todos.iter().map(|s| (s.estudiante.id, s.saldo)).collect();
        // Ana 10.00 - 5.00 = 5.00, Luis 1.50; Rosa is inactive and excluded.
        assert_eq!(resumen, vec![(1, 5.0), (2, 1.5)]);
        assert_eq!(
            todos[0].ultimo_pago.map(|f| f.to_string()),
            Some("2026-10-26".into())
        );
        assert_eq!(todos[1].ultimo_pago, None);

        let grado7 = saldos(&c, 7).unwrap();
        assert_eq!(grado7.len(), 1);
        assert_eq!(grado7[0].estudiante.id, 2);
    }
}
