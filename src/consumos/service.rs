use std::collections::HashMap;

use chrono::NaiveDate;
use tokio_rusqlite::rusqlite::{self, Connection, TransactionBehavior};

use super::repository;
use crate::{estudiantes, productos};

/// Saves what one student consumed on one day, in a single transaction.
///
/// `cantidades` holds (product id, quantity). Only products whose quantity
/// changed are written, so untouched products keep the price they were sold
/// at. Inactive or unknown products are skipped. Returns `None` when the
/// student does not exist or is inactive (nothing is written).
pub fn guardar_dia(
    c: &mut Connection,
    id_estudiante: i64,
    fecha: NaiveDate,
    cantidades: &[(i64, i64)],
) -> rusqlite::Result<Option<usize>> {
    // IMMEDIATE takes the write lock up front, so another process writing the
    // same file cannot invalidate this read-then-write sequence midway.
    let tx = c.transaction_with_behavior(TransactionBehavior::Immediate)?;

    match estudiantes::repository::por_id(&tx, id_estudiante)? {
        Some(estudiante) if estudiante.esta_activo => {}
        _ => return Ok(None),
    }
    let precios: HashMap<i64, f64> = productos::repository::activos(&tx)?
        .into_iter()
        .map(|p| (p.id, p.precio_unitario))
        .collect();

    let mut cambios = 0;
    for &(id_producto, cantidad) in cantidades {
        let Some(&precio) = precios.get(&id_producto) else {
            continue;
        };
        if actualizar_celda(&tx, id_estudiante, id_producto, fecha, cantidad, precio)? {
            cambios += 1;
        }
    }

    tx.commit()?;
    Ok(Some(cambios))
}

/// `ActualizarConsumo`, now transactional. Returns whether the cell changed.
///
/// If legacy duplicate rows exist for the cell, they are merged into the first
/// row when the cell is edited.
fn actualizar_celda(
    tx: &Connection,
    id_estudiante: i64,
    id_producto: i64,
    fecha: NaiveDate,
    cantidad: i64,
    precio: f64,
) -> rusqlite::Result<bool> {
    let filas = repository::filas_de_celda(tx, id_estudiante, id_producto, fecha)?;
    let actual: i64 = filas.iter().map(|f| f.cantidad).sum();
    if actual == cantidad {
        return Ok(false);
    }

    match filas.split_first() {
        None => repository::registrar(tx, id_estudiante, id_producto, cantidad, precio, fecha)?,
        Some((primera, duplicadas)) => {
            for fila in duplicadas {
                repository::eliminar(tx, fila.id)?;
            }
            if cantidad <= 0 {
                repository::eliminar(tx, primera.id)?;
            } else {
                repository::actualizar(tx, primera.id, cantidad, precio)?;
            }
        }
    }
    Ok(true)
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;
    use tokio_rusqlite::rusqlite::{Connection, params};

    use super::guardar_dia;
    use crate::test_support::{conexion, conexion_heredada};

    fn dia() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 10, 27).unwrap()
    }

    /// (cantidad, precio_unitario_venta, total_linea) of a cell, in insertion order.
    fn celda(c: &Connection, estudiante: i64, producto: i64) -> Vec<(i64, f64, f64)> {
        let mut stmt = c
            .prepare(
                "SELECT cantidad, precio_unitario_venta, total_linea FROM consumos
                 WHERE id_estudiante = ?1 AND id_producto = ?2 AND fecha_consumo = '2026-10-27'
                 ORDER BY id_consumo",
            )
            .unwrap();
        stmt.query_map([estudiante, producto], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?))
        })
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap()
    }

    fn vender(c: &Connection, estudiante: i64, producto: i64, cantidad: i64, precio: f64) {
        c.execute(
            "INSERT INTO consumos (id_estudiante, id_producto, cantidad, precio_unitario_venta, fecha_consumo)
             VALUES (?1, ?2, ?3, ?4, '2026-10-27')",
            params![estudiante, producto, cantidad, precio],
        )
        .unwrap();
    }

    #[test]
    fn registra_una_celda_nueva_con_el_precio_vigente() {
        let mut c = conexion();
        assert_eq!(guardar_dia(&mut c, 1, dia(), &[(1, 2)]).unwrap(), Some(1));
        assert_eq!(celda(&c, 1, 1), vec![(2, 1.5, 3.0)]);
    }

    #[test]
    fn una_celda_sin_cambios_conserva_su_precio_de_venta() {
        let mut c = conexion();
        vender(&c, 1, 1, 2, 1.0);
        assert_eq!(guardar_dia(&mut c, 1, dia(), &[(1, 2)]).unwrap(), Some(0));
        assert_eq!(celda(&c, 1, 1), vec![(2, 1.0, 2.0)]);
    }

    #[test]
    fn cambiar_la_cantidad_usa_el_precio_vigente() {
        let mut c = conexion();
        vender(&c, 1, 1, 2, 1.0);
        assert_eq!(guardar_dia(&mut c, 1, dia(), &[(1, 3)]).unwrap(), Some(1));
        assert_eq!(celda(&c, 1, 1), vec![(3, 1.5, 4.5)]);
    }

    #[test]
    fn cantidad_cero_elimina_la_celda() {
        let mut c = conexion();
        vender(&c, 1, 1, 2, 1.5);
        assert_eq!(guardar_dia(&mut c, 1, dia(), &[(1, 0)]).unwrap(), Some(1));
        assert!(celda(&c, 1, 1).is_empty());
    }

    #[test]
    fn duplicados_heredados_se_respetan_si_no_cambian_y_se_fusionan_al_editar() {
        // A database copied from the VPS before migration 001 may still have duplicates.
        let mut c = conexion_heredada();
        vender(&c, 1, 1, 1, 1.0);
        vender(&c, 1, 1, 1, 1.0);
        // The editor shows the sum (2): saving it untouched writes nothing.
        assert_eq!(guardar_dia(&mut c, 1, dia(), &[(1, 2)]).unwrap(), Some(0));
        assert_eq!(celda(&c, 1, 1).len(), 2);
        // Editing merges both rows into one.
        assert_eq!(guardar_dia(&mut c, 1, dia(), &[(1, 5)]).unwrap(), Some(1));
        assert_eq!(celda(&c, 1, 1), vec![(5, 1.5, 7.5)]);
    }

    #[test]
    fn con_el_indice_unique_guardar_varias_veces_mantiene_una_fila_por_celda() {
        let mut c = conexion();
        for cantidad in [1, 3, 2] {
            guardar_dia(&mut c, 1, dia(), &[(1, cantidad)]).unwrap();
        }
        assert_eq!(celda(&c, 1, 1), vec![(2, 1.5, 3.0)]);
    }

    #[test]
    fn la_base_rechaza_un_duplicado_insertado_por_fuera_de_la_app() {
        let c = conexion();
        vender(&c, 1, 1, 1, 1.5);
        let duplicado = c.execute(
            "INSERT INTO consumos (id_estudiante, id_producto, cantidad, precio_unitario_venta, fecha_consumo)
             VALUES (1, 1, 1, 1.5, '2026-10-27')",
            [],
        );
        assert!(duplicado.is_err());
    }

    #[test]
    fn un_estudiante_inactivo_o_inexistente_no_escribe_nada() {
        let mut c = conexion();
        assert_eq!(guardar_dia(&mut c, 3, dia(), &[(1, 2)]).unwrap(), None);
        assert_eq!(guardar_dia(&mut c, 99, dia(), &[(1, 2)]).unwrap(), None);
        let filas: i64 = c
            .query_row("SELECT COUNT(*) FROM consumos", [], |r| r.get(0))
            .unwrap();
        assert_eq!(filas, 0);
    }

    #[test]
    fn productos_inactivos_o_desconocidos_se_ignoran() {
        let mut c = conexion();
        let cambios = guardar_dia(&mut c, 1, dia(), &[(3, 2), (99, 1), (2, 1)]).unwrap();
        assert_eq!(cambios, Some(1));
        assert!(celda(&c, 1, 3).is_empty());
        assert_eq!(celda(&c, 1, 2), vec![(1, 5.0, 5.0)]);
    }
}
