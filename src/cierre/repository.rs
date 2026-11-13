use chrono::NaiveDate;
use tokio_rusqlite::rusqlite::{self, Connection};

use crate::estudiantes::Sector;

#[derive(Debug, Default, PartialEq)]
pub struct Totales {
    pub vendido: f64,
    pub unidades: i64,
    pub atendidos: i64,
    pub sector_menor: f64,
    pub sector_mayor: f64,
}

pub struct VentaProducto {
    pub nombre: String,
    pub unidades: i64,
    pub total: f64,
}

pub struct PagoDelDia {
    pub id_estudiante: i64,
    pub estudiante: String,
    pub monto: f64,
}

/// Sales totals of one day, split by sector.
pub fn totales(c: &Connection, fecha: NaiveDate) -> rusqlite::Result<Totales> {
    let sql = format!(
        "SELECT COALESCE(SUM(c.total_linea), 0),
                COALESCE(SUM(c.cantidad), 0),
                COUNT(DISTINCT c.id_estudiante),
                COALESCE(SUM(CASE WHEN {} THEN c.total_linea END), 0),
                COALESCE(SUM(CASE WHEN {} THEN c.total_linea END), 0)
         FROM consumos c
         LEFT JOIN estudiantes e ON e.id_estudiante = c.id_estudiante
         WHERE c.fecha_consumo = ?1",
        Sector::Menor.condicion_sql(),
        Sector::Mayor.condicion_sql()
    );
    c.query_row(&sql, [fecha], |r| {
        Ok(Totales {
            vendido: r.get(0)?,
            unidades: r.get(1)?,
            atendidos: r.get(2)?,
            sector_menor: r.get(3)?,
            sector_mayor: r.get(4)?,
        })
    })
}

/// Units and amount sold per product on one day, best sellers first.
pub fn por_producto(c: &Connection, fecha: NaiveDate) -> rusqlite::Result<Vec<VentaProducto>> {
    let mut stmt = c.prepare_cached(
        "SELECT p.nombre, SUM(c.cantidad), SUM(c.total_linea)
         FROM consumos c
         JOIN productos p ON p.id_producto = c.id_producto
         WHERE c.fecha_consumo = ?1
         GROUP BY p.id_producto
         ORDER BY SUM(c.total_linea) DESC, p.nombre",
    )?;
    stmt.query_map([fecha], |r| {
        Ok(VentaProducto {
            nombre: r.get(0)?,
            unidades: r.get(1)?,
            total: r.get(2)?,
        })
    })?
    .collect()
}

/// Payments registered with this date.
pub fn pagos(c: &Connection, fecha: NaiveDate) -> rusqlite::Result<Vec<PagoDelDia>> {
    let mut stmt = c.prepare_cached(
        "SELECT p.id_estudiante, e.apellidos || ', ' || e.nombres, p.monto
         FROM pagos p
         JOIN estudiantes e ON e.id_estudiante = p.id_estudiante
         WHERE p.fecha_pago = ?1
         ORDER BY e.apellidos, e.nombres",
    )?;
    stmt.query_map([fecha], |r| {
        Ok(PagoDelDia {
            id_estudiante: r.get(0)?,
            estudiante: r.get(1)?,
            monto: r.get(2)?,
        })
    })?
    .collect()
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;

    use super::{Totales, pagos, por_producto, totales};
    use crate::test_support::conexion;

    #[test]
    fn el_cierre_suma_ventas_por_sector_producto_y_pagos_del_dia() {
        let c = conexion();
        c.execute_batch(
            "INSERT INTO consumos (id_estudiante, id_producto, cantidad, precio_unitario_venta, fecha_consumo) VALUES
               (1, 1, 2, 1.5, '2026-10-27'),
               (2, 2, 1, 5.0, '2026-10-27'),
               (2, 1, 1, 1.5, '2026-10-27'),
               (1, 2, 9, 5.0, '2026-10-28');
             INSERT INTO pagos (id_estudiante, monto, fecha_pago) VALUES
               (1, 3.0, '2026-10-27'),
               (2, 9.0, '2026-10-26');",
        )
        .unwrap();
        let dia = NaiveDate::from_ymd_opt(2026, 10, 27).unwrap();

        assert_eq!(
            totales(&c, dia).unwrap(),
            Totales {
                vendido: 9.5,
                unidades: 4,
                atendidos: 2,
                sector_menor: 3.0,
                sector_mayor: 6.5,
            }
        );
        let productos: Vec<(String, i64, f64)> = por_producto(&c, dia)
            .unwrap()
            .into_iter()
            .map(|v| (v.nombre, v.unidades, v.total))
            .collect();
        assert_eq!(
            productos,
            vec![("Broaster".into(), 1, 5.0), ("Gelatina".into(), 3, 4.5)]
        );
        let cobrado: Vec<f64> = pagos(&c, dia).unwrap().iter().map(|p| p.monto).collect();
        assert_eq!(cobrado, vec![3.0]);
    }

    #[test]
    fn un_dia_sin_movimientos_da_ceros() {
        let c = conexion();
        let dia = NaiveDate::from_ymd_opt(2026, 1, 1).unwrap();
        assert_eq!(totales(&c, dia).unwrap(), Totales::default());
        assert!(por_producto(&c, dia).unwrap().is_empty());
    }
}
