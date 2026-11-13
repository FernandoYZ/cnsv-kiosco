use std::collections::HashMap;

use chrono::NaiveDate;
use tokio_rusqlite::rusqlite::{self, Connection};

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
    let tx = c.transaction()?;

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
